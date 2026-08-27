//! HTTP-laag: axum-router, handlers en askama-templates.
//!
//! De UI wordt serverside gerenderd (askama + HTMX vanaf een lokaal meegeleverd
//! bestand); er is bewust geen node-toolchain en geen aparte frontend-build.
//! Handlers roepen nooit rechtstreeks tag- of bestands-API's aan, maar gaan via
//! [`crate::tags`] en [`crate::fs`].

use std::sync::Arc;

use askama::Template;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::browse::{self, Listing};
use crate::config::Config;
use crate::fs::{Library, PathError};

/// Gedeelde toestand van de webserver.
///
/// Wordt in een `Arc` doorgegeven zodat handlers hem kunnen lezen zonder te
/// kopiëren. Alleen wat de handlers werkelijk nodig hebben staat erin; velden
/// als `max_art_size` komen erbij zodra de taken die ze gebruiken er zijn.
#[derive(Debug, Clone)]
pub struct AppState {
    /// De enige route van gebruikerspad naar filesystem-pad; handlers lossen
    /// nooit zelf een pad op.
    pub library: Arc<Library>,
}

/// Bouwt de volledige router.
///
/// Los van het opstarten van de server, zodat tests hem zonder netwerk kunnen
/// aanroepen.
pub fn router(config: Config, static_dir: &std::path::Path) -> Router {
    let state = AppState {
        library: Arc::new(Library::new(config.music_root)),
    };

    Router::new()
        .route("/", get(browse_root))
        // De bibliotheek is een boom van onbekende diepte, vandaar een
        // wildcard. Het pad gaat ongewijzigd naar `fs::Library`, die als enige
        // beoordeelt of het binnen `MUSIC_ROOT` valt.
        .route("/map/{*path}", get(browse_directory))
        .route("/healthz", get(healthz))
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// De volledige mappagina.
#[derive(Template)]
#[template(path = "directory.html")]
struct DirectoryTemplate {
    listing: Listing,
}

/// Alleen de lijst met submappen en tracks.
///
/// HTMX vervangt hiermee de lijst zonder de pagina opnieuw op te bouwen; zonder
/// JavaScript wordt dezelfde URL als gewone pagina opgevraagd.
#[derive(Template)]
#[template(path = "listing.html")]
struct ListingTemplate {
    listing: Listing,
}

/// De zoekterm uit de querystring (FR-3).
#[derive(Debug, Default, serde::Deserialize)]
struct BrowseQuery {
    #[serde(default)]
    q: String,
}

/// De bibliotheekwortel: het startpunt van elke bewerksessie.
async fn browse_root(
    State(state): State<AppState>,
    Query(query): Query<BrowseQuery>,
    headers: HeaderMap,
) -> Result<Html<String>, WebError> {
    render_listing(state, String::new(), query.q, &headers).await
}

/// Een map onder de wortel.
async fn browse_directory(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<BrowseQuery>,
    headers: HeaderMap,
) -> Result<Html<String>, WebError> {
    render_listing(state, path, query.q, &headers).await
}

async fn render_listing(
    state: AppState,
    path: String,
    query: String,
    headers: &HeaderMap,
) -> Result<Html<String>, WebError> {
    let library = Arc::clone(&state.library);

    // Het lezen van de tags van een hele map is blokkerende bestands-I/O. Op de
    // async-runtime zou dat de worker vasthouden waarop ook andere verzoeken
    // moeten draaien.
    let listing =
        tokio::task::spawn_blocking(move || browse::listing(&library, &path, &query)).await??;

    // HTMX vraagt alleen het stuk op dat het vervangt. Elk ander verzoek — een
    // gedeelde link, een herlaadactie, een browser zonder JavaScript — krijgt de
    // hele pagina.
    let html = if headers.contains_key("hx-request") {
        ListingTemplate { listing }.render()?
    } else {
        DirectoryTemplate { listing }.render()?
    };

    Ok(Html(html))
}

/// Healthcheck voor Docker.
///
/// Bewust zonder template of state: als dit endpoint iets nodig heeft dat stuk
/// kan, meet het niet meer wat het moet meten.
async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// Fout die naar een HTTP-respons vertaald kan worden.
#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("template kon niet gerenderd worden: {0}")]
    Render(#[from] askama::Error),

    #[error(transparent)]
    Pad(#[from] crate::fs::PathError),

    #[error("de achtergrondtaak is afgebroken: {0}")]
    Background(#[from] tokio::task::JoinError),
}

impl IntoResponse for WebError {
    fn into_response(self) -> axum::response::Response {
        match self {
            WebError::Background(error) => {
                tracing::error!(%error, "map kon niet gelezen worden");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Er ging iets mis bij het lezen van de map.",
                )
                    .into_response()
            }

            WebError::Render(error) => {
                // De technische oorzaak hoort in het log, niet in de browser.
                tracing::error!(%error, "pagina kon niet worden opgebouwd");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Er ging iets mis bij het opbouwen van de pagina.",
                )
                    .into_response()
            }

            WebError::Pad(error) => {
                let status = match error {
                    // Een pad buiten de bibliotheek is een geweigerd verzoek, geen
                    // vergissing in de URL: 403 in plaats van 404, zodat het in de
                    // logs te onderscheiden is van een dode link.
                    PathError::OutsideLibrary => StatusCode::FORBIDDEN,
                    PathError::NotFound => StatusCode::NOT_FOUND,
                    PathError::Unsupported => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                };

                tracing::warn!(%error, %status, "verzoek geweigerd");

                // De melding van PadFout bevat bewust geen pad.
                (status, error.to_string()).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::http::Request;
    use clap::Parser;
    use tower::ServiceExt;

    /// Bouwt een router met een wegwerp-`MUSIC_ROOT`.
    ///
    /// Tests raken nooit de echte bibliotheek; de root is een lege tempdir.
    fn test_router(root: &tempfile::TempDir) -> Router {
        let config = Config::try_parse_from([
            "sleeve-tag",
            "--music-root",
            root.path().to_str().expect("tempdir-pad moet UTF-8 zijn"),
        ])
        .expect("testconfiguratie moet geldig zijn");

        router(config, std::path::Path::new("static"))
    }

    async fn body_as_string(respons: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(respons.into_body(), 1024 * 1024)
            .await
            .expect("body moet leesbaar zijn");
        String::from_utf8(bytes.to_vec()).expect("body moet UTF-8 zijn")
    }

    #[tokio::test]
    async fn healthz_returns_200_with_a_short_body() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = test_router(&root)
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        assert_eq!(respons.status(), StatusCode::OK);
        assert_eq!(body_as_string(respons).await, "ok");
    }

    #[tokio::test]
    async fn index_renders_with_the_name_sleeve() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = test_router(&root)
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        assert_eq!(respons.status(), StatusCode::OK);

        let html = body_as_string(respons).await;
        assert!(html.contains("Sleeve"), "pagina was: {html}");
        assert!(html.contains("<!DOCTYPE html>"), "pagina was: {html}");
        assert!(
            html.contains("width=device-width"),
            "viewport-regel ontbreekt, de pagina is dan onbruikbaar op een telefoon: {html}"
        );
    }

    #[tokio::test]
    async fn index_loads_only_local_assets() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = test_router(&root)
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        let html = body_as_string(respons).await;

        // Op de NAS is er geen internet; een externe stylesheet of script zou
        // daar pas opvallen als de pagina half leeg blijft.
        assert!(
            !html.contains("https://") && !html.contains("http://"),
            "de pagina verwijst naar een externe host: {html}"
        );
        assert!(
            html.contains("/static/htmx.min.js"),
            "htmx wordt niet lokaal geladen: {html}"
        );
    }

    #[tokio::test]
    async fn static_files_are_served() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = test_router(&root)
            .oneshot(
                Request::builder()
                    .uri("/static/htmx.min.js")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        assert_eq!(respons.status(), StatusCode::OK);
        assert!(body_as_string(respons).await.contains("htmx"));
    }

    #[tokio::test]
    async fn unknown_path_returns_404() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = test_router(&root)
            .oneshot(
                Request::builder()
                    .uri("/bestaat-niet")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        assert_eq!(respons.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn static_path_cannot_escape_the_directory() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = test_router(&root)
            .oneshot(
                Request::builder()
                    .uri("/static/../Cargo.toml")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        assert_ne!(
            respons.status(),
            StatusCode::OK,
            "een pad met .. mag nooit een bestand buiten static/ opleveren"
        );
    }
}
