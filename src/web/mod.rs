//! HTTP-laag: axum-router, handlers en askama-templates.
//!
//! De UI wordt serverside gerenderd (askama + HTMX vanaf een lokaal meegeleverd
//! bestand); er is bewust geen node-toolchain en geen aparte frontend-build.
//! Handlers roepen nooit rechtstreeks tag- of bestands-API's aan, maar gaan via
//! [`crate::tags`] en [`crate::fs`].

use std::sync::Arc;

use askama::Template;
use axum::Router;
use axum::extract::{Form, FromRequest, Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::batch::AlbumPage;
use crate::browse::{self, Crumb, Listing, THUMBNAIL_SIZE_PARAM};
use crate::config::Config;
use crate::cover::{self, CoverDetails, CoverPage};
use crate::edit::{EditPage, Notice};
use crate::fs::{Library, PathError};
use crate::tags::RawTags;
use crate::{art, atomic, batch, edit, tags};

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

    /// Hoe er geschreven wordt; komt uit `BACKUP_ON_WRITE`.
    pub write_options: atomic::Options,

    /// Waar een aangeleverde hoes aan moet voldoen; komt uit `MAX_ART_SIZE`,
    /// `ART_QUALITY` en `MAX_UPLOAD_MB`.
    pub art_limits: art::Limits,
}

/// Bouwt de volledige router.
///
/// Los van het opstarten van de server, zodat tests hem zonder netwerk kunnen
/// aanroepen.
pub fn router(config: Config, static_dir: &std::path::Path) -> Router {
    // De uploadgrens geldt op twee plekken, en dat is met opzet: axum kapt de
    // body af vóór hij in het geheugen past, en `art::prepare` meldt wat er aan
    // de hand is zodra de bytes er wél zijn.
    let upload_limit = config.max_upload_mb as usize * 1024 * 1024;

    let state = AppState {
        library: Arc::new(Library::new(config.music_root)),
        write_options: atomic::Options {
            backup: config.backup_on_write,
        },
        art_limits: art::Limits {
            max_width: config.max_art_size.width,
            max_height: config.max_art_size.height,
            quality: config.art_quality,
            max_upload_mb: config.max_upload_mb,
        },
    };

    Router::new()
        .route("/", get(browse_root))
        // De bibliotheek is een boom van onbekende diepte, vandaar een
        // wildcard. Het pad gaat ongewijzigd naar `fs::Library`, die als enige
        // beoordeelt of het binnen `MUSIC_ROOT` valt.
        .route("/map/{*path}", get(browse_directory))
        .route("/art/{*path}", get(art_of_file))
        .route("/tags/{*path}", get(raw_tags_of_file))
        .route("/hoes/{*path}", get(cover_of_file).post(save_cover))
        .route("/bewerk/{*path}", get(edit_form).post(save_tags))
        // De albumweergave hoort bij een map, dus ook de wortel heeft er een.
        .route("/album", get(album_root).post(album_root_selection))
        .route("/album/{*path}", get(album_page).post(album_selection))
        .route("/healthz", get(healthz))
        // Axum staat standaard 2 MB toe; een hoes van vijf megabyte zou dan
        // afketsen op een kale 413 in plaats van op onze eigen melding.
        .layer(axum::extract::DefaultBodyLimit::max(upload_limit))
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

/// Wat de querystring over het versmallen van de lijst zegt.
///
/// `q` is de zoekterm (FR-3); `aandacht` zet het filter op wat een signalering
/// heeft (FR-4). Beide staan in de URL, zodat een gefilterde lijst te delen en
/// te bookmarken is en het ook zonder JavaScript werkt. Wat de waarden
/// betekenen, beslist [`browse::Filter`] en niet deze handler.
#[derive(Debug, Default, serde::Deserialize)]
struct BrowseQuery {
    #[serde(default)]
    q: String,

    #[serde(default)]
    aandacht: String,
}

impl BrowseQuery {
    fn filter(&self) -> browse::Filter {
        browse::Filter::from_query(&self.q, &self.aandacht)
    }
}

/// De bibliotheekwortel: het startpunt van elke bewerksessie.
async fn browse_root(
    State(state): State<AppState>,
    Query(query): Query<BrowseQuery>,
    headers: HeaderMap,
) -> Result<Html<String>, WebError> {
    render_listing(state, String::new(), query.filter(), &headers).await
}

/// Een map onder de wortel.
async fn browse_directory(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<BrowseQuery>,
    headers: HeaderMap,
) -> Result<Html<String>, WebError> {
    render_listing(state, path, query.filter(), &headers).await
}

async fn render_listing(
    state: AppState,
    path: String,
    filter: browse::Filter,
    headers: &HeaderMap,
) -> Result<Html<String>, WebError> {
    let library = Arc::clone(&state.library);

    // Het lezen van de tags van een hele map is blokkerende bestands-I/O. Op de
    // async-runtime zou dat de worker vasthouden waarop ook andere verzoeken
    // moeten draaien.
    let listing =
        tokio::task::spawn_blocking(move || browse::listing(&library, &path, &filter)).await??;

    // HTMX vraagt alleen het stuk op dat het vervangt. Elk ander verzoek — een
    // gedeelde link, een herlaadactie, een browser zonder JavaScript — krijgt de
    // hele pagina.
    let html = if is_htmx(headers) {
        ListingTemplate { listing }.render()?
    } else {
        DirectoryTemplate { listing }.render()?
    };

    Ok(Html(html))
}

/// De volledige albumpagina.
#[derive(Template)]
#[template(path = "album.html")]
struct AlbumTemplate {
    page: AlbumPage,
}

/// Alleen het formulier: de tabel, de gedeelde velden en wat er gebeurt.
///
/// HTMX vervangt hiermee het formulier zonder de pagina opnieuw op te bouwen;
/// zonder JavaScript komt dezelfde inhoud als hele pagina terug.
#[derive(Template)]
#[template(path = "albumform.html")]
struct AlbumFormTemplate {
    page: AlbumPage,
}

/// De voorbeeldweergave vóór het opslaan van een batch (FR-11).
#[derive(Template)]
#[template(path = "albumpreview.html")]
struct AlbumPreviewTemplate {
    preview: batch::Preview,
}

/// Alleen het voorbeeldformulier, voor HTMX.
#[derive(Template)]
#[template(path = "albumpreviewform.html")]
struct AlbumPreviewFormTemplate {
    preview: batch::Preview,
}

/// De albumweergave van de wortel.
async fn album_root(State(state): State<AppState>) -> Result<Html<String>, WebError> {
    render_album(state, String::new(), batch::Form::select_all(), None, false).await
}

/// De albumweergave van een map onder de wortel (FR-8).
///
/// Bij het openen is alles geselecteerd: een album corrigeren begint vrijwel
/// altijd bij het hele album.
async fn album_page(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Html<String>, WebError> {
    render_album(state, path, batch::Form::select_all(), None, false).await
}

async fn album_root_selection(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<Html<String>, WebError> {
    let form = batch::Form::parse(&body);
    render_album(state, String::new(), form, None, is_htmx(&headers)).await
}

/// Neemt een gewijzigde selectie, invoer of knop aan en toont het resultaat.
///
/// Vrijwel elke POST hierheen bouwt alleen de pagina opnieuw op: welke
/// bestanden de gedeelde velden nu beschrijven, en wat er dus zou veranderen.
/// Alleen `actie=opslaan` schrijft, en die knop staat uitsluitend op de
/// voorbeeldweergave — zo vindt een batch-wijziging nooit zonder voorbeeld
/// plaats.
async fn album_selection(
    State(state): State<AppState>,
    Path(path): Path<String>,
    request: axum::extract::Request,
) -> Result<Html<String>, WebError> {
    let htmx = is_htmx(request.headers());
    let (form, cover) = read_album_request(&state, request).await?;
    render_album(state, path, form, cover, htmx).await
}

/// Leest het albumformulier, in welke vorm het ook binnenkomt.
///
/// Twee vormen, en dat is met opzet. Elk vinkje in de albumweergave post het
/// hele formulier opnieuw; dat gaat urlencoded en blijft klein. Alleen de
/// laatste stap — de voorbeeldweergave — mag een hoes meedragen, en die is
/// daarom multipart. Zo reist een afbeelding van megabytes precies één keer,
/// op het moment dat er ook werkelijk iets mee gebeurt.
async fn read_album_request(
    state: &AppState,
    request: axum::extract::Request,
) -> Result<(batch::Form, Option<Vec<u8>>), WebError> {
    let is_multipart = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("multipart/form-data"));

    if !is_multipart {
        let bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
            .await
            .map_err(|_| WebError::Unreadable)?;
        let body = String::from_utf8_lossy(&bytes);

        return Ok((batch::Form::parse(&body), None));
    }

    // De afwijzing van deze extractor is geen `MultipartError` maar een
    // respons; hem als "onleesbaar" behandelen zegt hetzelfde en houdt de
    // foutafhandeling op één plek.
    let mut multipart = Multipart::from_request(request, state)
        .await
        .map_err(|_| WebError::Unreadable)?;

    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut cover: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(WebError::Upload)? {
        let name = field.name().unwrap_or_default().to_string();

        if name == "afbeelding" {
            let bytes = field.bytes().await.map_err(WebError::Upload)?;
            // Een leeg bestandsveld hoort niet als "een hoes" te tellen: dan is
            // er simpelweg niets gekozen.
            if !bytes.is_empty() {
                cover = Some(bytes.to_vec());
            }
            continue;
        }

        let value = field.text().await.map_err(WebError::Upload)?;
        pairs.push((name, value));
    }

    Ok((batch::Form::from_pairs(pairs), cover))
}

async fn render_album(
    state: AppState,
    path: String,
    form: batch::Form,
    cover: Option<Vec<u8>>,
    fragment: bool,
) -> Result<Html<String>, WebError> {
    let listing = read_listing(&state, &path).await?;

    match form.action {
        // De voorbeeldweergave: wat krijgt welk bestand (FR-11). Er wordt niets
        // geschreven; de knop om dat wél te doen staat pas op die pagina.
        batch::Action::Preview => {
            let preview = describe_preview(&state, &listing, &form);
            render_preview(preview, fragment)
        }

        batch::Action::Save => {
            let preview = describe_preview(&state, &listing, &form);

            // Een plan met een fout erin wordt niet half uitgevoerd: dan komt
            // het voorbeeld terug met wat eraan mankeert.
            if !preview.problems.is_empty() {
                return render_preview(preview, fragment);
            }

            // Een aangeleverde hoes wordt één keer klaargemaakt, niet per
            // bestand: verkleinen en hercoderen is werk dat voor elk bestand
            // hetzelfde uitpakt.
            let prepared = match cover {
                Some(bytes) => match art::prepare(&bytes, state.art_limits) {
                    Ok(prepared) => Some(prepared),
                    Err(error) => {
                        // De afbeelding deugt niet: dan gaat er niets door, ook
                        // de tags niet. Half uitvoeren van een plan dat niet
                        // klopt is erger dan niets doen.
                        let mut preview = preview;
                        preview
                            .problems
                            .push(format!("De hoes deugt niet: {error}"));
                        return render_preview(preview, fragment);
                    }
                },
                None => None,
            };

            let report = save_batch(&state, &listing, &form, prepared.as_ref()).await?;

            // De waarden komen na afloop uit een verse leesronde: wat er nu op
            // het scherm staat, staat werkelijk in de bestanden. De invoer is
            // verwerkt en gaat dus weg.
            let listing = read_listing(&state, &path).await?;
            let mut page = batch::album(&listing, &form.without_input());
            page.report = Some(report);

            render_page(page, fragment)
        }

        _ => render_page(batch::album(&listing, &form), fragment),
    }
}

/// Bouwt de voorbeeldweergave en vult aan wat `batch::` niet kan weten.
///
/// De uploadgrens komt uit de configuratie en de losse hoes uit de map; `batch::`
/// kent geen van beide, want die module opent geen bestanden en leest geen
/// omgeving.
fn describe_preview(
    state: &AppState,
    listing: &browse::Listing,
    form: &batch::Form,
) -> batch::Preview {
    let mut preview = batch::preview(listing, form);
    preview.max_upload_mb = state.art_limits.max_upload_mb;
    preview.folder_cover = existing_folder_cover_in(state, listing);
    preview
}

/// Wat er als losse hoes in deze map staat, met zijn omvang.
fn existing_folder_cover_in(state: &AppState, listing: &browse::Listing) -> Option<String> {
    let track = listing.tracks.first()?;
    let absolute = state.library.resolve(&track.path).ok()?;
    let cover = state.library.sibling(&absolute, cover::FOLDER_COVER).ok()?;
    let bytes = std::fs::metadata(&cover).ok()?.len();

    Some(format!("{} ({bytes} bytes)", cover::FOLDER_COVER))
}

/// Leest de map in met de tags van elk bestand.
async fn read_listing(state: &AppState, path: &str) -> Result<browse::Listing, WebError> {
    let library = Arc::clone(&state.library);
    let wanted = path.to_string();

    // Net als de maplijst: elk bestand in de map wordt geopend om zijn tags te
    // lezen, en dat hoort niet op de async-runtime.
    Ok(
        tokio::task::spawn_blocking(move || {
            browse::listing(&library, &wanted, &Default::default())
        })
        .await??,
    )
}

fn render_page(page: AlbumPage, fragment: bool) -> Result<Html<String>, WebError> {
    let html = if fragment {
        AlbumFormTemplate { page }.render()?
    } else {
        AlbumTemplate { page }.render()?
    };

    Ok(Html(html))
}

fn render_preview(preview: batch::Preview, fragment: bool) -> Result<Html<String>, WebError> {
    let html = if fragment {
        AlbumPreviewFormTemplate { preview }.render()?
    } else {
        AlbumPreviewTemplate { preview }.render()?
    };

    Ok(Html(html))
}

/// Schrijft de batch weg, bestand voor bestand (FR-11).
///
/// Elk bestand wordt opnieuw ingelezen vlak voor het geschreven wordt: tussen
/// het voorbeeld en deze klik kan er van alles gebeurd zijn, en het plan hoort
/// op de werkelijke inhoud te worden toegepast en niet op een oude leesronde.
///
/// Een fout bij één bestand stopt de rest niet. Dat is de regel uit FR-11, en
/// ze staat hier: de lus loopt door en verzamelt per bestand wat er gebeurd is.
async fn save_batch(
    state: &AppState,
    listing: &browse::Listing,
    form: &batch::Form,
    cover: Option<&art::Prepared>,
) -> Result<batch::SaveReport, WebError> {
    // Met een hoes erbij wordt élk aangevinkt bestand aangeraakt, ook een
    // waarvan de tags al kloppen; zonder hoes blijft het plan wat het was.
    let plan = if cover.is_some() {
        batch::intents_with_selection(listing, form)
    } else {
        batch::intents(listing, form)
    };
    let library = Arc::clone(&state.library);
    let options = state.write_options;
    let embed = cover.map(|prepared| (prepared.mime.clone(), prepared.data.clone()));

    let results: Vec<batch::SaveResult> = tokio::task::spawn_blocking(move || {
        plan.into_iter()
            .map(|file| batch::SaveResult {
                outcome: save_one(&library, options, &file, embed.as_ref()),
                name: file.name,
            })
            .collect()
    })
    .await?;

    let mut report = batch::SaveReport { results };

    // De losse hoes komt ná de bestanden en met een eigen regel: gaat dat mis,
    // dan blijft staan wat er wél geschreven is (FR-14).
    if let Some(prepared) = cover
        && form.folder_cover
    {
        let anchor = first_track_path(listing);
        if !anchor.is_empty() {
            let result =
                write_folder_cover(state, &anchor, prepared, form.overwrite_folder_cover).await?;
            report.results.push(result);
        }
    }

    Ok(report)
}

/// Het pad van een track in deze map, als anker voor de losse hoes.
///
/// `atomic::place` neemt eigenaar, groep en rechten over van een bestand dat er
/// al staat; daar is één track voor nodig.
fn first_track_path(listing: &browse::Listing) -> String {
    listing
        .tracks
        .first()
        .map(|track| track.path.clone())
        .unwrap_or_default()
}

/// Werkt één bestand uit het plan bij.
fn save_one(
    library: &crate::fs::Library,
    options: crate::atomic::Options,
    file: &batch::FileIntent,
    cover: Option<&(String, Vec<u8>)>,
) -> batch::Outcome {
    let outcome = || -> Result<batch::Outcome, String> {
        let absolute = library
            .resolve(&file.path)
            .map_err(|error| error.to_string())?;
        let current = tags::read(&absolute).map_err(|error| error.to_string())?;

        let wanted = file.wanted(&current.tags)?;
        let changes = batch::changes_between(&current.tags, &wanted);

        let mut labels: Vec<String> = Vec::new();

        if !changes.is_empty() {
            let written =
                tags::write(&absolute, &wanted, options).map_err(|error| error.to_string())?;

            // Wat er is opgeruimd hoort in het rapport, achter de velden die de
            // gebruiker zelf heeft gewijzigd.
            labels.extend(changes.into_iter().map(|change| change.label));
            labels.extend(written.removal_labels());
        }

        // De hoes gaat als tweede, in dezelfde ronde: zo staat er per bestand
        // één regel met alles wat het gekregen heeft. Zit dezelfde hoes er al
        // in, dan raakt `write_art` het bestand niet aan.
        if let Some((mime, data)) = cover {
            let written = tags::write_art(&absolute, Some((mime, data)), options)
                .map_err(|error| error.to_string())?;

            if written.changed {
                labels.push("Hoes".to_string());
                labels.extend(written.removal_labels());
            }
        }

        if labels.is_empty() {
            return Ok(batch::Outcome::Unchanged);
        }

        Ok(batch::Outcome::Saved(labels))
    };

    match outcome() {
        Ok(outcome) => outcome,
        Err(reason) => {
            tracing::error!(path = %file.path, reason, "bestand uit de batch is niet opgeslagen");
            batch::Outcome::Failed(reason)
        }
    }
}

/// Of dit verzoek van HTMX komt en dus met een fragment toe kan.
fn is_htmx(headers: &HeaderMap) -> bool {
    headers.contains_key("hx-request")
}

/// De geavanceerde weergave: alles wat er ruw in één bestand staat (FR-7).
#[derive(Template)]
#[template(path = "rawtags.html")]
struct RawTagsTemplate {
    /// Bestandsnaam, als kop van de pagina.
    name: String,

    /// Tot en met de map waarin het bestand staat.
    crumbs: Vec<Crumb>,

    raw: RawTags,
}

/// Toont alle ruwe tags van één bestand, alleen-lezen.
///
/// Deze pagina bestaat om te kunnen zien wát er werkelijk in een bestand staat,
/// inclusief velden die het genormaliseerde model niet kent. Ze biedt bewust
/// geen enkele manier om er iets aan te veranderen: ruwe frames bewerken is
/// geen doel van het MVP.
async fn raw_tags_of_file(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Html<String>, WebError> {
    let library = Arc::clone(&state.library);
    let wanted = path.clone();

    let raw = tokio::task::spawn_blocking(move || {
        let absolute = library.resolve(&wanted)?;
        tags::read_raw_tags(&absolute).map_err(WebError::from)
    })
    .await??;

    let page = RawTagsTemplate {
        name: browse::name_of_file(&path).to_string(),
        crumbs: browse::crumbs_to_parent(&path),
        raw,
    };

    Ok(Html(page.render()?))
}

/// De hoesweergave van één bestand (FR-12).
#[derive(Template)]
#[template(path = "cover.html")]
struct CoverTemplate {
    page: CoverPage,
}

/// Toont de embedded hoes groot, met formaat, afmetingen en grootte.
///
/// De feiten komen uit dezelfde leesronde als de tags: `tags::read` beschrijft
/// de hoes zonder de pixels uit te pakken, dus deze pagina kost niet meer dan
/// het openen van het bestand plus het opsommen van de map.
async fn cover_of_file(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Html<String>, WebError> {
    let page = load_cover(&state, &path, None, None).await?;
    Ok(Html(CoverTemplate { page }.render()?))
}

/// Bouwt de hoespagina van één bestand.
///
/// De map wordt erbij opgesomd om te weten hoeveel tracks er zijn; dat is
/// alleen een `read_dir` en geen leesronde over de tags.
async fn load_cover(
    state: &AppState,
    path: &str,
    notice: Option<cover::Notice>,
    report: Option<batch::SaveReport>,
) -> Result<CoverPage, WebError> {
    let library = Arc::clone(&state.library);
    let wanted = path.to_string();

    let (track, tracks_in_folder, folder_cover) = tokio::task::spawn_blocking(move || {
        let absolute = library.resolve(&wanted)?;
        let track = tags::read(&absolute).map_err(WebError::from)?;

        let siblings = library
            .list_directory(browse::parent_of(&wanted))
            .map(|contents| contents.files.len())
            .unwrap_or(1);

        Ok::<_, WebError>((track, siblings, existing_folder_cover(&library, &absolute)))
    })
    .await??;

    Ok(CoverPage {
        name: browse::name_of_file(path).to_string(),
        crumbs: browse::crumbs_to_parent(path),
        art_url: browse::art_url(path),
        edit_url: browse::edit_url(path),
        url: browse::cover_url(path),
        back_url: browse::url_for(browse::parent_of(path)),
        tracks_in_folder,
        details: track.art.as_ref().map(CoverDetails::of),
        folder_cover,
        notice,
        report,
        max_upload_mb: state.art_limits.max_upload_mb,
    })
}

/// De losse hoes die al naast dit bestand staat, als die er is.
///
/// Alleen om de gebruiker te laten zien waar hij ja tegen zegt; het bestand
/// wordt niet geopend voor zijn inhoud, alleen voor zijn omvang.
fn existing_folder_cover(
    library: &Library,
    absolute: &std::path::Path,
) -> Option<cover::FolderCover> {
    let path = library.sibling(absolute, cover::FOLDER_COVER).ok()?;
    let size = std::fs::metadata(&path)
        .ok()
        .filter(|meta| meta.is_file())?
        .len();

    Some(cover::FolderCover::new(cover::FOLDER_COVER, size as usize))
}

/// Wat er met de hoes moet gebeuren, en waar.
#[derive(Debug, Default)]
struct CoverForm {
    /// De aangeklikte knop.
    action: String,

    /// De aangeleverde bytes; leeg bij een verwijderactie.
    upload: Vec<u8>,

    /// Of de hoes ook als los bestand in de map moet komen (FR-14).
    as_file: bool,

    /// Of een bestaande `cover.jpg` vervangen mag worden.
    ///
    /// Die bevestiging moet vóór het versturen gegeven worden: na een POST is
    /// de bestandsinvoer van de browser leeg, dus een tweede ronde waarin de
    /// gebruiker alsnog ja zegt bestaat hier niet.
    overwrite: bool,
}

impl CoverForm {
    /// Of de actie op de hele map slaat in plaats van op dit ene bestand.
    fn whole_folder(&self) -> bool {
        self.action.ends_with("-alle")
    }

    /// Of de hoes verwijderd moet worden.
    fn removes(&self) -> bool {
        self.action.starts_with("verwijder")
    }
}

/// Embedt een geüploade hoes of verwijdert de bestaande (FR-13 en FR-16).
///
/// De afbeelding gaat eerst door [`art::prepare`]: valideren op de bytes zelf
/// en verkleinen wat te groot is. Pas daarna wordt er geschreven, bestand voor
/// bestand — een fout bij het ene bestand houdt het andere niet tegen.
///
/// Na afloop wordt de situatie opnieuw ingelezen: wat er op het scherm staat,
/// staat werkelijk in het bestand.
async fn save_cover(
    State(state): State<AppState>,
    Path(path): Path<String>,
    multipart: Multipart,
) -> Result<Html<String>, WebError> {
    let form = read_cover_form(multipart).await?;

    // Verwijderen heeft geen afbeelding nodig; embedden wel.
    let prepared = if form.removes() {
        None
    } else {
        match art::prepare(&form.upload, state.art_limits) {
            Ok(prepared) => Some(prepared),
            Err(error) => {
                tracing::info!(%error, "aangeleverde hoes is geweigerd");

                let page = load_cover(
                    &state,
                    &path,
                    Some(cover::Notice::Refused(format!(
                        "Er is niets gewijzigd: {error}."
                    ))),
                    None,
                )
                .await?;

                return Ok(Html(CoverTemplate { page }.render()?));
            }
        }
    };

    let mut report = write_cover(&state, &path, form.whole_folder(), prepared.as_ref()).await?;

    // Pas ná het embedden, en met een eigen regel in hetzelfde rapport: gaat
    // dit mis, dan blijft wat er wél geschreven is gewoon staan (FR-14).
    if let Some(prepared) = prepared.as_ref()
        && form.as_file
    {
        let result = write_folder_cover(&state, &path, prepared, form.overwrite).await?;
        report.results.push(result);
    }

    let notice = prepared.as_ref().map(cover::Notice::accepted);

    let page = load_cover(&state, &path, notice, Some(report)).await?;
    Ok(Html(CoverTemplate { page }.render()?))
}

/// Leest het multipart-formulier van de hoespagina.
async fn read_cover_form(mut multipart: Multipart) -> Result<CoverForm, WebError> {
    let mut form = CoverForm::default();

    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("actie") => form.action = field.text().await?,
            Some("afbeelding") => form.upload = field.bytes().await?.to_vec(),
            // Een vinkje komt alleen mee wanneer het aan staat; de waarde zelf
            // doet er niet toe.
            Some("mapbestand") => {
                let _ = field.bytes().await?;
                form.as_file = true;
            }
            Some("overschrijf") => {
                let _ = field.bytes().await?;
                form.overwrite = true;
            }
            // Een onbekend veld hoort de rest niet tegen te houden, maar moet
            // wel uitgelezen worden om bij het volgende te komen.
            _ => {
                let _ = field.bytes().await?;
            }
        }
    }

    Ok(form)
}

/// Schrijft de hoes naar één bestand of naar de hele map.
///
/// Bestand voor bestand, met per bestand een uitkomst: dezelfde regel als bij
/// de batch-tagbewerking, en om dezelfde reden — één onschrijfbaar bestand mag
/// de rest van het album niet tegenhouden.
async fn write_cover(
    state: &AppState,
    path: &str,
    whole_folder: bool,
    prepared: Option<&art::Prepared>,
) -> Result<batch::SaveReport, WebError> {
    let library = Arc::clone(&state.library);
    let options = state.write_options;
    let cover = prepared.map(|prepared| (prepared.mime.clone(), prepared.data.clone()));
    let wanted = path.to_string();

    let results = tokio::task::spawn_blocking(move || {
        let targets = cover_targets(&library, &wanted, whole_folder);

        targets
            .into_iter()
            .map(|(name, absolute)| batch::SaveResult {
                name,
                outcome: write_one_cover(&absolute, cover.as_ref(), options),
            })
            .collect()
    })
    .await?;

    Ok(batch::SaveReport { results })
}

/// De bestanden die deze actie raakt, met hun naam voor in het rapport.
fn cover_targets(
    library: &Library,
    path: &str,
    whole_folder: bool,
) -> Vec<(String, std::path::PathBuf)> {
    if !whole_folder {
        return match library.resolve(path) {
            Ok(absolute) => vec![(browse::name_of_file(path).to_string(), absolute)],
            // Een pad dat niet mag, levert geen doel op; de lus hieronder heeft
            // dan niets te doen en het rapport blijft leeg.
            Err(_) => Vec::new(),
        };
    }

    library
        .list_directory(browse::parent_of(path))
        .map(|contents| {
            contents
                .files
                .into_iter()
                .map(|entry| (entry.name, entry.path))
                .collect()
        })
        .unwrap_or_default()
}

/// Zet of verwijdert de hoes van één bestand.
fn write_one_cover(
    absolute: &std::path::Path,
    cover: Option<&(String, Vec<u8>)>,
    options: crate::atomic::Options,
) -> batch::Outcome {
    let cover = cover.map(|(mime, data)| (mime.as_str(), data.as_slice()));

    match tags::write_art(absolute, cover, options) {
        Ok(written) if written.changed => {
            let mut labels = vec![if cover.is_some() {
                "Hoes".to_string()
            } else {
                "Hoes verwijderd".to_string()
            }];
            labels.extend(written.removal_labels());

            batch::Outcome::Saved(labels)
        }
        Ok(_) => batch::Outcome::Unchanged,
        Err(error) => {
            tracing::error!(path = %absolute.display(), %error, "de hoes kon niet weggeschreven worden");
            batch::Outcome::Failed(error.to_string())
        }
    }
}

/// Zet de hoes ook als `cover.jpg` in de albummap (FR-14).
///
/// Dit is de enige plek waar Sleeve een nieuw bestand in de bibliotheek
/// aanmaakt. Het loopt daarom langs [`atomic::place`]: atomisch, met eigenaar,
/// groep en rechten van de track ernaast, en alleen over een bestaand bestand
/// heen wanneer de gebruiker dat heeft aangevinkt.
///
/// De uitkomst komt als gewone regel in het rapport terug; een fout hier laat
/// het geslaagde embedden onaangetast.
async fn write_folder_cover(
    state: &AppState,
    path: &str,
    prepared: &art::Prepared,
    overwrite: bool,
) -> Result<batch::SaveResult, WebError> {
    let library = Arc::clone(&state.library);
    let quality = state.art_limits.quality;
    let data = prepared.data.clone();
    let wanted = path.to_string();

    let outcome = tokio::task::spawn_blocking(move || {
        let track = library.resolve(&wanted)?;
        let target = library.sibling(&track, cover::FOLDER_COVER)?;

        // Eén vaste naam, dus ook één vast formaat; een PNG wordt hier JPEG.
        let jpeg = match art::as_jpeg(&data, quality) {
            Ok(jpeg) => jpeg,
            Err(error) => return Ok(batch::Outcome::Failed(format!("{error}."))),
        };

        let permission = if overwrite {
            atomic::Overwrite::Allow
        } else {
            atomic::Overwrite::Refuse
        };

        Ok::<_, PathError>(match atomic::place(&target, &jpeg, &track, permission) {
            Ok(atomic::Placement::Created) => {
                batch::Outcome::Saved(vec!["Nieuw in de map gezet".to_string()])
            }
            Ok(atomic::Placement::Replaced) => {
                batch::Outcome::Saved(vec!["Vervangen in de map".to_string()])
            }
            Ok(atomic::Placement::Unchanged) => batch::Outcome::Unchanged,
            Err(atomic::PlaceError::Exists) => batch::Outcome::Failed(
                "er stond al een cover.jpg in de map en overschrijven was niet aangevinkt."
                    .to_string(),
            ),
            Err(error) => {
                tracing::error!(path = %target.display(), %error, "cover.jpg kon niet weggeschreven worden");
                batch::Outcome::Failed(format!("{error}."))
            }
        })
    })
    .await??;

    Ok(batch::SaveResult {
        name: cover::FOLDER_COVER.to_string(),
        outcome,
    })
}

/// Het bewerkformulier van één bestand (FR-5 en FR-6).
#[derive(Template)]
#[template(path = "edit.html")]
struct EditTemplate {
    page: EditPage,
}

/// Toont het formulier met de waarden die nu in het bestand staan.
/// Waar de gebruiker vandaan kwam toen hij dit formulier opende.
///
/// Alleen om de weg terug te wijzen. Wat er niet in staat, is de selectie die
/// hij in de albumweergave had: die leeft in een verstuurd formulier en niet in
/// een URL. De terug-knop van de browser brengt hem daar wél compleet terug.
#[derive(Debug, Default, serde::Deserialize)]
struct EditQuery {
    #[serde(default)]
    terug: String,
}

async fn edit_form(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<EditQuery>,
) -> Result<Html<String>, WebError> {
    let page = load_page(&state, path, &query.terug, None, None).await?;
    Ok(Html(EditTemplate { page }.render()?))
}

/// Slaat de ingevulde waarden op en toont daarna wat er werkelijk in staat.
///
/// Na een geslaagde schrijfactie wordt het bestand **opnieuw ingelezen** en
/// worden die waarden getoond. Dat is het hele punt van FR-6: de bevestiging
/// komt uit het bestand en niet uit wat de gebruiker net intikte, want alleen
/// dan zegt hij iets.
///
/// Er wordt bewust niet doorverwezen na het opslaan. Het herladen van een POST
/// is hier ongevaarlijk: `tags::write` doet niets wanneer er niets verandert,
/// dus een tweede keer versturen van dezelfde waarden raakt het bestand niet
/// aan. Dat scheelt een flash-mechanisme om de bevestiging te bewaren.
async fn save_tags(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<EditQuery>,
    Form(fields): Form<edit::Form>,
) -> Result<Html<String>, WebError> {
    // Eerst de invoer, dan pas het bestand: een typefout in een tracknummer
    // hoort geen schrijfactie te starten die halverwege afketst.
    let wanted = match fields.to_tags() {
        Ok(tags) => tags,
        Err(problems) => {
            let page = load_page(
                &state,
                path,
                &query.terug,
                Some(fields),
                Some(Notice::Failed(problems)),
            )
            .await?;
            return Ok(Html(EditTemplate { page }.render()?));
        }
    };

    let library = Arc::clone(&state.library);
    let options = state.write_options;
    let target = path.clone();

    let outcome = tokio::task::spawn_blocking(move || {
        let absolute = library.resolve(&target)?;
        Ok::<_, PathError>(tags::write(&absolute, &wanted, options))
    })
    .await??;

    let notice = match outcome {
        Ok(written) => {
            let mut lines = vec![
                "Opgeslagen. Hieronder staat wat er nu werkelijk in het bestand staat.".to_string(),
            ];
            lines.extend(written.removal_notice());

            Notice::Saved(lines)
        }

        Err(error) => {
            tracing::error!(path = %path, %error, "tags konden niet opgeslagen worden");

            // De ingevulde waarden blijven staan, zodat de gebruiker het
            // opnieuw kan proberen zonder alles over te typen.
            let page = load_page(
                &state,
                path,
                &query.terug,
                Some(fields),
                Some(Notice::Failed(vec![format!(
                    "Er is niets opgeslagen: {error}. Het bestand is onveranderd gebleven."
                )])),
            )
            .await?;
            return Ok(Html(EditTemplate { page }.render()?));
        }
    };

    let page = load_page(&state, path, &query.terug, None, Some(notice)).await?;
    Ok(Html(EditTemplate { page }.render()?))
}

/// Bouwt de bewerkpagina van één bestand.
///
/// `fields` overschrijft wat er in de invoervelden komt te staan; zonder die
/// waarde komen ze uit het bestand. Dat onderscheid is het verschil tussen "hier
/// is wat er in het bestand staat" en "hier is wat je zojuist intikte, probeer
/// het nog eens".
async fn load_page(
    state: &AppState,
    path: String,
    origin: &str,
    fields: Option<edit::Form>,
    notice: Option<Notice>,
) -> Result<EditPage, WebError> {
    let library = Arc::clone(&state.library);
    let target = path.clone();

    let track = tokio::task::spawn_blocking(move || {
        let absolute = library.resolve(&target)?;
        tags::read(&absolute).map_err(WebError::from)
    })
    .await??;

    let from_album = origin == browse::FROM_ALBUM;
    let parent = browse::parent_of(&path);

    Ok(EditPage {
        name: browse::name_of_file(&path).to_string(),
        crumbs: browse::crumbs_to_parent(&path),
        // De herkomst blijft in het adres staan, ook na het opslaan: het
        // formulier post naar deze URL, en de weg terug hoort daarna nog
        // dezelfde te zijn.
        url: if from_album {
            browse::edit_url_from_album(&path)
        } else {
            browse::edit_url(&path)
        },
        back_url: if from_album {
            browse::album_url(parent)
        } else {
            browse::url_for(parent)
        },
        back_label: if from_album {
            "Terug naar de albumweergave".to_string()
        } else {
            "Terug naar de map".to_string()
        },
        raw_url: browse::raw_tags_url(&path),
        art_url: browse::art_url(&path),
        has_art: track.art.is_some(),
        cover_url: browse::cover_url(&path),
        format: track.format.to_string(),
        duration: browse::format_duration(track.duration),
        fields: fields.unwrap_or_else(|| edit::Form::from_tags(&track.tags)),
        notice,
        max_upload_mb: state.art_limits.max_upload_mb,
    })
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

    #[error("het formulier kon niet gelezen worden: {0}")]
    Upload(#[from] axum::extract::multipart::MultipartError),

    /// De body van een gewoon formulier was niet te lezen.
    ///
    /// In de praktijk: een afgebroken verbinding of een body die de grens
    /// overschrijdt. Er is dan niets gebeurd.
    #[error("het formulier kon niet gelezen worden")]
    Unreadable,
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

                    // Schrijffouten liggen niet aan het verzoek; het bestand is
                    // op dat moment nog onaangetast.
                    tags::TagError::Unwritable | tags::TagError::Mismatch => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };

                tracing::warn!(%error, %status, "bestand kon niet gelezen worden");
                (status, error.to_string()).into_response()
            }

            // Geen fout in de aanvraag, maar er is niets te tonen.
            WebError::NoArt => (StatusCode::NOT_FOUND, WebError::NoArt.to_string()).into_response(),

            // Een onleesbaar of te groot formulier: het verzoek klopt niet, en
            // er is niets geschreven.
            WebError::Upload(error) => {
                tracing::warn!(%error, "upload kon niet gelezen worden");
                (
                    error.status(),
                    "De upload kon niet gelezen worden. Is de afbeelding misschien te groot?"
                        .to_string(),
                )
                    .into_response()
            }

            WebError::Unreadable => {
                tracing::warn!("het formulier kon niet gelezen worden");
                (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Het formulier kon niet gelezen worden. Is er iets te groots meegestuurd?"
                        .to_string(),
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

    /// Bouwt een bibliotheek met één album met en zonder hoes erin.
    fn root_with_art() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("tempdir");
        let album = root.path().join("Album");
        std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

        crate::testfixtures::copy_to(&album, crate::testfixtures::MP3_WITH_ART);
        crate::testfixtures::copy_to(&album, crate::testfixtures::MP3_WITH_TAGS);
        crate::testfixtures::copy_to(&album, crate::testfixtures::FLAC_WITH_TAGS);
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
    async fn the_listing_shows_what_is_wrong_in_the_directory() {
        let root = root_with_art();
        let html = body_as_string(get(&root, "/map/Album").await).await;

        // Beide fixtures hebben tracknummer 3; dat hoort de pagina te melden.
        assert!(
            html.contains("Let op in deze map"),
            "de mapmeldingen ontbreken: {html}"
        );
        assert!(
            html.contains("tracknummer 3 komt meer dan eens voor"),
            "het dubbele tracknummer wordt niet gemeld: {html}"
        );

        // En per bestand: de MP3 zonder hoes hoort dat als label te krijgen.
        assert!(
            html.contains("geen hoes"),
            "het ontbreken van een hoes wordt niet gemarkeerd: {html}"
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

    #[tokio::test]
    async fn raw_tags_show_the_original_key_names() {
        let root = root_with_art();

        // MP3: ID3v2-frames.
        let mp3 = body_as_string(get(&root, "/tags/Album/tagged.mp3").await).await;
        assert!(mp3.contains("ID3v2"), "de tagsoort ontbreekt: {mp3}");
        assert!(mp3.contains("TIT2"), "het titelframe ontbreekt: {mp3}");
        assert!(mp3.contains("TPE1"), "het artiestframe ontbreekt: {mp3}");

        // FLAC: Vorbis-comments, met hun eigen namen.
        let flac = body_as_string(get(&root, "/tags/Album/tagged.flac").await).await;
        assert!(
            flac.contains("Vorbis-comments"),
            "de tagsoort ontbreekt: {flac}"
        );
        assert!(flac.contains("TITLE"), "het titelveld ontbreekt: {flac}");
        assert!(flac.contains("ARTIST"), "het artiestveld ontbreekt: {flac}");
        assert!(
            !flac.contains("TIT2"),
            "een FLAC hoort geen ID3v2-frames te tonen: {flac}"
        );
    }

    #[tokio::test]
    async fn raw_tags_summarise_the_embedded_art() {
        let root = root_with_art();
        let html = body_as_string(get(&root, "/tags/Album/tagged-with-art.mp3").await).await;

        // Type en grootte, niet de data zelf.
        assert!(
            html.contains("image/jpeg") && html.contains("bytes"),
            "de hoes wordt niet samengevat: {html}"
        );
        assert!(
            html.len() < 20_000,
            "de pagina is {} bytes; dat ruikt naar ruwe afbeeldingsdata",
            html.len()
        );
    }

    #[tokio::test]
    async fn the_advanced_view_offers_no_way_to_change_anything() {
        let root = root_with_art();
        let html = body_as_string(get(&root, "/tags/Album/tagged.mp3").await).await;

        // Ruwe frames bewerken is geen doel van het MVP; deze pagina hoort
        // daarom geen enkel bedienbaar element te bevatten.
        //
        // De kopbalk telt niet mee: die is op elke pagina hetzelfde en gaat
        // over de weergave, niet over dit bestand.
        let inhoud = html
            .split_once("<main")
            .map(|(_, rest)| rest)
            .expect("de pagina hoort een inhoudsblok te hebben");

        for forbidden in ["<form", "<input", "<textarea", "<button", "<select"] {
            assert!(
                !inhoud.contains(forbidden),
                "'{forbidden}' staat op een alleen-lezen pagina: {html}"
            );
        }
        assert!(
            html.contains("alleen-lezen"),
            "de pagina zegt niet dat ze alleen-lezen is: {html}"
        );
    }

    #[tokio::test]
    async fn raw_tags_refuse_what_is_not_audio() {
        let root = root_with_art();

        assert_eq!(
            get(&root, "/tags/Album/notities.txt").await.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(
            get(&root, "/tags/Album/bestaat-niet.mp3").await.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get(&root, "/tags/../../etc/passwd").await.status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn the_listing_leads_to_the_edit_form() {
        let root = root_with_art();
        let html = body_as_string(get(&root, "/map/Album").await).await;

        assert!(
            html.contains(r#"href="/bewerk/Album/tagged.mp3""#),
            "er is geen weg naar het bewerkformulier: {html}"
        );
        assert!(
            !html.contains(r#"href="/tags/Album/tagged.mp3""#),
            "de geavanceerde weergave hoort vanaf de bestandspagina bereikbaar \
             te zijn, niet vanuit de lijst: {html}"
        );
    }

    /// Doet één POST met formuliervelden.
    async fn post_form(
        root: &tempfile::TempDir,
        uri: &str,
        fields: &[(&str, &str)],
    ) -> axum::response::Response {
        let body = fields
            .iter()
            .map(|(name, value)| {
                format!(
                    "{}={}",
                    name,
                    percent_encoding::utf8_percent_encode(
                        value,
                        percent_encoding::NON_ALPHANUMERIC
                    )
                )
            })
            .collect::<Vec<_>>()
            .join("&");

        test_router(root)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .expect("verzoek"),
            )
            .await
            .expect("respons")
    }

    /// Alle kernvelden, zodat een POST het hele formulier meestuurt zoals een
    /// browser dat doet.
    fn form_fields<'a>(title: &'a str, track: &'a str) -> Vec<(&'a str, &'a str)> {
        vec![
            ("title", title),
            ("artist", "De Testartiest"),
            ("album_artist", "De Albumartiest"),
            ("album", "Fixtures voor Sleeve"),
            ("track", track),
            ("track_total", "12"),
            ("disc", "1"),
            ("disc_total", "2"),
            ("year", "2024"),
            ("genre", "Ambient"),
            ("composer", "De Componist"),
            ("comment", "Een commentaar"),
        ]
    }

    #[tokio::test]
    async fn the_edit_form_shows_the_current_values() {
        let root = root_with_art();
        let respons = get(&root, "/bewerk/Album/tagged.mp3").await;

        assert_eq!(respons.status(), StatusCode::OK);
        let html = body_as_string(respons).await;

        // De waarden uit de fixture, in de invoervelden.
        assert!(
            html.contains(r#"value="Stilte in D""#),
            "de titel staat niet in het formulier: {html}"
        );
        assert!(
            html.contains(r#"value="De Testartiest""#),
            "de artiest staat niet in het formulier: {html}"
        );
        assert!(
            html.contains(r#"value="3""#),
            "het tracknummer staat niet in het formulier: {html}"
        );

        // En de uitleg over wat een leeg veld doet.
        assert!(
            html.contains("leegmaken verwijdert"),
            "de pagina legt niet uit wat een leeg veld betekent: {html}"
        );
    }

    #[tokio::test]
    async fn saving_writes_the_file_and_shows_what_came_back() {
        let root = root_with_art();

        let respons = post_form(
            &root,
            "/bewerk/Album/tagged.mp3",
            &form_fields("Een nieuwe titel", "7"),
        )
        .await;

        assert_eq!(respons.status(), StatusCode::OK);
        let html = body_as_string(respons).await;

        assert!(
            html.contains("Opgeslagen"),
            "er is geen bevestiging: {html}"
        );
        assert!(
            html.contains(r#"value="Een nieuwe titel""#),
            "de nieuwe titel staat niet in het formulier: {html}"
        );

        // En het bestand op schijf draagt hem werkelijk.
        let path = root.path().join("Album").join("tagged.mp3");
        let tags = crate::tags::read(&path).expect("teruglezen").tags;
        assert_eq!(tags.title.as_deref(), Some("Een nieuwe titel"));
        assert_eq!(tags.track, Some(7));
    }

    #[tokio::test]
    async fn an_emptied_field_removes_the_tag() {
        let root = root_with_art();

        let mut fields = form_fields("Stilte in D", "3");
        fields.retain(|(name, _)| *name != "composer");
        fields.push(("composer", ""));

        let respons = post_form(&root, "/bewerk/Album/tagged.mp3", &fields).await;
        assert_eq!(respons.status(), StatusCode::OK);

        let path = root.path().join("Album").join("tagged.mp3");
        assert_eq!(
            crate::tags::read(&path).expect("teruglezen").tags.composer,
            None,
            "de componist is niet verwijderd"
        );
    }

    #[tokio::test]
    async fn invalid_input_is_refused_before_anything_is_written() {
        let root = root_with_art();
        let path = root.path().join("Album").join("tagged.mp3");
        let before = std::fs::read(&path).expect("lezen");

        let respons = post_form(
            &root,
            "/bewerk/Album/tagged.mp3",
            &form_fields("Een nieuwe titel", "drie"),
        )
        .await;

        assert_eq!(respons.status(), StatusCode::OK);
        let html = body_as_string(respons).await;

        assert!(
            html.contains("Tracknummer moet een getal"),
            "de fout wordt niet uitgelegd: {html}"
        );
        // De ingevulde waarden blijven staan, zodat er niets overgetypt hoeft.
        assert!(
            html.contains(r#"value="Een nieuwe titel""#),
            "de ingevulde titel is kwijt: {html}"
        );
        assert!(
            html.contains(r#"value="drie""#),
            "de foute invoer is kwijt: {html}"
        );

        assert_eq!(
            std::fs::read(&path).expect("lezen"),
            before,
            "er is geschreven ondanks ongeldige invoer"
        );
    }

    #[tokio::test]
    async fn the_edit_form_refuses_what_it_should() {
        let root = root_with_art();

        assert_eq!(
            get(&root, "/bewerk/Album/notities.txt").await.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(
            get(&root, "/bewerk/Album/bestaat-niet.mp3").await.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get(&root, "/bewerk/../../etc/passwd").await.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            post_form(&root, "/bewerk/../../etc/passwd", &form_fields("x", "1"))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn the_edit_form_leads_to_the_advanced_view() {
        let root = root_with_art();
        let html = body_as_string(get(&root, "/bewerk/Album/tagged.mp3").await).await;

        assert!(
            html.contains(r#"href="/tags/Album/tagged.mp3""#),
            "er is geen weg naar de geavanceerde weergave: {html}"
        );
    }
}
