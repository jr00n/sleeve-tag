//! De albumweergave over HTTP, tegen de echte binary (FR-8).
//!
//! Wat hier getest wordt is het samenspel dat de unit-tests niet zien: een map
//! inlezen, de selectie uit de body halen, en er een pagina van maken waarop te
//! zien is wat er zou gebeuren. Dat laatste is in deze fase het enige resultaat:
//! er wordt nog niets geschreven, en een van de tests hieronder houdt dat vast.
//!
//! De bibliotheek is een tempdir met kopieën van de fixtures. De echte
//! muziekbibliotheek wordt nooit aangeraakt.

mod common;

use common::{Server, place_fixture};

/// Het album van de fixture met een volledige tagset.
const ALBUM_IN_FIXTURE: &str = "Fixtures voor Sleeve";

/// Bouwt een map met twee bestanden die niet dezelfde tags hebben.
///
/// `een.mp3` heeft de volledige tagset, `twee.mp3` helemaal niets. Daarmee
/// loopt elk gedeeld veld uiteen, en dat is precies de situatie waarvoor deze
/// pagina bestaat.
fn library_with_a_mixed_album() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "een.mp3", "tagged.mp3");
    place_fixture(&album, "twee.mp3", "untagged.mp3");

    root
}

fn server() -> Server {
    Server::start_in(library_with_a_mixed_album(), &[])
}

fn assert_ok(response: &str) {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
}

/// Of het vinkje van dit bestand aan staat.
///
/// De waarde en het `checked` staan in het template op dezelfde regel, juist
/// zodat een test er iets over kan zeggen.
fn is_ticked(page: &str, name: &str) -> bool {
    page.contains(&format!("value=\"{name}\" checked"))
}

#[test]
fn the_album_view_opens_with_everything_selected() {
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    assert!(page.contains("2 van 2 bestanden geselecteerd"), "{page}");
    assert!(is_ticked(&page, "een.mp3"), "{page}");
    assert!(is_ticked(&page, "twee.mp3"), "{page}");

    // Alle vijf de gedeelde velden uit FR-8 staan er.
    for label in ["Albumartiest", "Album", "Jaar", "Genre", "Discnummer"] {
        assert!(page.contains(label), "veld '{label}' ontbreekt:\n{page}");
    }
}

#[test]
fn the_directory_view_links_to_it() {
    let server = server();
    let page = server.get("/map/Album");

    assert_ok(&page);
    assert!(page.contains("/album/Album"), "{page}");
}

#[test]
fn a_single_file_can_be_deselected() {
    let server = server();
    let page = server.post_form("/album/Album", &[("bestand", "een.mp3")]);

    assert_ok(&page);
    assert!(page.contains("1 van 2 bestanden geselecteerd"), "{page}");
    assert!(is_ticked(&page, "een.mp3"), "{page}");
    assert!(!is_ticked(&page, "twee.mp3"), "{page}");
}

#[test]
fn select_all_and_select_nothing_are_actions_of_their_own() {
    let server = server();

    // Vanuit een selectie van één bestand terug naar alles.
    let all = server.post_form(
        "/album/Album",
        &[("actie", "alles"), ("bestand", "een.mp3")],
    );
    assert_ok(&all);
    assert!(all.contains("2 van 2 bestanden geselecteerd"), "{all}");

    let none = server.post_form(
        "/album/Album",
        &[
            ("actie", "niets"),
            ("bestand", "een.mp3"),
            ("bestand", "twee.mp3"),
        ],
    );
    assert_ok(&none);
    assert!(none.contains("0 van 2 bestanden geselecteerd"), "{none}");
    assert!(!is_ticked(&none, "een.mp3"), "{none}");
}

#[test]
fn the_selection_and_the_input_survive_each_other() {
    // AC #2: de selectie aanpassen mag de ingevulde velden niet wissen, en
    // andersom. Alles gaat in één formulier mee, dus dit is één verzoek.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("bestand", "een.mp3"),
            ("album_artist", "De Albumartiest"),
            ("album", "Een nieuw album"),
            ("year", "1999"),
            ("genre", "Ambient"),
            ("disc", "2"),
        ],
    );

    assert_ok(&page);
    assert!(page.contains("1 van 2 bestanden geselecteerd"), "{page}");
    assert!(is_ticked(&page, "een.mp3"), "{page}");
    assert!(page.contains("value=\"Een nieuw album\""), "{page}");
    assert!(page.contains("value=\"De Albumartiest\""), "{page}");
    assert!(page.contains("value=\"1999\""), "{page}");

    // En wat er zou gebeuren, staat erbij (AC #3).
    assert!(
        page.contains("Album wordt “Een nieuw album” in 1 bestand."),
        "{page}"
    );
    assert!(
        page.contains("Genre wordt “Ambient” in 1 bestand."),
        "{page}"
    );
}

#[test]
fn differing_values_are_visible_in_the_input() {
    // AC #5: een selectie waarin het album uiteenloopt, hoort dat te zeggen —
    // en niet één van de twee waarden als de waarheid te tonen.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    assert!(page.contains("Nu: verschillend"), "{page}");
    assert!(page.contains(ALBUM_IN_FIXTURE), "{page}");
    assert!(
        page.contains("Verschillend — leeg laten behoudt per bestand"),
        "{page}"
    );

    // Alleen het bestand met tags erin: dan is er niets verschillends meer.
    let one = server.post_form("/album/Album", &[("bestand", "een.mp3")]);
    assert!(
        one.contains(&format!("Nu: “{ALBUM_IN_FIXTURE}” in de hele selectie.")),
        "{one}"
    );
}

#[test]
fn leaving_a_field_empty_is_not_the_same_as_clearing_it() {
    // AC #4: het hele verschil waar deze pagina op staat of valt.
    let server = server();

    let untouched = server.post_form("/album/Album", &[("actie", "alles")]);
    assert_ok(&untouched);
    assert!(
        untouched.contains("Er is nog niets ingevuld"),
        "{untouched}"
    );

    let cleared = server.post_form("/album/Album", &[("actie", "alles"), ("wis_album", "aan")]);
    assert_ok(&cleared);
    assert!(
        cleared.contains("Album wordt verwijderd uit 2 bestanden."),
        "{cleared}"
    );
    // De andere velden blijven ongemoeid; wissen geldt per veld.
    assert!(cleared.contains("Genre blijft ongemoeid."), "{cleared}");
}

#[test]
fn nothing_is_written_yet() {
    // Het wegschrijven hoort bij de voorbeeldweergave. Zolang die er niet is,
    // mag geen enkele POST naar deze pagina een bestand aanraken.
    let root = library_with_a_mixed_album();
    let album = root.path().join("Album");
    let before: Vec<Vec<u8>> = ["een.mp3", "twee.mp3"]
        .iter()
        .map(|name| std::fs::read(album.join(name)).expect("bestand moet leesbaar zijn"))
        .collect();

    let server = Server::start_in(root, &[]);
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "alles"),
            ("album", "Een heel ander album"),
            ("wis_genre", "aan"),
        ],
    );
    assert_ok(&page);

    for (name, original) in ["een.mp3", "twee.mp3"].iter().zip(before) {
        let now = std::fs::read(album.join(name)).expect("bestand moet leesbaar zijn");
        assert_eq!(now, original, "{name} is aangeraakt en dat hoort niet");
    }
}

#[test]
fn a_disc_number_that_is_not_a_number_is_refused() {
    let server = server();
    let page = server.post_form("/album/Album", &[("actie", "alles"), ("disc", "twee")]);

    assert_ok(&page);
    assert!(page.contains("Discnummer moet een getal"), "{page}");
    assert!(page.contains("twee"), "{page}");
}

#[test]
fn htmx_gets_only_the_form_back() {
    // Met JavaScript wordt alleen het formulier vervangen; zonder JavaScript
    // moet dezelfde POST een bruikbare pagina opleveren.
    let server = server();

    let fragment = server.post_form_with_headers(
        "/album/Album",
        &[("bestand", "een.mp3")],
        &[("HX-Request", "true")],
    );
    assert_ok(&fragment);
    assert!(!fragment.contains("<!DOCTYPE html>"), "{fragment}");
    assert!(fragment.contains("id=\"album\""), "{fragment}");

    let whole = server.post_form("/album/Album", &[("bestand", "een.mp3")]);
    assert_ok(&whole);
    assert!(whole.contains("<!DOCTYPE html>"), "{whole}");
    assert!(whole.contains("id=\"album\""), "{whole}");
}

#[test]
fn a_directory_outside_the_library_is_refused() {
    let server = server();
    let response = server.get("/album/../../etc");

    assert!(
        response.starts_with("HTTP/1.1 403") || response.starts_with("HTTP/1.1 404"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
}
