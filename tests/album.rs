//! De albumweergave over HTTP, tegen de echte binary (FR-8).
//!
//! Wat hier getest wordt is het samenspel dat de unit-tests niet zien: een map
//! inlezen, de selectie uit de body halen, er een pagina van maken waarop te
//! zien is wat er zou gebeuren, en dat daarna werkelijk wegschrijven. Alleen de
//! route langs de voorbeeldweergave schrijft; de tests hieronder houden vast
//! dat elke andere POST de bestanden byte voor byte met rust laat.
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

    // Alle gedeelde velden uit FR-8 staan er.
    for label in [
        "Albumartiest",
        "Album",
        "Jaar",
        "Genre",
        "Discnummer",
        "Aantal discs",
    ] {
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
    for action in [
        "hernummer",
        "hernummer-disc",
        "disc",
        "disctotaal",
        "titelnaam",
        "artiest",
        "hoofdletters",
        "herstel",
    ] {
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
fn only_the_preview_route_writes() {
    // Het wegschrijven hoort bij de voorbeeldweergave. Elke andere POST naar
    // deze pagina — een selectie, een hulpactie, een ingevuld veld — laat de
    // bestanden met rust.
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
    for action in [
        "hernummer",
        "hernummer-disc",
        "disc",
        "disctotaal",
        "titelnaam",
        "artiest",
        "hoofdletters",
        "herstel",
    ] {
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

/// Een map met bestanden zonder tags, maar met sprekende namen.
///
/// Daarmee is te zien wat de vier hulpacties rond schijven en bestandsnamen
/// doen: er staat nog nergens een discnummer, en de titel staat alleen nog in
/// de naam.
fn library_with_untagged_names() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "01-Eerste_stuk.mp3", "untagged.mp3");
    place_fixture(&album, "02-Tweede_stuk.mp3", "untagged.mp3");

    root
}

#[test]
fn a_title_can_be_read_from_the_file_name() {
    // AC #4: zonder titeltag is de bestandsnaam de enige plek waar hij staat.
    let server = Server::start_in(library_with_untagged_names(), &[]);
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "titelnaam"),
            ("bestand", "01-Eerste_stuk.mp3"),
            ("bestand", "02-Tweede_stuk.mp3"),
        ],
    );

    assert_ok(&page);
    assert!(
        is_proposed(&page, "titel:01-Eerste_stuk.mp3", "Eerste stuk"),
        "{page}"
    );
    assert!(
        is_proposed(&page, "titel:02-Tweede_stuk.mp3", "Tweede stuk"),
        "{page}"
    );
    assert!(page.contains("Er is niets opgeslagen"), "{page}");
}

#[test]
fn a_disc_number_and_a_disc_total_are_one_click_away() {
    // AC #2 en #3: het nummer staat vooraf op de knop, en het totaal geldt voor
    // de hele map.
    let server = Server::start_in(library_with_untagged_names(), &[]);

    let page = server.get("/album/Album");
    assert_ok(&page);
    // Er is nog geen schijf in gebruik, dus de eerstvolgende vrije is 1.
    assert!(page.contains("Deze schijf nummer 1 geven"), "{page}");

    let numbered = server.post_form(
        "/album/Album",
        &[("actie", "disc"), ("bestand", "01-Eerste_stuk.mp3")],
    );
    assert_ok(&numbered);
    assert!(
        numbered.contains("Schijf 1 staat als voorstel"),
        "{numbered}"
    );
    assert!(
        numbered.contains("Discnummer wordt “1” in 1 bestand."),
        "{numbered}"
    );

    let totals = server.post_form(
        "/album/Album",
        &[("actie", "disctotaal"), ("bestand", "01-Eerste_stuk.mp3")],
    );
    assert_ok(&totals);
    assert!(
        totals.contains("Aantal discs wordt “1” in 2 bestanden."),
        "{totals}"
    );
    // De actie geldt voor de hele map, dus alles staat aangevinkt.
    assert!(
        totals.contains("2 van 2 bestanden geselecteerd"),
        "{totals}"
    );
}

/// De velden die samen een batch beschrijven, klaar om te posten.
fn a_batch() -> Vec<(&'static str, &'static str)> {
    vec![
        ("bestand", "een.mp3"),
        ("bestand", "twee.mp3"),
        ("album", "Een heel ander album"),
        ("wis_genre", "aan"),
        ("titel:twee.mp3", "Ruis in B"),
    ]
}

#[test]
fn the_preview_shows_per_file_what_changes() {
    // AC #1 en #2: oude en nieuwe waarde per veld, en verwijderingen als
    // zodanig.
    let server = server();
    let mut fields = a_batch();
    fields.push(("actie", "voorbeeld"));

    let page = server.post_form("/album/Album", &fields);

    assert_ok(&page);
    assert!(page.contains("Voorbeeld"), "{page}");
    assert!(page.contains("2 bestanden worden gewijzigd."), "{page}");

    // De oude waarde staat erbij, niet alleen de nieuwe.
    assert!(page.contains(ALBUM_IN_FIXTURE), "{page}");
    assert!(page.contains("Een heel ander album"), "{page}");
    assert!(page.contains("Ruis in B"), "{page}");

    // AC #2: het genre verdwijnt uit het bestand dat er een had.
    assert!(page.contains("wordt verwijderd"), "{page}");

    // En de knop om het te doen staat er pas hier.
    assert!(page.contains("value=\"opslaan\""), "{page}");
}

#[test]
fn a_file_without_changes_is_shown_as_such() {
    // AC #3: dat er níéts met een bestand gebeurt, is de helft van wat een
    // voorbeeld moet vertellen.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "voorbeeld"),
            ("bestand", "een.mp3"),
            ("bestand", "twee.mp3"),
            ("album", ALBUM_IN_FIXTURE),
        ],
    );

    assert_ok(&page);
    // een.mp3 heeft dit album al.
    assert!(page.contains("Blijft ongewijzigd"), "{page}");
    assert!(page.contains("1 bestand wordt gewijzigd."), "{page}");
}

#[test]
fn the_preview_writes_nothing_and_can_be_cancelled() {
    // AC #6: annuleren brengt het formulier terug, en er is niets geschreven.
    let root = library_with_a_mixed_album();
    let album = root.path().join("Album");
    let before = std::fs::read(album.join("een.mp3")).expect("bestand moet leesbaar zijn");

    let server = Server::start_in(root, &[]);
    let mut fields = a_batch();
    fields.push(("actie", "voorbeeld"));

    let preview = server.post_form("/album/Album", &fields);
    assert_ok(&preview);

    let mut cancelled = a_batch();
    cancelled.push(("actie", "terug"));
    let back = server.post_form("/album/Album", &cancelled);

    assert_ok(&back);
    // Terug in het formulier: de tabel en de gedeelde velden staan er weer.
    assert!(back.contains("Gedeelde velden"), "{back}");
    assert!(back.contains("value=\"Een heel ander album\""), "{back}");
    assert!(!back.contains("Definitief opslaan"), "{back}");

    let after = std::fs::read(album.join("een.mp3")).expect("bestand moet leesbaar zijn");
    assert_eq!(after, before, "er is geschreven en dat hoort niet");
}

#[test]
fn saving_the_batch_writes_the_files_and_reports_per_file() {
    // AC #5: na afloop staat er per bestand wat ermee gebeurd is, en de tabel
    // toont wat er werkelijk in de bestanden staat.
    let server = server();
    let mut fields = a_batch();
    fields.push(("actie", "opslaan"));

    let page = server.post_form("/album/Album", &fields);

    assert_ok(&page);
    assert!(page.contains("2 bestanden bijgewerkt."), "{page}");
    assert!(page.contains("Bijgewerkt: Album"), "{page}");

    // De invoer is verwerkt en staat niet meer voorgevuld klaar.
    assert!(!page.contains("value=\"Een heel ander album\""), "{page}");

    // Een verse leesronde toont de nieuwe waarden.
    let fresh = server.get("/album/Album");
    assert!(fresh.contains("Een heel ander album"), "{fresh}");
    assert!(fresh.contains("placeholder=\"Ruis in B\""), "{fresh}");
    // Het genre is werkelijk verdwenen.
    assert!(!fresh.contains("Ambient"), "{fresh}");
}

#[test]
fn a_file_that_cannot_be_written_does_not_stop_the_others() {
    // AC #4 en #7: bestand voor bestand, en een fout bij het ene bestand
    // blokkeert het andere niet.
    use std::os::unix::fs::PermissionsExt;

    let root = library_with_a_mixed_album();
    let album = root.path().join("Album");
    let locked = album.join("twee.mp3");

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o444))
        .expect("rechten moeten te zetten zijn");

    // Als dit proces als root draait, gelden bestandsrechten niet en valt de
    // situatie niet na te bootsen; de test heeft dan niets te zeggen.
    if std::fs::OpenOptions::new()
        .write(true)
        .open(&locked)
        .is_ok()
    {
        eprintln!("overgeslagen: dit proces mag ook in een alleen-lezen bestand schrijven");
        return;
    }

    let untouched = std::fs::read(&locked).expect("bestand moet leesbaar zijn");

    let server = Server::start_in(root, &[]);
    let mut fields = a_batch();
    fields.push(("actie", "opslaan"));

    let page = server.post_form("/album/Album", &fields);

    assert_ok(&page);
    assert!(
        page.contains("1 bestand bijgewerkt; 1 bestand is niet opgeslagen."),
        "{page}"
    );
    assert!(page.contains("Niet opgeslagen"), "{page}");
    assert!(page.contains("onveranderd gebleven"), "{page}");

    // Het bestand dat wél kon, is bijgewerkt ...
    let fresh = server.get("/album/Album");
    assert!(fresh.contains("Een heel ander album"), "{fresh}");

    // ... en het bestand dat niet kon, is byte voor byte heel gebleven.
    let after = std::fs::read(&locked).expect("bestand moet leesbaar zijn");
    assert_eq!(after, untouched, "het onschrijfbare bestand is aangeraakt");
}

#[test]
fn a_batch_with_a_mistake_in_it_is_not_written_at_all() {
    let root = library_with_a_mixed_album();
    let album = root.path().join("Album");
    let before = std::fs::read(album.join("een.mp3")).expect("bestand moet leesbaar zijn");

    let server = Server::start_in(root, &[]);
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "opslaan"),
            ("bestand", "een.mp3"),
            ("album", "Een heel ander album"),
            ("disc", "twee"),
        ],
    );

    assert_ok(&page);
    assert!(page.contains("Discnummer moet een getal"), "{page}");
    assert!(page.contains("wordt er niets opgeslagen"), "{page}");

    let after = std::fs::read(album.join("een.mp3")).expect("bestand moet leesbaar zijn");
    assert_eq!(after, before, "er is geschreven en dat hoort niet");
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

/// De bytes van een fixture-afbeelding.
fn cover_bytes(name: &str) -> Vec<u8> {
    std::fs::read(common::fixture_path(name)).expect("fixture moet leesbaar zijn")
}

/// Of dit bestand een embedded hoes heeft, gevraagd via de app zelf.
fn has_cover(server: &Server, file: &str) -> bool {
    let response = server.get_bytes(&format!("/art/Album/{file}"));
    String::from_utf8_lossy(&response[..response.len().min(20)]).contains("200 OK")
}

#[test]
fn the_preview_offers_a_cover_for_the_selection() {
    // AC #1 en #2: de hoes hoort bij de selectie, en per bestand staat er wat
    // hij zou doen — toevoegen of vervangen.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "voorbeeld"),
            ("bestand", "een.mp3"),
            ("album", "Een heel ander album"),
        ],
    );

    assert_ok(&page);

    // Het vak hoort er te staan, met het aantal erbij en de uploadgrens.
    assert!(page.contains("Hoes voor deze 1 bestanden"), "{page}");
    assert!(page.contains("data-neerzetvak"), "{page}");
    assert!(page.contains(r#"name="afbeelding""#), "{page}");

    // een.mp3 heeft geen hoes, dus toevoegen. twee.mp3 staat niet aangevinkt en
    // hoort dus ook geen hoesregel te krijgen.
    assert!(page.contains("hoes wordt toegevoegd"), "{page}");

    let na_twee = page
        .find("twee.mp3")
        .map(|positie| page[positie..].to_string())
        .expect("twee.mp3 hoort in het voorbeeld te staan");
    assert!(
        !na_twee.contains("hoes wordt"),
        "een niet-aangevinkt bestand hoort geen hoesregel te krijgen:\n{na_twee}"
    );

    // De multipart-vorm is nodig om de afbeelding mee te kunnen sturen.
    assert!(page.contains(r#"enctype="multipart/form-data""#), "{page}");
}

#[test]
fn a_cover_lands_in_the_selected_files_only() {
    // AC #4 en #5: de aangevinkte bestanden krijgen de hoes, de rest blijft
    // onaangeraakt — ook al staan ze in dezelfde map.
    let root = library_with_a_mixed_album();
    let untouched = root.path().join("Album").join("twee.mp3");
    let before = std::fs::read(&untouched).expect("fixture moet leesbaar zijn");

    let server = Server::start_in(root, &[]);

    let page = server.post_multipart(
        "/album/Album",
        &[("actie", "opslaan"), ("bestand", "een.mp3")],
        Some(("afbeelding", "cover.jpg", &cover_bytes("cover.jpg"))),
    );

    assert_ok(&page);
    assert!(
        page.contains("Hoes"),
        "het rapport noemt de hoes niet:\n{page}"
    );

    assert!(
        has_cover(&server, "een.mp3"),
        "het aangevinkte bestand heeft geen hoes gekregen"
    );
    assert_eq!(
        std::fs::read(&untouched).expect("lezen"),
        before,
        "een bestand dat niet aangevinkt stond, is toch aangeraakt"
    );
}

#[test]
fn a_cover_can_go_with_the_tag_changes_in_one_go() {
    // Eén ronde, één rapport: de tags en de hoes staan samen bij het bestand.
    let server = server();

    let page = server.post_multipart(
        "/album/Album",
        &[
            ("actie", "opslaan"),
            ("bestand", "een.mp3"),
            ("album", "Een heel ander album"),
        ],
        Some(("afbeelding", "cover.jpg", &cover_bytes("cover.jpg"))),
    );

    assert_ok(&page);
    assert!(page.contains("Album"), "{page}");
    assert!(page.contains("Hoes"), "{page}");
    assert!(has_cover(&server, "een.mp3"));
}

#[test]
fn the_folder_cover_can_be_written_along() {
    // AC #6: dezelfde keuze als op de hoespagina, met dezelfde standaard.
    let root = library_with_a_mixed_album();
    let album = root.path().join("Album");
    let server = Server::start_in(root, &[]);

    // Zonder het vinkje komt er niets in de map.
    server.post_multipart(
        "/album/Album",
        &[("actie", "opslaan"), ("bestand", "een.mp3")],
        Some(("afbeelding", "cover.jpg", &cover_bytes("cover.jpg"))),
    );
    assert!(
        !album.join("cover.jpg").exists(),
        "er is een bestand aangemaakt dat niemand vroeg"
    );

    // Met het vinkje wel.
    let page = server.post_multipart(
        "/album/Album",
        &[
            ("actie", "opslaan"),
            ("bestand", "twee.mp3"),
            ("mapbestand", "ja"),
        ],
        Some(("afbeelding", "cover.jpg", &cover_bytes("cover.jpg"))),
    );

    assert_ok(&page);
    assert!(
        album.join("cover.jpg").exists(),
        "de losse hoes ontbreekt:\n{page}"
    );
}

#[test]
fn a_batch_without_a_cover_behaves_exactly_as_before() {
    // De hoes is een toevoeging: laat je het veld leeg, dan gebeurt er precies
    // wat er in het voorbeeld stond en niets meer.
    let root = library_with_a_mixed_album();
    let track = root.path().join("Album").join("een.mp3");
    let server = Server::start_in(root, &[]);

    let page = server.post_multipart(
        "/album/Album",
        &[
            ("actie", "opslaan"),
            ("bestand", "een.mp3"),
            ("album", "Een heel ander album"),
        ],
        None,
    );

    assert_ok(&page);
    assert!(
        !has_cover(&server, "een.mp3"),
        "er is een hoes verschenen die niemand heeft meegestuurd"
    );

    // En de tagwijziging is er wel gewoon doorheen gekomen.
    let bytes = std::fs::read(&track).expect("lezen");
    let inhoud = String::from_utf8_lossy(&bytes);
    assert!(
        inhoud.contains("Een heel ander album"),
        "de tagwijziging ontbreekt"
    );
}
