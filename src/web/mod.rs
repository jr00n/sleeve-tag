//! HTTP-laag: axum-router, handlers en askama-templates.
//!
//! De UI wordt serverside gerenderd (askama + HTMX vanaf een lokaal meegeleverd
//! bestand); er is bewust geen node-toolchain en geen aparte frontend-build.
//! Handlers roepen nooit rechtstreeks tag- of bestands-API's aan, maar gaan via
//! [`crate::tags`] en [`crate::fs`].

use std::sync::Arc;

use askama::Template;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::fs::{Bibliotheek, PadFout};

/// Gedeelde toestand van de webserver.
///
/// Wordt in een `Arc` doorgegeven zodat handlers hem kunnen lezen zonder te
/// kopiëren. Alleen wat de handlers werkelijk nodig hebben staat erin; velden
/// als `max_art_size` komen erbij zodra de taken die ze gebruiken er zijn.
#[derive(Debug, Clone)]
pub struct AppState {
    /// De enige route van gebruikerspad naar filesystem-pad; handlers lossen
    /// nooit zelf een pad op.
    pub bibliotheek: Arc<Bibliotheek>,
}

/// Bouwt de volledige router.
///
/// Los van het opstarten van de server, zodat tests hem zonder netwerk kunnen
/// aanroepen.
pub fn router(config: Config, static_dir: &std::path::Path) -> Router {
    let state = AppState {
        bibliotheek: Arc::new(Bibliotheek::nieuw(config.music_root)),
    };

    Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Startpagina.
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    music_root: String,
}

async fn index(State(state): State<AppState>) -> Result<Html<String>, WebError> {
    let pagina = IndexTemplate {
        music_root: state.bibliotheek.root().display().to_string(),
    };

    Ok(Html(pagina.render()?))
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
    Pad(#[from] crate::fs::PadFout),
}

impl IntoResponse for WebError {
    fn into_response(self) -> axum::response::Response {
        match self {
            WebError::Render(fout) => {
                // De technische oorzaak hoort in het log, niet in de browser.
                tracing::error!(%fout, "pagina kon niet worden opgebouwd");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Er ging iets mis bij het opbouwen van de pagina.",
                )
                    .into_response()
            }

            WebError::Pad(fout) => {
                let status = match fout {
                    // Een pad buiten de bibliotheek is een geweigerd verzoek, geen
                    // vergissing in de URL: 403 in plaats van 404, zodat het in de
                    // logs te onderscheiden is van een dode link.
                    PadFout::BuitenBibliotheek => StatusCode::FORBIDDEN,
                    PadFout::NietGevonden => StatusCode::NOT_FOUND,
                    PadFout::NietOndersteund => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                };

                tracing::warn!(%fout, %status, "verzoek geweigerd");

                // De melding van PadFout bevat bewust geen pad.
                (status, fout.to_string()).into_response()
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
    fn testrouter(root: &tempfile::TempDir) -> Router {
        let config = Config::try_parse_from([
            "sleeve-tag",
            "--music-root",
            root.path().to_str().expect("tempdir-pad moet UTF-8 zijn"),
        ])
        .expect("testconfiguratie moet geldig zijn");

        router(config, std::path::Path::new("static"))
    }

    async fn body_als_string(respons: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(respons.into_body(), 1024 * 1024)
            .await
            .expect("body moet leesbaar zijn");
        String::from_utf8(bytes.to_vec()).expect("body moet UTF-8 zijn")
    }

    #[tokio::test]
    async fn healthz_geeft_200_met_korte_body() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = testrouter(&root)
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        assert_eq!(respons.status(), StatusCode::OK);
        assert_eq!(body_als_string(respons).await, "ok");
    }

    #[tokio::test]
    async fn startpagina_rendert_met_de_naam_sleeve() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = testrouter(&root)
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        assert_eq!(respons.status(), StatusCode::OK);

        let html = body_als_string(respons).await;
        assert!(html.contains("Sleeve"), "pagina was: {html}");
        assert!(html.contains("<!DOCTYPE html>"), "pagina was: {html}");
        assert!(
            html.contains("width=device-width"),
            "viewport-regel ontbreekt, de pagina is dan onbruikbaar op een telefoon: {html}"
        );
    }

    #[tokio::test]
    async fn startpagina_laadt_uitsluitend_lokale_assets() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = testrouter(&root)
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        let html = body_als_string(respons).await;

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
    async fn statische_bestanden_worden_geserveerd() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = testrouter(&root)
            .oneshot(
                Request::builder()
                    .uri("/static/htmx.min.js")
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons");

        assert_eq!(respons.status(), StatusCode::OK);
        assert!(body_als_string(respons).await.contains("htmx"));
    }

    #[tokio::test]
    async fn onbekend_pad_geeft_404() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = testrouter(&root)
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
    async fn statisch_pad_kan_niet_buiten_de_map_kijken() {
        let root = tempfile::tempdir().expect("tempdir");
        let respons = testrouter(&root)
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
