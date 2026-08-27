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

use crate::fs::{DirEntry, Library, PathError};
use crate::tags;

/// Naam van de bibliotheekwortel in het broodkruimelpad.
///
/// De gebruiker hoeft niet te weten dat de map in de container `/music` heet.
const ROOT_NAME: &str = "Bibliotheek";

/// Wat er staat waar een tag ontbreekt.
const MISSING: &str = "—";

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

    pub track: Option<u32>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,

    /// Speelduur als `m:ss`, of `u:mm:ss` vanaf een uur.
    pub duration: String,

    /// `MP3` of `FLAC`.
    pub format: String,

    /// Of er een embedded hoes in het bestand zit; de thumbnail zelf volgt in
    /// een aparte taak.
    pub has_art: bool,
}

impl TrackSummary {
    /// Het tracknummer, of een lege tekst wanneer het ontbreekt.
    ///
    /// Bewust leeg en niet `—`: in een smalle kolom vóór de titel is een streepje
    /// per regel meer ruis dan informatie.
    pub fn track_label(&self) -> String {
        self.track
            .map(|number| number.to_string())
            .unwrap_or_default()
    }

    /// De titel, of een streepje wanneer die ontbreekt.
    ///
    /// De bestandsnaam invullen zou vriendelijk lijken, maar verbergt precies
    /// wat de gebruiker moet zien: hier staat geen titel in het bestand.
    pub fn title_label(&self) -> &str {
        self.title.as_deref().unwrap_or(MISSING)
    }

    pub fn artist_label(&self) -> &str {
        self.artist.as_deref().unwrap_or(MISSING)
    }

    pub fn album_label(&self) -> &str {
        self.album.as_deref().unwrap_or(MISSING)
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

    /// Van de bibliotheekwortel tot en met deze map.
    pub crumbs: Vec<Crumb>,

    pub folders: Vec<Folder>,
    pub tracks: Vec<TrackSummary>,

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

    let mut tracks: Vec<TrackSummary> = contents
        .files
        .iter()
        .filter_map(|entry| summarize(entry, &path))
        .filter(|track| matches_query(track, &needle))
        .collect();

    sort_tracks(&mut tracks);

    Ok(Listing {
        name: name_of(&path),
        crumbs: crumbs_for(&path),
        url: url_for(&path),
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

    Some(TrackSummary {
        path: join(directory, &entry.name),
        name: entry.name.clone(),
        track: track.tags.track,
        title: track.tags.title,
        artist: track.tags.artist,
        album: track.tags.album,
        duration: format_duration(track.duration),
        format: track.format.to_string(),
        has_art: track.art.is_some(),
    })
}

/// Sorteert op tracknummer, met de bestandsnaam als terugval.
///
/// Antwoord op het open punt in PRD §12: het tracknummer uit de tags bepaalt de
/// volgorde, want dat is de volgorde waarin het album bedoeld is. Bestanden
/// zonder tracknummer kunnen daar niet tussen worden geplaatst en komen
/// erachter, onderling op naam.
fn sort_tracks(tracks: &mut [TrackSummary]) {
    tracks.sort_by(|a, b| {
        a.track
            .unwrap_or(u32::MAX)
            .cmp(&b.track.unwrap_or(u32::MAX))
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
            .title
            .as_deref()
            .is_some_and(|title| title.to_lowercase().contains(needle))
}

/// Zet een speelduur om naar `m:ss`, of `u:mm:ss` vanaf een uur.
fn format_duration(duration: Duration) -> String {
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
fn url_for(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/map/{}", utf8_percent_encode(path, PATH_ESCAPES))
    }
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
        assert!(track.track.is_some(), "tracknummer ontbreekt");
        assert!(track.title.is_some(), "titel ontbreekt");
        assert!(track.artist.is_some(), "artiest ontbreekt");
        assert!(track.album.is_some(), "album ontbreekt");
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
                .find_map(|track| track.title.clone())
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
        assert!(!track.has_art);
    }

    #[test]
    fn reports_embedded_art() {
        let (_tempdir, library) = library_with_album();
        place(&library, "hoes.mp3", testfixtures::MP3_WITH_ART);

        let listing = album_listing(&library, "");
        assert!(listing.tracks[0].has_art, "de fixture heeft een hoes");
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
