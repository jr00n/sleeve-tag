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
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::browse::{self, Listing, THUMBNAIL_SIZE_PARAM};
use crate::config::Config;
use crate::fs::{Library, PathError};
use crate::{art, tags};

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
        .route("/art/{*path}", get(art_of_file))
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

/// Welke variant van de hoes gevraagd wordt.
#[derive(Debug, Default, serde::Deserialize)]
struct ArtQuery {
    #[serde(default)]
    size: String,
}

/// Maximale afmeting van een thumbnail, per as.
///
/// Het vakje in de maplijst is veertig pixels; op een scherm met een hoge
/// pixeldichtheid is dat honderdtwintig echte pixels. Honderdzestig geeft daar
/// marge op zonder dat de bytes noemenswaardig oplopen.
const THUMBNAIL_MAX_PIXELS: u32 = 160;

/// De embedded front cover van één bestand.
///
/// Zonder `?size=thumb` komt de hoes ongewijzigd terug, met het MIME-type zoals
/// het in het bestand staat; dat is wat de detailweergave straks nodig heeft.
/// Met `?size=thumb` komt er een verkleinde JPEG terug: de maplijst toont
/// dertig van deze plaatjes naast elkaar in een vakje van veertig pixels, en
/// daar dertig volledige hoezen voor over het netwerk sturen zou de pagina
/// onbruikbaar maken op een telefoon.
async fn art_of_file(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<ArtQuery>,
) -> Result<Response, WebError> {
    let library = Arc::clone(&state.library);
    let thumbnail = query.size == THUMBNAIL_SIZE_PARAM;

    // Uitlezen en verkleinen zijn allebei blokkerend: het eerste wacht op de
    // schijf, het tweede op de processor.
    let (mime, bytes) =
        tokio::task::spawn_blocking(move || read_cover(&library, &path, thumbnail)).await??;

    Ok((
        [
            (header::CONTENT_TYPE, mime),
            // Er is bewust geen cache-laag in het MVP, en na een latere
            // schrijfactie mag de browser geen oude hoes blijven tonen.
            (header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        bytes,
    )
        .into_response())
}

/// Haalt de hoes uit een bestand, eventueel verkleind.
///
/// Het pad gaat via [`Library::resolve`] en niet via `resolve_editable_file`:
/// dat laatste opent het bestand een extra keer om het formaat vast te stellen,
/// en bij dertig thumbnails per pagina telt dat op. Dat het geen audio is,
/// blijkt hier vanzelf uit het lezen.
fn read_cover(
    library: &Library,
    path: &str,
    thumbnail: bool,
) -> Result<(String, Vec<u8>), WebError> {
    let absolute = library.resolve(path)?;

    let Some((mime, data)) = tags::read_front_cover(&absolute)? else {
        return Err(WebError::NoArt);
    };

    if !thumbnail {
        return Ok((mime, data));
    }

    match art::thumbnail(&data, THUMBNAIL_MAX_PIXELS) {
        Ok(small) => Ok(("image/jpeg".to_string(), small)),

        // Een hoes die niet te verkleinen is, is nog steeds een hoes. Het
        // origineel doorgeven kost bandbreedte, maar laat de gebruiker zien wat
        // er in het bestand zit in plaats van een gebroken plaatje.
        Err(error) => {
            tracing::warn!(
                path = %absolute.display(),
                %error,
                "hoes kon niet verkleind worden; het origineel wordt geserveerd"
            );
            Ok((mime, data))
        }
    }
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

    #[error(transparent)]
    Tag(#[from] crate::tags::TagError),

    #[error("dit bestand bevat geen album art")]
    NoArt,
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

            WebError::Tag(error) => {
                let status = match error {
                    // Onleesbaar betekent hier in de praktijk: er staat niet
                    // wat de gebruiker dacht dat er stond.
                    tags::TagError::Unreadable => StatusCode::NOT_FOUND,
                    tags::TagError::UnsupportedFormat => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                };

                tracing::warn!(%error, %status, "bestand kon niet gelezen worden");
                (status, error.to_string()).into_response()
            }

            // Geen fout in de aanvraag, maar er is niets te tonen.
            WebError::NoArt => (StatusCode::NOT_FOUND, WebError::NoArt.to_string()).into_response(),

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

    /// Bouwt een bibliotheek met één album met en zonder hoes erin.
    fn root_with_art() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("tempdir");
        let album = root.path().join("Album");
        std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

        crate::testfixtures::copy_to(&album, crate::testfixtures::MP3_WITH_ART);
        crate::testfixtures::copy_to(&album, crate::testfixtures::MP3_WITH_TAGS);
        std::fs::write(album.join("notities.txt"), b"tekst")
            .expect("bestand moet te schrijven zijn");

        root
    }

    /// Doet één GET-verzoek en geeft de respons terug.
    async fn get(root: &tempfile::TempDir, uri: &str) -> axum::response::Response {
        test_router(root)
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("verzoek"),
            )
            .await
            .expect("respons")
    }

    fn content_type(respons: &axum::response::Response) -> String {
        respons
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string()
    }

    async fn body_bytes(respons: axum::response::Response) -> Vec<u8> {
        axum::body::to_bytes(respons.into_body(), 8 * 1024 * 1024)
            .await
            .expect("body moet leesbaar zijn")
            .to_vec()
    }

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

    #[tokio::test]
    async fn art_is_served_with_the_mime_type_from_the_file() {
        let root = root_with_art();
        let respons = get(&root, "/art/Album/tagged-with-art.mp3").await;

        assert_eq!(respons.status(), StatusCode::OK);
        assert_eq!(content_type(&respons), "image/jpeg");
        assert_eq!(
            respons
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-cache"),
            "zonder cache-laag mag de browser geen oude hoes vasthouden"
        );

        let bytes = body_bytes(respons).await;
        assert_eq!(
            crate::art::dimensions(&bytes).expect("het antwoord moet een afbeelding zijn"),
            (300, 300),
            "zonder ?size komt de hoes ongewijzigd terug"
        );
    }

    #[tokio::test]
    async fn a_thumbnail_is_scaled_down() {
        let root = root_with_art();
        let respons = get(&root, "/art/Album/tagged-with-art.mp3?size=thumb").await;

        assert_eq!(respons.status(), StatusCode::OK);
        assert_eq!(content_type(&respons), "image/jpeg");

        let thumb = body_bytes(respons).await;
        let (width, height) =
            crate::art::dimensions(&thumb).expect("de thumbnail moet een afbeelding zijn");

        // De afmetingen zijn hier de garantie. Dat het ook minder bytes
        // oplevert hangt van de hoes af en wordt met een realistische
        // afbeelding getest in `art::tests`; de fixture is een egaal vlak dat
        // al in ruim een kilobyte past.
        assert!(
            width <= THUMBNAIL_MAX_PIXELS && height <= THUMBNAIL_MAX_PIXELS,
            "thumbnail is {width}x{height}, verwacht hoogstens {THUMBNAIL_MAX_PIXELS}"
        );
        assert!(width < 300, "de hoes van 300 px is niet verkleind");
    }

    #[tokio::test]
    async fn a_file_without_art_gives_a_readable_404() {
        let root = root_with_art();
        let respons = get(&root, "/art/Album/tagged.mp3").await;

        assert_eq!(respons.status(), StatusCode::NOT_FOUND);
        assert!(
            body_as_string(respons).await.contains("geen album art"),
            "de melding hoort uit te leggen wat er aan de hand is"
        );
    }

    #[tokio::test]
    async fn art_of_something_that_is_not_audio_is_refused() {
        let root = root_with_art();

        assert_eq!(
            get(&root, "/art/Album/notities.txt").await.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(
            get(&root, "/art/Album/bestaat-niet.mp3").await.status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn art_cannot_escape_the_library() {
        let root = root_with_art();

        assert_eq!(
            get(&root, "/art/../../etc/passwd").await.status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn the_listing_links_to_the_thumbnail_endpoint() {
        let root = root_with_art();
        let html = body_as_string(get(&root, "/map/Album").await).await;

        assert!(
            html.contains(r#"src="/art/Album/tagged-with-art.mp3?size=thumb""#),
            "de hoes wordt niet als thumbnail opgevraagd: {html}"
        );
        assert!(
            html.contains(r#"loading="lazy""#),
            "zonder lazy loading blokkeren de hoezen het renderen: {html}"
        );
        assert!(
            !html.contains(r#"src="/art/Album/tagged.mp3"#),
            "een bestand zonder hoes hoort geen verzoek uit te lokken: {html}"
        );
    }
}
