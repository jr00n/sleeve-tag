//! Het weergavemodel van één map: wat de mapbrowser op het scherm zet.
//!
//! Deze module brengt de padlaag ([`crate::fs`]) en het tagmodel
//! ([`crate::tags`]) bij elkaar en levert een structuur die de templates
//! rechtstreeks kunnen renderen. Ze opent zelf geen bestanden en bouwt zelf geen
//! paden op: tag-I/O gaat uitsluitend via [`crate::tags`], zoals de
//! architectuurtest afdwingt.
//!
//! Alles wat naar de browser gaat is relatief aan `MUSIC_ROOT`. Het absolute
//! pad van de NAS blijft binnen [`crate::fs`].

use std::path::Path;
use std::time::Duration;

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use crate::checks::{self, FolderIssue, TrackIssue};
use crate::fs::{DirEntry, Library, PathError};
use crate::tags::{self, Tags};

/// Naam van de bibliotheekwortel in het broodkruimelpad.
///
/// De gebruiker hoeft niet te weten dat de map in de container `/music` heet.
const ROOT_NAME: &str = "Bibliotheek";

/// Wat er staat waar een tag ontbreekt.
const MISSING: &str = "—";

/// Waarde van de `size`-parameter waarmee om de verkleinde hoes wordt gevraagd.
///
/// Staat hier omdat de URL's hier worden opgebouwd; het endpoint in
/// [`crate::web`] leest dezelfde constante, zodat de twee niet uit elkaar
/// kunnen lopen.
pub const THUMBNAIL_SIZE_PARAM: &str = "thumb";

/// Tekens die in een padsegment van een URL gecodeerd moeten worden.
///
/// `/` blijft er bewust buiten: het scheidt de segmenten en hoort niet gecodeerd
/// te worden. `?` en `#` juist wel, anders begint de browser halverwege een
/// mapnaam aan een query of een fragment.
const PATH_ESCAPES: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'%');

/// Eén stap in het broodkruimelpad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub name: String,
    pub url: String,
}

/// Een submap zoals de browser hem toont.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    pub url: String,
}

/// Eén regel in de bestandslijst.
///
/// De `Option`-velden komen rechtstreeks uit het tagmodel; de `*_label`-methoden
/// maken er tekst van die een template zonder verdere logica kan tonen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackSummary {
    /// Bestandsnaam, inclusief extensie.
    pub name: String,

    /// Pad relatief aan `MUSIC_ROOT`; het handvat voor latere bewerkacties.
    pub path: String,

    /// Het volledige genormaliseerde tagmodel van dit bestand.
    ///
    /// De lijst toont er maar een handvol velden van, maar de signalering
    /// kijkt ook naar albumartiest en jaar, en het bewerkformulier heeft
    /// straks alles nodig.
    pub tags: Tags,

    /// Wat er aan dit bestand mankeert; leeg wanneer er niets te melden is.
    pub issues: Vec<TrackIssue>,

    /// Tagblokken in dit bestand die niet bij het formaat horen, bij naam.
    ///
    /// Komt uit `tags::` en gaat naar de signalering; de lijst zelf toont het
    /// niet, want daar staat de melding die er uit volgt al.
    pub foreign_tags: Vec<String>,

    /// Speelduur als `m:ss`, of `u:mm:ss` vanaf een uur.
    pub duration: String,

    /// `MP3` of `FLAC`.
    pub format: String,

    /// Wat er over de embedded hoes bekend is; `None` wanneer het bestand er
    /// geen heeft.
    ///
    /// De maplijst gebruikt alleen het bestaan ervan — dat bepaalt of ze een
    /// afbeelding of een placeholder toont, zodat de browser geen verzoek doet
    /// dat toch een 404 oplevert. De signalering kijkt naar de inhoud: twee
    /// tracks van hetzelfde album horen dezelfde hoes te hebben.
    pub art: Option<crate::tags::ArtInfo>,

    /// URL van de verkleinde hoes. Alleen zinvol wanneer er een hoes is.
    pub art_url: String,

    /// URL van het bewerkformulier van dit bestand.
    ///
    /// De ingang naar de geavanceerde weergave zit dáár en niet in deze lijst:
    /// FR-7 beschrijft die als onderdeel van de bestandspagina, en een tweede
    /// link per regel maakt de lijst op een telefoon alleen maar drukker.
    pub edit_url: String,
}

impl TrackSummary {
    /// Of er een embedded hoes in dit bestand zit.
    pub fn has_art(&self) -> bool {
        self.art.is_some()
    }

    /// Het tracknummer, of een lege tekst wanneer het ontbreekt.
    ///
    /// Bewust leeg en niet `—`: in een smalle kolom vóór de titel is een streepje
    /// per regel meer ruis dan informatie.
    pub fn track_label(&self) -> String {
        self.tags
            .track
            .map(|number| number.to_string())
            .unwrap_or_default()
    }

    /// De titel, of een streepje wanneer die ontbreekt.
    ///
    /// De bestandsnaam invullen zou vriendelijk lijken, maar verbergt precies
    /// wat de gebruiker moet zien: hier staat geen titel in het bestand.
    pub fn title_label(&self) -> &str {
        self.tags.title.as_deref().unwrap_or(MISSING)
    }

    pub fn artist_label(&self) -> &str {
        self.tags.artist.as_deref().unwrap_or(MISSING)
    }

    pub fn album_label(&self) -> &str {
        self.tags.album.as_deref().unwrap_or(MISSING)
    }
}

/// Alles wat één mappagina nodig heeft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// Naam van de map zelf; voor de root de naam van de bibliotheek.
    pub name: String,

    /// Pad van de map relatief aan `MUSIC_ROOT`; leeg voor de root.
    pub path: String,

    /// URL van deze pagina, zonder query. Het formulier en de HTMX-aanroep
    /// wijzen hiernaartoe.
    pub url: String,

    /// URL van de albumweergave van deze map: dezelfde bestanden, maar dan om
    /// er met een selectie tegelijk aan te werken (FR-8).
    pub album_url: String,

    /// Van de bibliotheekwortel tot en met deze map.
    pub crumbs: Vec<Crumb>,

    pub folders: Vec<Folder>,
    pub tracks: Vec<TrackSummary>,

    /// Wat er tussen de bestanden van deze map onderling niet klopt.
    ///
    /// Wordt over de héle map bepaald, ook wanneer er gefilterd wordt: aan de
    /// map verandert niets doordat de gebruiker zoekt.
    pub folder_issues: Vec<FolderIssue>,

    /// De zoekterm zoals de gebruiker hem heeft ingevuld.
    pub query: String,
}

/// Bouwt het weergavemodel van één map.
///
/// `relative` is het door de gebruiker aangeleverde pad en gaat ongewijzigd naar
/// [`Library::list_directory`], die het controleert. `query` filtert binnen deze
/// map op bestandsnaam of titel.
///
/// Dit is blokkerende I/O: elk bestand wordt geopend om zijn tags te lezen. De
/// aanroeper hoort dat buiten de async-runtime te doen.
pub fn listing(library: &Library, relative: &str, query: &str) -> Result<Listing, PathError> {
    let contents = library.list_directory(relative)?;

    let path = library
        .relative_path(&contents.path)
        .map(to_url_path)
        .unwrap_or_default();

    let needle = query.trim().to_lowercase();

    let folders: Vec<Folder> = contents
        .directories
        .iter()
        .filter(|entry| needle.is_empty() || entry.name.to_lowercase().contains(&needle))
        .map(|entry| Folder {
            url: url_for(&join(&path, &entry.name)),
            name: entry.name.clone(),
        })
        .collect();

    // Eerst de hele map inlezen en beoordelen, dan pas filteren: een melding
    // als "twee verschillende albumtitels" hoort niet te verdwijnen zodra de
    // gebruiker zoekt.
    let mut tracks: Vec<TrackSummary> = contents
        .files
        .iter()
        .filter_map(|entry| summarize(entry, &path))
        .collect();

    let folder_issues = review(&mut tracks);

    tracks.retain(|track| matches_query(track, &needle));
    sort_tracks(&mut tracks);

    Ok(Listing {
        folder_issues,
        name: name_of(&path),
        crumbs: crumbs_for(&path),
        url: url_for(&path),
        album_url: album_url(&path),
        path,
        folders,
        tracks,
        query: query.trim().to_string(),
    })
}

/// Leest één bestand en maakt er een lijstregel van.
///
/// Levert `None` wanneer de tags niet te lezen zijn. Dat is geen fout die de
/// pagina hoort te breken: het betekent dat het bestand ondanks zijn extensie
/// geen MP3 of FLAC is, en dus niet bewerkbaar. Dat is precies het oordeel van
/// `fs::is_editable`, maar dan zonder het bestand een tweede keer te openen.
fn summarize(entry: &DirEntry, directory: &str) -> Option<TrackSummary> {
    let track = match tags::read(&entry.path) {
        Ok(track) => track,
        Err(error) => {
            tracing::debug!(
                path = %entry.path.display(),
                %error,
                "bestand overgeslagen: niet als audio te lezen"
            );
            return None;
        }
    };

    let path = join(directory, &entry.name);

    Some(TrackSummary {
        art_url: thumbnail_url(&path),
        edit_url: edit_url(&path),
        path,
        name: entry.name.clone(),
        duration: format_duration(track.duration),
        format: track.format.to_string(),
        art: track.art,
        tags: track.tags,
        foreign_tags: track.foreign_tags,
        // Wordt hierna ingevuld: wat er aan één bestand mankeert hangt mede af
        // van de rest van de map.
        issues: Vec::new(),
    })
}

/// Laat de signalering over de hele map lopen en hangt de bevindingen op.
///
/// Geeft de meldingen op mapniveau terug; die per bestand komen op de rij zelf
/// terecht.
fn review(tracks: &mut [TrackSummary]) -> Vec<FolderIssue> {
    let entries: Vec<checks::Entry<'_>> = tracks
        .iter()
        .map(|track| checks::Entry {
            tags: &track.tags,
            art: track.art.as_ref(),
            foreign_tags: &track.foreign_tags,
        })
        .collect();

    let review = checks::review(&entries);

    for (track, issues) in tracks.iter_mut().zip(review.tracks) {
        track.issues = issues;
    }

    review.folder
}

/// Sorteert op tracknummer, met de bestandsnaam als terugval.
///
/// Antwoord op het open punt in PRD §12: het tracknummer uit de tags bepaalt de
/// volgorde, want dat is de volgorde waarin het album bedoeld is. Bestanden
/// zonder tracknummer kunnen daar niet tussen worden geplaatst en komen
/// erachter, onderling op naam.
fn sort_tracks(tracks: &mut [TrackSummary]) {
    tracks.sort_by(|a, b| {
        a.tags
            .track
            .unwrap_or(u32::MAX)
            .cmp(&b.tags.track.unwrap_or(u32::MAX))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Of een regel bij de zoekterm past (FR-3): bestandsnaam of titel.
fn matches_query(track: &TrackSummary, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    track.name.to_lowercase().contains(needle)
        || track
            .tags
            .title
            .as_deref()
            .is_some_and(|title| title.to_lowercase().contains(needle))
}

/// Zet een speelduur om naar `m:ss`, of `u:mm:ss` vanaf een uur.
pub fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let (hours, minutes, seconds) = (total / 3600, (total % 3600) / 60, total % 60);

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// De naam van de map zelf, of die van de bibliotheek voor de root.
fn name_of(path: &str) -> String {
    path.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(ROOT_NAME)
        .to_string()
}

/// Het broodkruimelpad van de wortel tot en met deze map (AC #2).
fn crumbs_for(path: &str) -> Vec<Crumb> {
    let mut crumbs = vec![Crumb {
        name: ROOT_NAME.to_string(),
        url: url_for(""),
    }];

    let mut walked = String::new();
    for part in path.split('/').filter(|part| !part.is_empty()) {
        walked = join(&walked, part);
        crumbs.push(Crumb {
            name: part.to_string(),
            url: url_for(&walked),
        });
    }

    crumbs
}

/// De URL van een mappagina; de root is de startpagina.
pub fn url_for(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/map/{}", encode(path))
    }
}

/// De URL van de albumweergave van een map (FR-8).
///
/// De wortel heeft geen padsegment; die krijgt de kale route, net zoals de
/// mapweergave daar `/` gebruikt.
pub fn album_url(path: &str) -> String {
    if path.is_empty() {
        "/album".to_string()
    } else {
        format!("/album/{}", encode(path))
    }
}

/// De URL van de verkleinde hoes van één bestand.
fn thumbnail_url(path: &str) -> String {
    format!("/art/{}?size={THUMBNAIL_SIZE_PARAM}", encode(path))
}

/// De URL van de geavanceerde weergave van één bestand.
pub fn raw_tags_url(path: &str) -> String {
    format!("/tags/{}", encode(path))
}

/// De URL van het bewerkformulier van één bestand.
pub fn edit_url(path: &str) -> String {
    format!("/bewerk/{}", encode(path))
}

/// De URL van de hoes op ware grootte.
pub fn art_url(path: &str) -> String {
    format!("/art/{}", encode(path))
}

/// De URL van de hoesweergave van één bestand (FR-12).
///
/// Niet te verwarren met [`art_url`]: dat is de afbeelding zelf, dit is de
/// pagina eromheen.
pub fn cover_url(path: &str) -> String {
    format!("/hoes/{}", encode(path))
}

/// Broodkruimels tot en met de map waarin dit bestand staat.
///
/// De bestandsnaam zelf hoort er niet bij: die is de kop van de pagina, en een
/// bestand is geen map om naartoe te navigeren.
pub fn crumbs_to_parent(path: &str) -> Vec<Crumb> {
    crumbs_for(parent_of(path))
}

/// De naam van een bestand, los van het pad ernaartoe.
pub fn name_of_file(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Het pad van de map waarin dit bestand staat; leeg voor de wortel.
pub fn parent_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(index) => &path[..index],
        None => "",
    }
}

/// Codeert een relatief pad voor gebruik in een URL.
fn encode(path: &str) -> impl std::fmt::Display {
    utf8_percent_encode(path, PATH_ESCAPES)
}

/// Plakt een naam achter een relatief pad.
fn join(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.to_string()
    } else {
        format!("{directory}/{name}")
    }
}

/// Zet een relatief pad om naar de `/`-notatie die in een URL past.
fn to_url_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testfixtures;

    /// Bouwt een bibliotheek met één album erin.
    ///
    /// De root is een tempdir en wordt gecanonicaliseerd, omdat macOS `/var`
    /// naar `/private/var` laat wijzen.
    fn library_with_album() -> (tempfile::TempDir, Library) {
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        std::fs::create_dir_all(tempdir.path().join("Artiest").join("Album"))
            .expect("albummap moet aan te maken zijn");

        let root =
            std::fs::canonicalize(tempdir.path()).expect("root moet te canonicaliseren zijn");
        (tempdir, Library::new(root))
    }

    /// Kopieert een fixture naar de albummap onder een eigen naam.
    ///
    /// De naam is los van de fixture, zodat een test meerdere tracks met
    /// verschillende bestandsnamen in dezelfde map kan zetten.
    fn place(library: &Library, name: &str, fixture: &str) {
        let album = library.root().join("Artiest").join("Album");
        std::fs::copy(testfixtures::fixture_path(fixture), album.join(name))
            .expect("fixture moet te kopiëren zijn");
    }

    fn album_listing(library: &Library, query: &str) -> Listing {
        listing(library, "Artiest/Album", query).expect("de albummap moet te tonen zijn")
    }

    #[test]
    fn shows_the_fields_from_the_tag_model() {
        let (_tempdir, library) = library_with_album();
        place(&library, "tagged.mp3", testfixtures::MP3_WITH_TAGS);

        let listing = album_listing(&library, "");
        let track = listing.tracks.first().expect("er moet één track staan");

        assert_eq!(track.name, "tagged.mp3");
        assert_eq!(track.path, "Artiest/Album/tagged.mp3");
        assert_eq!(track.format, "MP3");
        assert!(track.tags.track.is_some(), "tracknummer ontbreekt");
        assert!(track.tags.title.is_some(), "titel ontbreekt");
        assert!(track.tags.artist.is_some(), "artiest ontbreekt");
        assert!(track.tags.album.is_some(), "album ontbreekt");
        assert_eq!(track.duration, "0:01", "de fixture is één seconde stilte");
    }

    #[test]
    fn shows_subdirectories_and_starts_at_the_root() {
        let (_tempdir, library) = library_with_album();

        let root = listing(&library, "", "").expect("de root moet te tonen zijn");

        assert_eq!(root.name, ROOT_NAME);
        assert_eq!(root.path, "");
        assert_eq!(root.url, "/");
        assert_eq!(
            root.folders,
            vec![Folder {
                name: "Artiest".to_string(),
                url: "/map/Artiest".to_string(),
            }]
        );
        assert!(root.tracks.is_empty());
    }

    #[test]
    fn breadcrumbs_lead_back_to_the_root() {
        let (_tempdir, library) = library_with_album();

        let listing = album_listing(&library, "");

        assert_eq!(
            listing.crumbs,
            vec![
                Crumb {
                    name: ROOT_NAME.to_string(),
                    url: "/".to_string()
                },
                Crumb {
                    name: "Artiest".to_string(),
                    url: "/map/Artiest".to_string()
                },
                Crumb {
                    name: "Album".to_string(),
                    url: "/map/Artiest/Album".to_string()
                },
            ]
        );
    }

    #[test]
    fn sorts_by_track_number_with_the_filename_as_fallback() {
        let (_tempdir, library) = library_with_album();

        // De getagde fixture heeft tracknummer 3; de ongetagde heeft er geen.
        place(&library, "b-met-nummer.mp3", testfixtures::MP3_WITH_TAGS);
        place(
            &library,
            "c-zonder-nummer.mp3",
            testfixtures::MP3_WITHOUT_TAGS,
        );
        place(
            &library,
            "a-zonder-nummer.flac",
            testfixtures::FLAC_WITHOUT_TAGS,
        );

        let listing = album_listing(&library, "");
        let order: Vec<&str> = listing
            .tracks
            .iter()
            .map(|track| track.name.as_str())
            .collect();

        assert_eq!(
            order,
            vec![
                "b-met-nummer.mp3",
                "a-zonder-nummer.flac",
                "c-zonder-nummer.mp3"
            ],
            "een tracknummer gaat voor; de rest volgt op bestandsnaam"
        );
    }

    #[test]
    fn filters_on_the_filename() {
        let (_tempdir, library) = library_with_album();
        place(&library, "eerste.mp3", testfixtures::MP3_WITHOUT_TAGS);
        place(&library, "tweede.mp3", testfixtures::MP3_WITHOUT_TAGS);

        let listing = album_listing(&library, "TWEE");

        assert_eq!(listing.tracks.len(), 1, "filter is hoofdletterongevoelig");
        assert_eq!(listing.tracks[0].name, "tweede.mp3");
        assert_eq!(listing.query, "TWEE");
    }

    #[test]
    fn filters_on_the_title() {
        let (_tempdir, library) = library_with_album();
        place(&library, "aaa.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "bbb.mp3", testfixtures::MP3_WITHOUT_TAGS);

        let title = {
            let all = album_listing(&library, "");
            all.tracks
                .iter()
                .find_map(|track| track.tags.title.clone())
                .expect("de getagde fixture heeft een titel")
        };

        // Een deel van de titel dat niet in de bestandsnaam voorkomt.
        let listing = album_listing(&library, &title);

        assert_eq!(listing.tracks.len(), 1);
        assert_eq!(listing.tracks[0].name, "aaa.mp3");
    }

    #[test]
    fn filter_also_applies_to_subdirectories() {
        let (_tempdir, library) = library_with_album();
        std::fs::create_dir(library.root().join("Andere artiest"))
            .expect("map moet aan te maken zijn");

        let listing = listing(&library, "", "artiest").expect("de root moet te tonen zijn");

        assert_eq!(
            listing.folders.iter().map(|f| &f.name).collect::<Vec<_>>(),
            vec!["Andere artiest", "Artiest"]
        );

        let none = listing_of_root(&library, "bestaat niet");
        assert!(none.folders.is_empty());
    }

    fn listing_of_root(library: &Library, query: &str) -> Listing {
        listing(library, "", query).expect("de root moet te tonen zijn")
    }

    #[test]
    fn skips_a_file_that_only_looks_like_audio() {
        let (_tempdir, library) = library_with_album();
        place(&library, "echt.mp3", testfixtures::MP3_WITH_TAGS);

        // Juiste extensie, verkeerde inhoud: die hoort niet als bewerkbaar
        // bestand in de lijst te staan.
        place(&library, "nep.mp3", testfixtures::COVER_JPEG);

        let listing = album_listing(&library, "");

        assert_eq!(
            listing.tracks.iter().map(|t| &t.name).collect::<Vec<_>>(),
            vec!["echt.mp3"]
        );
    }

    #[test]
    fn labels_fill_in_missing_tags() {
        let (_tempdir, library) = library_with_album();
        place(&library, "kaal.flac", testfixtures::FLAC_WITHOUT_TAGS);

        let listing = album_listing(&library, "");
        let track = listing.tracks.first().expect("er moet één track staan");

        assert_eq!(track.track_label(), "");
        assert_eq!(track.title_label(), MISSING);
        assert_eq!(track.artist_label(), MISSING);
        assert_eq!(track.album_label(), MISSING);
        assert_eq!(track.format, "FLAC");
        assert!(!track.has_art());
    }

    #[test]
    fn reports_embedded_art_with_a_thumbnail_url() {
        let (_tempdir, library) = library_with_album();
        place(&library, "hoes.mp3", testfixtures::MP3_WITH_ART);

        let listing = album_listing(&library, "");
        let track = &listing.tracks[0];

        assert!(track.has_art(), "de fixture heeft een hoes");
        assert_eq!(
            track.art_url, "/art/Artiest/Album/hoes.mp3?size=thumb",
            "de lijst hoort de verkleinde variant op te vragen"
        );
    }

    #[test]
    fn a_thumbnail_url_escapes_the_path() {
        assert_eq!(
            thumbnail_url("Sigur Rós/( )/01 intro.flac"),
            "/art/Sigur%20R%C3%B3s/(%20)/01%20intro.flac?size=thumb"
        );
    }

    #[test]
    fn issues_land_on_the_file_they_belong_to() {
        let (_tempdir, library) = library_with_album();
        place(&library, "compleet.mp3", testfixtures::MP3_WITH_ART);
        place(&library, "kaal.mp3", testfixtures::MP3_WITHOUT_TAGS);

        let listing = album_listing(&library, "");

        let complete = listing
            .tracks
            .iter()
            .find(|track| track.name == "compleet.mp3")
            .expect("het volledig getagde bestand moet er staan");
        assert!(
            complete.issues.is_empty(),
            "onterechte meldingen: {:?}",
            complete.issues
        );

        let bare = listing
            .tracks
            .iter()
            .find(|track| track.name == "kaal.mp3")
            .expect("het ongetagde bestand moet er staan");
        assert!(bare.issues.contains(&TrackIssue::MissingTitle));
        assert!(bare.issues.contains(&TrackIssue::MissingArt));
        assert!(bare.issues.contains(&TrackIssue::MissingTrackNumber));
    }

    #[test]
    fn folder_issues_describe_the_whole_directory() {
        let (_tempdir, library) = library_with_album();
        // Beide fixtures hebben tracknummer 3 en hetzelfde album, dus dit
        // levert een dubbel tracknummer op maar geen afwijkende albumtitel.
        place(&library, "een.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "twee.flac", testfixtures::FLAC_WITH_TAGS);

        let listing = album_listing(&library, "");

        assert!(
            listing
                .folder_issues
                .contains(&FolderIssue::DuplicateTrackNumbers(vec![3])),
            "gevonden: {:?}",
            listing.folder_issues
        );
    }

    #[test]
    fn folder_issues_survive_a_filter() {
        let (_tempdir, library) = library_with_album();
        place(&library, "een.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "twee.flac", testfixtures::FLAC_WITH_TAGS);

        let everything = album_listing(&library, "");
        let filtered = album_listing(&library, "een");

        assert_eq!(filtered.tracks.len(), 1, "het filter hoort te werken");
        assert_eq!(
            filtered.folder_issues, everything.folder_issues,
            "aan de map verandert niets doordat de gebruiker zoekt"
        );
    }

    #[test]
    fn a_tidy_directory_reports_nothing() {
        let (_tempdir, library) = library_with_album();
        place(&library, "hoes.mp3", testfixtures::MP3_WITH_ART);

        let listing = album_listing(&library, "");

        assert!(
            listing.folder_issues.is_empty(),
            "gevonden: {:?}",
            listing.folder_issues
        );
        assert!(
            listing.tracks[0].issues.is_empty(),
            "gevonden: {:?}",
            listing.tracks[0].issues
        );
    }

    #[test]
    fn refuses_a_path_outside_the_library() {
        let (_tempdir, library) = library_with_album();

        assert_eq!(
            listing(&library, "../..", "").unwrap_err(),
            PathError::OutsideLibrary
        );
    }

    #[test]
    fn durations_are_readable() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0:00");
        assert_eq!(format_duration(Duration::from_secs(9)), "0:09");
        assert_eq!(format_duration(Duration::from_secs(204)), "3:24");
        assert_eq!(format_duration(Duration::from_secs(3600)), "1:00:00");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1:02:05");
    }

    #[test]
    fn urls_escape_what_a_path_may_contain() {
        assert_eq!(url_for(""), "/");
        assert_eq!(url_for("Artiest/Album"), "/map/Artiest/Album");
        assert_eq!(
            url_for("Sigur Rós/( )"),
            "/map/Sigur%20R%C3%B3s/(%20)",
            "spaties en accenten horen gecodeerd te worden, de scheidende / niet"
        );
        assert_eq!(url_for("vraag? #1"), "/map/vraag%3F%20%231");
    }

    #[test]
    fn a_directory_with_thirty_tracks_stays_quick() {
        // De eis uit PRD §8.5 geldt op de NAS (task-27). Deze test bewaakt dat
        // de mapweergave lineair blijft in het aantal bestanden en niet stiekem
        // per bestand meerdere keren opent.
        let (_tempdir, library) = library_with_album();
        for number in 1..=30 {
            place(
                &library,
                &format!("track-{number:02}.mp3"),
                testfixtures::MP3_WITH_TAGS,
            );
        }

        let start = std::time::Instant::now();
        let listing = album_listing(&library, "");
        let elapsed = start.elapsed();

        assert_eq!(listing.tracks.len(), 30);
        assert!(
            elapsed < Duration::from_secs(1),
            "dertig tracks kostten {elapsed:?}"
        );
    }
}
