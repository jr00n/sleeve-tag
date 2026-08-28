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
fn every_row_offers_a_title_and_a_track_number() {
    // AC #1: titel en tracknummer horen inline in de tabel te staan, en niet in
    // een apart formulier per bestand.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    for name in ["een.mp3", "twee.mp3"] {
        assert!(page.contains(&format!("name=\"titel:{name}\"")), "{page}");
        assert!(page.contains(&format!("name=\"nummer:{name}\"")), "{page}");
    }

    // Wat er nu in het bestand staat, staat als grijze tekst in het veld en
    // niet als waarde: leeg laten verandert er niets aan.
    assert!(page.contains("placeholder=\"Stilte in D\""), "{page}");
    assert!(!page.contains("value=\"Stilte in D\""), "{page}");
}

#[test]
fn shared_fields_and_overrides_go_together() {
    // AC #2, #3 en #6: de gedeelde velden gelden voor de selectie, de override
    // voor dat ene bestand, en het een wist het ander niet.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("bestand", "een.mp3"),
            ("bestand", "twee.mp3"),
            ("album", "Een nieuw album"),
            ("titel:twee.mp3", "Ruis in B"),
            ("nummer:twee.mp3", "2"),
        ],
    );

    assert_ok(&page);
    assert!(page.contains("2 van 2 bestanden geselecteerd"), "{page}");
    assert!(page.contains("value=\"Een nieuw album\""), "{page}");
    assert!(page.contains("value=\"Ruis in B\""), "{page}");
    assert!(
        page.contains("Album wordt “Een nieuw album” in 2 bestanden."),
        "{page}"
    );
    assert!(
        page.contains("1 bestand krijgt een eigen waarde uit de tabel."),
        "{page}"
    );
}

#[test]
fn an_override_survives_a_change_of_selection() {
    // AC #2: het uitvinken van een ander bestand mag de tabel niet leegvegen.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "niets"),
            ("titel:een.mp3", "Blijft staan"),
            ("album", "Blijft ook staan"),
        ],
    );

    assert_ok(&page);
    assert!(page.contains("0 van 2 bestanden geselecteerd"), "{page}");
    assert!(page.contains("value=\"Blijft staan\""), "{page}");
    assert!(page.contains("value=\"Blijft ook staan\""), "{page}");
    assert!(
        page.contains("Niet geselecteerd; deze invoer wordt niet opgeslagen."),
        "{page}"
    );
}

#[test]
fn a_bad_track_number_is_reported_at_the_row_it_was_typed_in() {
    // AC #4: de melding staat bij de rij, en de rest blijft bruikbaar.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "alles"),
            ("nummer:een.mp3", "drie"),
            ("titel:twee.mp3", "Wel goed"),
        ],
    );

    assert_ok(&page);
    assert!(page.contains("Tracknummer moet een getal"), "{page}");
    assert!(page.contains("drie"), "{page}");
    assert!(page.contains("rijveld__fout"), "{page}");
    assert!(page.contains("de melding staat bij de rij zelf"), "{page}");

    // De goede rij telt gewoon mee.
    assert!(
        page.contains("1 bestand krijgt een eigen waarde uit de tabel."),
        "{page}"
    );
}

/// Of dit invoerveld deze waarde als voorstel draagt.
///
/// Naam en waarde staan in het template op dezelfde regel, juist zodat een test
/// er iets over kan zeggen.
fn is_proposed(page: &str, field: &str, value: &str) -> bool {
    page.contains(&format!("name=\"{field}\" value=\"{value}\""))
}

#[test]
fn the_helper_actions_are_offered_on_the_page() {
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    for action in ["hernummer", "artiest", "hoofdletters", "herstel"] {
        assert!(
            page.contains(&format!("name=\"actie\" value=\"{action}\"")),
            "hulpactie '{action}' ontbreekt:\n{page}"
        );
    }
}

#[test]
fn renumbering_fills_the_track_column() {
    // AC #1: opeenvolgend nummeren volgens de sortering van de tabel.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "hernummer"),
            ("bestand", "een.mp3"),
            ("bestand", "twee.mp3"),
        ],
    );

    assert_ok(&page);
    assert!(is_proposed(&page, "nummer:een.mp3", "1"), "{page}");
    assert!(is_proposed(&page, "nummer:twee.mp3", "2"), "{page}");
    assert!(page.contains("1 tot en met 2"), "{page}");
}

#[test]
fn copying_the_artist_fills_the_album_artist_column() {
    // AC #2: per bestand, en alleen waar er een artiest te kopiëren valt.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "artiest"),
            ("bestand", "een.mp3"),
            ("bestand", "twee.mp3"),
        ],
    );

    assert_ok(&page);
    assert!(
        is_proposed(&page, "albumartiest:een.mp3", "De Testartiest"),
        "{page}"
    );
    assert!(is_proposed(&page, "albumartiest:twee.mp3", ""), "{page}");
    assert!(page.contains("1 zonder artiest"), "{page}");
}

#[test]
fn normalising_capitals_is_a_proposal_and_can_be_emptied_again() {
    // AC #3 en #5: het voorstel staat in de velden, en de herstelknop haalt het
    // er weer uit zonder de selectie kwijt te raken.
    let server = server();
    let proposed = server.post_form(
        "/album/Album",
        &[
            ("actie", "hoofdletters"),
            ("bestand", "een.mp3"),
            ("titel:een.mp3", "STILTE IN D MAJEUR"),
        ],
    );

    assert_ok(&proposed);
    assert!(
        is_proposed(&proposed, "titel:een.mp3", "Stilte in D Majeur"),
        "{proposed}"
    );
    assert!(proposed.contains("Er is niets opgeslagen"), "{proposed}");

    let restored = server.post_form(
        "/album/Album",
        &[
            ("actie", "herstel"),
            ("bestand", "een.mp3"),
            ("titel:een.mp3", "Stilte in D Majeur"),
        ],
    );

    assert_ok(&restored);
    assert!(is_proposed(&restored, "titel:een.mp3", ""), "{restored}");
    assert!(
        restored.contains("1 van 2 bestanden geselecteerd"),
        "{restored}"
    );
    assert!(restored.contains("Er is nog niets ingevuld"), "{restored}");
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
            ("titel:een.mp3", "Een heel andere titel"),
            ("nummer:twee.mp3", "9"),
        ],
    );
    assert_ok(&page);

    // AC #4: ook een hulpactie vult alleen de invoervelden.
    for action in ["hernummer", "artiest", "hoofdletters", "herstel"] {
        let page = server.post_form(
            "/album/Album",
            &[
                ("actie", action),
                ("bestand", "een.mp3"),
                ("bestand", "twee.mp3"),
            ],
        );
        assert_ok(&page);
    }

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
