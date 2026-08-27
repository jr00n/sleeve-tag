//! De mapbrowser over HTTP, tegen de echte binary (FR-1 t/m FR-3).
//!
//! De unit-tests in `src/browse.rs` controleren het weergavemodel; deze test
//! controleert wat er werkelijk in de browser terechtkomt: de velden in de HTML,
//! de volgorde van de regels en het filter uit de querystring.
//!
//! De bibliotheek is een tempdir met kopieën van de fixtures. De echte
//! muziekbibliotheek wordt nooit aangeraakt.

mod common;

use std::path::PathBuf;

use common::{Server, place_fixture};

/// De waarden die in `tests/fixtures/genereer-fixtures.sh` in de getagde
/// fixtures zijn gezet.
const TITLE: &str = "Stilte in D";
const ARTIST: &str = "De Testartiest";
const ALBUM: &str = "Fixtures voor Sleeve";
const TRACK_NUMBER: &str = "3";

/// Bouwt een bibliotheek met één album en geeft de tempdir terug.
///
/// De bestandsnamen staan bewust dwars op de tracknummers: `zzz-getagd.mp3`
/// heeft tracknummer 3 en hoort daarom vóór de twee bestanden zonder
/// tracknummer te staan, ook al sorteert zijn naam als laatste.
fn library_with_album() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Artiest").join("Het Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "zzz-getagd.mp3", "tagged.mp3");
    place_fixture(&album, "aaa-kaal.flac", "untagged.flac");
    place_fixture(&album, "mmm-kaal.mp3", "untagged.mp3");

    // Niet-audio en een bestand dat alleen zo heet: geen van beide is een
    // bewerkbaar bestand en geen van beide hoort in de lijst.
    place_fixture(&album, "folder.jpg", "cover.jpg");
    place_fixture(&album, "nep.mp3", "cover.png");
    std::fs::write(album.join("notities.txt"), b"tekst").expect("bestand moet te schrijven zijn");

    root
}

/// Alleen de body van een respons, zodat headers geen assertie kunnen redden.
fn body(response: &str) -> String {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_else(|| panic!("respons had geen body:\n{response}"))
}

/// De positie van een tekst in de pagina; paniekt als hij ontbreekt.
fn position(html: &str, needle: &str) -> usize {
    html.find(needle)
        .unwrap_or_else(|| panic!("'{needle}' staat niet op de pagina:\n{html}"))
}

fn assert_ok(response: &str) {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
}

#[test]
fn a_directory_shows_its_files_with_the_main_tags() {
    let server = Server::start_in(library_with_album(), &[]);
    let response = server.get("/map/Artiest/Het%20Album");
    assert_ok(&response);

    let html = body(&response);

    // FR-2: tracknummer, titel, artiest, album, duur en formaat.
    for expected in [
        TRACK_NUMBER,
        TITLE,
        ARTIST,
        ALBUM,
        "0:01", // de fixtures zijn één seconde stilte
        "MP3",
        "FLAC",
    ] {
        assert!(
            html.contains(expected),
            "'{expected}' ontbreekt op de pagina:\n{html}"
        );
    }

    // De drie audiobestanden staan er; de rest niet.
    for name in ["zzz-getagd.mp3", "aaa-kaal.flac", "mmm-kaal.mp3"] {
        assert!(html.contains(name), "'{name}' ontbreekt:\n{html}");
    }
    for name in ["folder.jpg", "notities.txt", "nep.mp3"] {
        assert!(
            !html.contains(name),
            "'{name}' is geen bewerkbaar bestand en hoort niet in de lijst:\n{html}"
        );
    }
}

#[test]
fn files_are_sorted_by_track_number_then_by_filename() {
    let server = Server::start_in(library_with_album(), &[]);
    let html = body(&server.get("/map/Artiest/Het%20Album"));

    let tagged = position(&html, "zzz-getagd.mp3");
    let first_untagged = position(&html, "aaa-kaal.flac");
    let second_untagged = position(&html, "mmm-kaal.mp3");

    assert!(
        tagged < first_untagged,
        "een bestand mét tracknummer hoort vooraan te staan:\n{html}"
    );
    assert!(
        first_untagged < second_untagged,
        "bestanden zonder tracknummer horen op naam te staan:\n{html}"
    );
}

#[test]
fn the_root_lists_the_top_level_directories() {
    let server = Server::start_in(library_with_album(), &[]);
    let html = body(&server.get("/"));

    assert!(html.contains("Artiest"), "de artiestmap ontbreekt:\n{html}");
    assert!(
        html.contains("/map/Artiest"),
        "er is geen link naar de artiestmap:\n{html}"
    );
}

#[test]
fn breadcrumbs_lead_back_to_the_root() {
    let server = Server::start_in(library_with_album(), &[]);
    let html = body(&server.get("/map/Artiest/Het%20Album"));

    assert!(
        html.contains("Bibliotheek"),
        "het broodkruimelpad begint niet bij de bibliotheek:\n{html}"
    );
    assert!(
        html.contains(r#"href="/map/Artiest""#),
        "er is geen weg terug naar de bovenliggende map:\n{html}"
    );
}

#[test]
fn filtering_works_on_the_title_and_on_the_filename() {
    let server = Server::start_in(library_with_album(), &[]);

    // Op titel: de zoekterm komt niet in de bestandsnaam voor.
    let by_title = body(&server.get("/map/Artiest/Het%20Album?q=stilte"));
    assert!(
        by_title.contains("zzz-getagd.mp3"),
        "filteren op titel vond het bestand niet:\n{by_title}"
    );
    assert!(
        !by_title.contains("aaa-kaal.flac"),
        "een bestand zonder overeenkomst bleef staan:\n{by_title}"
    );

    // Op bestandsnaam: de zoekterm komt in geen enkele tag voor.
    let by_name = body(&server.get("/map/Artiest/Het%20Album?q=aaa"));
    assert!(
        by_name.contains("aaa-kaal.flac"),
        "filteren op bestandsnaam vond het bestand niet:\n{by_name}"
    );
    assert!(
        !by_name.contains("zzz-getagd.mp3"),
        "een bestand zonder overeenkomst bleef staan:\n{by_name}"
    );

    // Niets gevonden is een lege lijst met uitleg, geen fout.
    let response = server.get("/map/Artiest/Het%20Album?q=bestaatniet");
    assert_ok(&response);
    assert!(
        body(&response).contains("Niets in deze map"),
        "een leeg resultaat hoort uitgelegd te worden:\n{response}"
    );
}

#[test]
fn htmx_receives_only_the_list() {
    let server = Server::start_in(library_with_album(), &[]);

    // Het fragment dat HTMX terugkrijgt is dezelfde lijst zonder de omhullende
    // pagina; anders zou de pagina zichzelf in zichzelf nesten.
    let fragment = server.get_with_headers("/map/Artiest/Het%20Album", &[("HX-Request", "true")]);
    assert_ok(&fragment);

    let html = body(&fragment);
    assert!(
        html.contains("zzz-getagd.mp3"),
        "de lijst ontbreekt:\n{html}"
    );
    assert!(
        !html.contains("<!DOCTYPE html>"),
        "HTMX kreeg de hele pagina in plaats van alleen de lijst:\n{html}"
    );
}

#[test]
fn navigating_above_the_root_is_refused() {
    let server = Server::start_in(library_with_album(), &[]);

    for attempt in [
        "/map/../../etc/passwd",
        "/map/Artiest/../../../etc/hosts",
        "/map/..",
    ] {
        let response = server.get(attempt);
        assert!(
            !response.starts_with("HTTP/1.1 200 OK"),
            "'{attempt}' leverde een pagina op:\n{response}"
        );
    }

    // Een map die niet bestaat is een dode link, geen geweigerd verzoek.
    let missing = server.get("/map/Artiest/Bestaat%20Niet");
    assert!(
        missing.starts_with("HTTP/1.1 404"),
        "antwoord begon met: {}",
        missing.lines().next().unwrap_or_default()
    );
}

#[test]
fn a_directory_with_thirty_tracks_renders_quickly() {
    // PRD §8.5 eist < 1 s voor dertig tracks. De maatgevende meting gebeurt op
    // de NAS (task-27); deze test bewaakt dat er per bestand niet meer werk
    // wordt gedaan dan één keer openen en lezen.
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album: PathBuf = root.path().join("Groot album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    for number in 1..=30 {
        place_fixture(&album, &format!("track-{number:02}.mp3"), "tagged.mp3");
    }

    let server = Server::start_in(root, &[]);

    let start = std::time::Instant::now();
    let response = server.get("/map/Groot%20album");
    let elapsed = start.elapsed();

    assert_ok(&response);
    let html = body(&response);
    for number in 1..=30 {
        let name = format!("track-{number:02}.mp3");
        assert!(html.contains(&name), "'{name}' ontbreekt in de lijst");
    }

    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "dertig tracks kostten {elapsed:?}"
    );
}

#[test]
fn the_page_stays_within_the_library() {
    // Wat er ook op de pagina staat, het absolute pad van de NAS hoort er niet
    // bij te staan: de gebruiker werkt met paden relatief aan MUSIC_ROOT.
    let root = library_with_album();
    let absolute = std::fs::canonicalize(root.path())
        .expect("root moet te canonicaliseren zijn")
        .display()
        .to_string();

    let server = Server::start_in(root, &[]);
    let html = body(&server.get("/map/Artiest/Het%20Album"));

    assert!(
        !html.contains(&absolute),
        "het absolute pad van de bibliotheek staat op de pagina:\n{html}"
    );
}
