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

/// De tabelrij van dit bestand, zodat een test iets over één rij kan zeggen.
///
/// De bestandsnaam staat in het selectievinkje van de rij, en dat vinkje staat
/// er maar één keer per rij in.
fn row_html<'a>(page: &'a str, name: &str) -> &'a str {
    page.split("<tr")
        .find(|chunk| chunk.contains(&format!("value=\"{name}\"")))
        .unwrap_or_else(|| panic!("de rij van {name} hoort in de tabel te staan:\n{page}"))
}

/// Wat de voorbeeldweergave over dit ene bestand zegt.
fn preview_html<'a>(page: &'a str, name: &str) -> &'a str {
    page.split("voorbeeld__bestand")
        .find(|chunk| chunk.contains(&format!(">{name}<")))
        .unwrap_or_else(|| panic!("{name} hoort in het voorbeeld te staan:\n{page}"))
}

#[test]
fn a_row_shows_the_file_the_way_the_design_shows_it() {
    // De tabel houdt over wat per bestand verschilt: het tracknummer, de titel
    // en de speelduur. Onder de titel staat de bestandsnaam met wat er aan het
    // bestand mankeert. Dat komt uit dezelfde listing als de maplijst; er gaat
    // geen bestand extra open.
    let server = Server::start_in(library_with_covers(), &[]);
    let page = server.get("/album/Album");

    assert_ok(&page);
    for column in ["#", "Titel", "Lengte"] {
        assert!(page.contains(&format!(">{column}</th>")), "{page}");
    }

    // Wat voor de hele selectie geldt, staat in het paneel ernaast en niet als
    // kolom die het per rij herhaalt.
    for column in ["Disc", "Hoes", "Bewerken", "Albumartiest", "Genre"] {
        assert!(
            !page.contains(&format!(">{column}</th>")),
            "kolom '{column}' hoort er niet meer te staan:\n{page}"
        );
    }
    assert!(!page.contains("batchtabel__hoesje"), "{page}");

    // De bestandsnaam staat onder de titel en is meteen de weg naar het
    // bewerkformulier van dit ene bestand; het vinkje houdt hem als opschrift,
    // maar buiten beeld.
    let met = row_html(&page, "een.mp3");
    assert!(met.contains("batchtabel__bestand"), "{met}");
    assert!(met.contains("/bewerk/Album/een.mp3"), "{met}");
    assert!(met.contains("vinkje__tekst alleen-voorlezen"), "{met}");
}

#[test]
fn every_row_offers_the_two_fields_that_differ_per_file() {
    // FR-9: tracknummer en titel zijn per bestand in de tabel in te tikken, en
    // dat zijn de enige twee. Albumartiest, album, jaar en genre stonden hier
    // ooit ook, náást hun gedeelde veld; dan staat hetzelfde veld twee keer op
    // één scherm en moet de tabel uitleggen welke van de twee wint.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    for name in ["een.mp3", "twee.mp3"] {
        for field in ["nummer", "titel"] {
            assert!(page.contains(&format!("name=\"{field}:{name}\"")), "{page}");
        }
        for field in ["artiest", "albumartiest", "albumtitel", "jaar", "genre"] {
            assert!(
                !page.contains(&format!("name=\"{field}:{name}\"")),
                "'{field}' hoort alleen nog als gedeeld veld te bestaan:\n{page}"
            );
        }
    }

    // AC #3: wat er nu in het bestand staat, staat als grijze tekst in het veld
    // en niet als waarde — leeg laten verandert er dus niets aan.
    let first = row_html(&page, "een.mp3");
    assert!(first.contains("placeholder="), "{first}");

    // En wat het album deelt, staat één keer, in het paneel ernaast — met wat
    // er nú in de selectie staat als tekst naast het veld.
    for current in ["De Albumartiest", ALBUM_IN_FIXTURE, "2024", "Ambient"] {
        assert!(page.contains(&format!("“{current}”")), "{page}");
    }

    // AC #5: de tabel scrollt binnen zijn eigen rand en niet met de pagina mee.
    assert!(page.contains("class=\"tabelrand\""), "{page}");
}

#[test]
fn a_shared_field_reaches_every_selected_file() {
    // FR-8: één waarde voor de hele selectie, en die hoort in de
    // voorbeeldweergave te staan en daarna werkelijk in elk bestand te belanden.
    let server = server();
    let batch: Vec<(&str, &str)> = vec![
        ("bestand", "een.mp3"),
        ("bestand", "twee.mp3"),
        ("album", "Gedeeld album"),
        ("genre", "Klassiek"),
        ("titel:een.mp3", "Eigen titel"),
    ];

    let mut fields = batch.clone();
    fields.push(("actie", "voorbeeld"));
    let page = server.post_form("/album/Album", &fields);

    assert_ok(&page);
    let first = preview_html(&page, "een.mp3");
    assert!(first.contains("Gedeeld album"), "{first}");
    assert!(first.contains("Klassiek"), "{first}");
    // En wat er per rij is ingetikt, geldt alleen daar.
    assert!(first.contains("Eigen titel"), "{first}");

    let second = preview_html(&page, "twee.mp3");
    assert!(second.contains("Gedeeld album"), "{second}");
    assert!(!second.contains("Eigen titel"), "{second}");

    let mut fields = batch.clone();
    fields.push(("actie", "opslaan"));
    let saved = server.post_form("/album/Album", &fields);

    assert_ok(&saved);
    assert!(saved.contains("2 bestanden bijgewerkt."), "{saved}");

    // Een verse leesronde: elk bestand heeft gekregen wat het voorbeeld beloofde.
    let fresh = server.get("/album/Album");
    assert!(
        fresh.contains("“Gedeeld album” in de hele selectie"),
        "{fresh}"
    );
    assert!(
        row_html(&fresh, "een.mp3").contains("placeholder=\"Eigen titel\""),
        "{fresh}"
    );
}

#[test]
fn an_empty_column_leaves_the_file_as_it_is() {
    // AC #3: leeg betekent in de tabel hetzelfde als bij een gedeeld veld —
    // ongemoeid laten. Wissen blijft een aparte, expliciete keuze, en die is
    // hier niet gemaakt.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "voorbeeld"),
            ("bestand", "een.mp3"),
            ("titel:een.mp3", "   "),
            ("nummer:een.mp3", ""),
        ],
    );

    assert_ok(&page);
    assert!(page.contains("Er verandert niets"), "{page}");
    assert!(!page.contains("wordt verwijderd"), "{page}");
}

#[test]
fn a_mistake_in_one_row_leaves_the_other_columns_and_rows_alone() {
    // AC #4: de melding staat bij het veld waarin hij is ingetikt, en houdt
    // alleen die rij tegen.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "alles"),
            ("nummer:een.mp3", "drie"),
            ("titel:een.mp3", "Wel een titel"),
            ("titel:twee.mp3", "Een Ander"),
        ],
    );

    assert_ok(&page);
    let broken = row_html(&page, "een.mp3");
    assert!(broken.contains("Tracknummer moet een getal"), "{broken}");
    assert!(broken.contains("rijveld__fout"), "{broken}");
    // Eén veld van deze rij is als onbruikbaar gemarkeerd, niet de hele rij.
    assert_eq!(broken.matches("aria-invalid").count(), 1, "{broken}");
    assert!(broken.contains("value=\"Wel een titel\""), "{broken}");

    let fine = row_html(&page, "twee.mp3");
    assert!(!fine.contains("aria-invalid"), "{fine}");
    assert!(!fine.contains("rijveld__fout"), "{fine}");

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

    // De schijf-acties zijn weg: wat ze deden gold voor een set op twee
    // schijven in één map, en die staat in de praktijk in CD1/CD2-submappen.
    for action in ["hernummer-disc", "disc", "disctotaal"] {
        assert!(
            !page.contains(&format!("name=\"actie\" value=\"{action}\"")),
            "hulpactie '{action}' hoort er niet meer te zijn:\n{page}"
        );
    }

    // Ze staan ingeklapt; wie ze nodig heeft, klapt ze open. Zonder JavaScript
    // werkt dat ook.
    assert!(
        page.contains("<summary class=\"hulpacties__kop\">"),
        "{page}"
    );
    assert!(
        !page.contains("<details class=\"hulpacties\" open>"),
        "{page}"
    );
}

#[test]
fn the_helper_actions_stay_open_once_one_has_run() {
    // De melding van de actie staat binnen de `<details>`. Dicht terugkomen zou
    // verzwijgen wat er zojuist gebeurd is.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[("actie", "hernummer"), ("bestand", "een.mp3")],
    );

    assert_ok(&page);
    assert!(
        page.contains("<details class=\"hulpacties\" open>"),
        "{page}"
    );
    assert!(page.contains("Er is niets opgeslagen"), "{page}");
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
fn copying_the_artist_fills_the_shared_album_artist() {
    // FR-10: de artiest van de selectie komt als voorstel bij Albumartiest, in
    // het gedeelde veld. Per bestand kopiëren zou een verzamelalbum uit elkaar
    // trekken: elk bestand een eigen albumartiest is precies wat een album niet
    // is.
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
    assert!(page.contains("value=\"De Testartiest\""), "{page}");
    // Het kale bestand heeft geen artiest, en telt dus niet mee.
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
/// Daarmee is te zien wat "titel uit bestandsnaam" doet: de titel staat alleen
/// nog in de naam.
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
    let list = page.find("<ul class=\"voorbeeld\">").unwrap();
    for label in ["Definitief opslaan", "Annuleren", "Terug naar de map"] {
        assert!(
            page.find(label).unwrap() < list,
            "{label} hoort boven de lijst"
        );
    }
}

#[test]
fn a_file_without_changes_is_omitted_from_the_preview() {
    // De lijst toont alleen wijzigingen; de samenvatting telt de overige bestanden.
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
    assert!(
        !page.contains("<p class=\"voorbeeld__naam\">een.mp3</p>"),
        "{page}"
    );
    assert!(
        page.contains("<p class=\"voorbeeld__naam\">twee.mp3</p>"),
        "{page}"
    );
    assert!(page.contains("De overige 1 blijven ongemoeid."), "{page}");
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

    assert!(
        !page.contains("<p class=\"voorbeeld__naam\">twee.mp3</p>"),
        "een niet-aangevinkt bestand hoort niet in de lijst: {page}"
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

// ── De balk onderaan de albumweergave ──────────────────────────────────────

/// Of de knop met deze actie aan te klikken is.
///
/// De waarde en het `disabled` staan in het template op dezelfde regel, juist
/// zodat een test er iets over kan zeggen.
fn is_enabled(page: &str, action: &str) -> bool {
    assert!(
        page.contains(&format!("value=\"{action}\"")),
        "de knop “{action}” staat niet op de pagina:\n{page}"
    );

    !page.contains(&format!("value=\"{action}\" disabled"))
}

#[test]
fn the_bar_says_what_is_pending_and_offers_no_way_to_write() {
    // Het voorbeeld moet al bereikbaar zijn vóór de eerste invoer: typen en
    // Tab versturen het formulier niet. Schrijven kan alleen via het voorbeeld.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    assert!(
        page.contains("Er is nog niets ingevuld, dus er staat niets open."),
        "{page}"
    );
    assert!(is_enabled(&page, "voorbeeld"), "{page}");
    let first_button = page
        .split("id=\"album\"")
        .nth(1)
        .unwrap()
        .split("<button")
        .nth(1)
        .unwrap()
        .split('>')
        .next()
        .unwrap();
    assert!(
        first_button.contains("value=\"voorbeeld\""),
        "{first_button}"
    );

    // De balk kent geen opslaan; die knop verschijnt pas in het voorbeeld.
    assert!(!page.contains("value=\"opslaan\""), "{page}");
    assert!(page.contains("Invoer leegmaken"), "{page}");
}

#[test]
fn the_bar_counts_the_files_that_get_a_change() {
    // AC #1: het aantal staat er terwijl je bezig bent, en het klopt met wat de
    // voorbeeldweergave daarna toont.
    let server = server();
    let fields = [
        ("bestand", "een.mp3"),
        ("bestand", "twee.mp3"),
        ("album", "Een heel ander album"),
    ];

    let page = server.post_form("/album/Album", &fields);

    assert_ok(&page);
    assert!(
        page.contains("2 bestanden krijgen een wijziging."),
        "{page}"
    );
    assert!(is_enabled(&page, "voorbeeld"), "{page}");

    let mut naar_voorbeeld = fields.to_vec();
    naar_voorbeeld.push(("actie", "voorbeeld"));
    let preview = server.post_form("/album/Album", &naar_voorbeeld);

    assert_ok(&preview);
    assert!(
        preview.contains("2 bestanden worden gewijzigd."),
        "{preview}"
    );
}

#[test]
fn the_bar_counts_nothing_when_the_value_is_already_there() {
    // AC #1 en #7: wie invult wat er al staat, hoort dat meteen te zien en niet
    // pas als het voorbeeld leeg blijkt. een.mp3 heeft dit album al; twee.mp3
    // heeft helemaal geen tags.
    let server = server();

    let both = server.post_form(
        "/album/Album",
        &[
            ("bestand", "een.mp3"),
            ("bestand", "twee.mp3"),
            ("album", ALBUM_IN_FIXTURE),
        ],
    );

    assert_ok(&both);
    assert!(both.contains("1 bestand krijgt een wijziging."), "{both}");

    // Alleen het bestand dat het album al heeft: dan verandert er niets.
    let already = server.post_form(
        "/album/Album",
        &[("bestand", "een.mp3"), ("album", ALBUM_IN_FIXTURE)],
    );

    assert_ok(&already);
    assert!(
        already.contains("Geen enkel bestand krijgt een wijziging"),
        "{already}"
    );
    // Er staat wél iets open: het voorbeeld blijft bereikbaar, want daar hangt
    // ook de hoes aan.
    assert!(is_enabled(&already, "voorbeeld"), "{already}");
}

#[test]
fn emptying_the_input_from_the_bar_keeps_the_selection() {
    // AC #3: in één klik leeg, en de vinkjes blijven staan.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("actie", "herstel"),
            ("bestand", "een.mp3"),
            ("album", "Een heel ander album"),
        ],
    );

    assert_ok(&page);
    assert!(is_ticked(&page, "een.mp3"), "{page}");
    assert!(!page.contains("value=\"Een heel ander album\""), "{page}");
    assert!(
        page.contains("Er is nog niets ingevuld, dus er staat niets open."),
        "{page}"
    );
}

#[test]
fn the_table_groups_the_files_per_disc() {
    // AC #1 t/m #3: een kop per schijf, de bestanden zonder discnummer als
    // eigen groep achteraan, met de telling en wat er aandacht vraagt.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    assert!(page.contains("Schijf 1"), "{page}");
    assert!(page.contains("Zonder discnummer"), "{page}");

    // De getagde fixture staat op schijf 1; de kale heeft geen discnummer en
    // mist zo ongeveer alles, en dat telt de kop van zijn groep.
    assert!(page.contains("1 bestand"), "{page}");
    assert!(page.contains("1 vraagt aandacht"), "{page}");

    // De kop wijst aan waar een schijf begint en verder niets: selecteren doen
    // de vinkjes en de twee knoppen boven de lijst.
    assert!(!page.contains("value=\"schijf:"), "{page}");
}

#[test]
fn a_set_of_two_discs_gets_a_heading_per_disc() {
    // Geen enkele fixture staat op schijf 2; die komt hier via de app zelf in
    // het bestand, langs de enige route die schrijft.
    let server = server();
    let saved = server.post_form(
        "/album/Album",
        &[("actie", "opslaan"), ("bestand", "twee.mp3"), ("disc", "2")],
    );
    assert_ok(&saved);
    assert!(saved.contains("1 bestand bijgewerkt"), "{saved}");

    let page = server.get("/album/Album");
    assert_ok(&page);
    assert!(page.contains("Schijf 1"), "{page}");
    assert!(page.contains("Schijf 2"), "{page}");
    assert!(
        !page.contains("Zonder discnummer"),
        "elk bestand heeft nu een schijf:\n{page}"
    );
}

/// Bouwt een map waarin de hoezen uiteenlopen.
///
/// `een.mp3` en `twee.mp3` dragen dezelfde hoes (300 × 300 JPEG), `drie.mp3`
/// een andere (500 × 500 PNG) en `vier.mp3` helemaal geen. Daarmee is elke
/// uitspraak te maken die het hoespaneel over een selectie kan doen.
fn library_with_covers() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "een.mp3", "tagged-with-art.mp3");
    place_fixture(&album, "twee.mp3", "tagged-with-art.mp3");
    place_fixture(&album, "drie.mp3", "tagged-with-other-art.mp3");
    place_fixture(&album, "vier.mp3", "tagged.mp3");

    root
}

/// Vraagt de albumweergave op met precies deze bestanden aangevinkt.
fn album_with_selection(server: &Server, files: &[&str]) -> String {
    let fields: Vec<(&str, &str)> = files.iter().map(|name| ("bestand", *name)).collect();
    let page = server.post_form("/album/Album", &fields);

    assert_ok(&page);
    page
}

#[test]
fn the_album_view_shows_the_cover_of_the_selection() {
    let server = Server::start_in(library_with_covers(), &[]);
    let page = album_with_selection(&server, &["een.mp3", "twee.mp3"]);

    assert!(
        page.contains("Dezelfde hoes in deze 2 bestanden."),
        "{page}"
    );
    assert!(
        page.contains("JPEG · 300 × 300 pixels · 1,3 kB"),
        "formaat, afmetingen en omvang horen erbij:\n{page}"
    );
    assert!(
        page.contains("/art/Album/een.mp3?size=paneel"),
        "de hoes zelf hoort in het paneel te staan:\n{page}"
    );

    // De knop noemt op hoeveel bestanden de hoes terechtkomt, en staat er ook
    // zonder ingevuld veld: een hoes kiezen is op zichzelf al iets te doen.
    assert!(
        page.contains("value=\"voorbeeld\">Hoes vervangen in deze 2 bestanden"),
        "{page}"
    );
}

#[test]
fn a_selection_with_different_covers_says_so_instead_of_showing_one() {
    let server = Server::start_in(library_with_covers(), &[]);
    let page = album_with_selection(&server, &["een.mp3", "drie.mp3"]);

    assert!(
        page.contains("De hoes wisselt binnen de selectie: 2 verschillende in deze 2 bestanden."),
        "{page}"
    );
    assert!(
        !page.contains("hoespaneel__afbeelding"),
        "er hoort er geen uitgekozen te worden:\n{page}"
    );
}

#[test]
fn a_selection_in_which_not_every_file_has_a_cover_says_so() {
    let server = Server::start_in(library_with_covers(), &[]);
    let page = album_with_selection(&server, &["een.mp3", "vier.mp3"]);

    assert!(
        page.contains("Eén hoes in 1 van de 2 bestanden; de rest heeft er geen."),
        "{page}"
    );
    assert!(
        !page.contains("hoespaneel__afbeelding"),
        "een hoes tonen zou het bestand zonder verzwijgen:\n{page}"
    );
    assert!(
        page.contains("value=\"voorbeeld\">Hoes in deze 2 bestanden zetten"),
        "{page}"
    );
}

#[test]
fn the_cover_panel_writes_nothing_and_hands_its_choice_to_the_preview() {
    let root = library_with_covers();
    let album = root.path().join("Album");
    let files = ["een.mp3", "twee.mp3", "drie.mp3", "vier.mp3"];
    let before: Vec<Vec<u8>> = files
        .iter()
        .map(|name| std::fs::read(album.join(name)).expect("bestand moet leesbaar zijn"))
        .collect();

    let server = Server::start_in(root, &[]);

    // Het vinkje uit het paneel reist als gewoon formulierveld mee en komt bij
    // een volgende ronde aangevinkt terug. De waarde en het `checked` staan in
    // het template op dezelfde regel, juist zodat een test er iets over kan
    // zeggen.
    let page = server.post_form(
        "/album/Album",
        &[("bestand", "een.mp3"), ("mapbestand", "ja")],
    );
    assert_ok(&page);
    assert!(page.contains("value=\"ja\" checked"), "{page}");

    // De knop in het paneel leidt naar de voorbeeldweergave — de enige stap die
    // schrijft — en de keuze staat daar aangevinkt klaar.
    let preview = server.post_form(
        "/album/Album",
        &[
            ("bestand", "een.mp3"),
            ("mapbestand", "ja"),
            ("actie", "voorbeeld"),
        ],
    );
    assert_ok(&preview);
    assert!(preview.contains("Definitief opslaan"), "{preview}");
    assert!(preview.contains("value=\"ja\" checked"), "{preview}");
    assert!(
        preview.contains("name=\"afbeelding\""),
        "het bestandsveld hoort in deze stap te staan:\n{preview}"
    );

    // Geen van beide verzoeken heeft een byte geschreven, ook de losse cover.jpg
    // niet.
    for (name, original) in files.iter().zip(before) {
        let now = std::fs::read(album.join(name)).expect("bestand moet leesbaar zijn");
        assert_eq!(now, original, "{name} is aangeraakt en dat hoort niet");
    }
    assert!(
        !album.join("cover.jpg").exists(),
        "er hoort geen losse hoes geschreven te zijn"
    );
}

/// Een map met twee schijven, om de kop boven de lijst iets te laten tellen.
///
/// De discnummers zitten in de fixtures zelf niet, dus ze worden hier gezet via
/// dezelfde weg als een gebruiker: de albumweergave, en daarna de
/// voorbeeldweergave die als enige schrijft.
fn library_with_two_discs() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "een.mp3", "tagged.mp3");
    place_fixture(&album, "twee.mp3", "tagged.mp3");

    root
}

/// De indeling uit TASK-44: één paneel naast de lijst.
///
/// Wat hier te controleren valt zonder browser, is de volgorde in de HTML en
/// welke onderdelen in welk blok staan. Dát het paneel op een breed scherm
/// links komt te staan en op een smal scherm onder de lijst valt, doet
/// `static/app.css` met `grid-column` en een `@media`-regel; dat is alleen in
/// een browser te zien en staat daarom in de notities van de taak en niet hier.
#[test]
fn the_editor_stands_beside_the_list_in_one_panel() {
    let server = server();
    let page = server.get("/album/Album");
    assert_ok(&page);

    // De twee kolommen zijn er, en de lijst staat vóór het paneel in de HTML:
    // zonder stylesheet — en op een smal scherm — is dat de volgorde waarin ze
    // onder elkaar komen, en de lijst hoort dan boven te staan.
    let lijst = page
        .find("albumlayout__lijst")
        .expect("de lijstkolom hoort er te staan");
    let paneel = page
        .find("class=\"editor\"")
        .expect("het bewerkpaneel hoort er te staan");
    assert!(lijst < paneel, "de lijst hoort vóór het paneel te staan");

    // AC #2: hoes, gedeelde velden en hulpacties staan samen in dat paneel, in
    // de volgorde van het ontwerp.
    let hoes = page.find("class=\"hoespaneel\"").expect("hoes");
    let velden = page.find("class=\"gedeeld\"").expect("gedeelde velden");
    let hulp = page.find("class=\"hulpacties\"").expect("hulpacties");
    assert!(paneel < hoes, "de hoes hoort in het paneel te staan");
    assert!(hoes < velden, "de velden horen onder de hoes te staan");
    assert!(
        hulp > velden,
        "de hulpacties horen onder de velden te staan"
    );

    // De tabel staat in de lijstkolom en niet in het paneel.
    let tabel = page.find("class=\"batchtabel\"").expect("de tabel");
    assert!(
        lijst < tabel && tabel < paneel,
        "de tabel hoort in de lijstkolom te staan"
    );

    // AC #8: geen knop op dit scherm schrijft. De enige weg vooruit is de
    // voorbeeldweergave.
    assert!(page.contains("value=\"voorbeeld\""), "{page}");
    assert!(!page.contains("Definitief opslaan"), "{page}");
}

/// AC #3: de korte velden staan naast elkaar op één regel.
///
/// De groepering komt uit `AlbumPage::field_rows` en niet uit de template; wat
/// hier getest wordt is dat de HTML die groepering ook werkelijk uitschrijft.
/// Hoe breed die regel dan wordt, is een zaak van de stylesheet.
#[test]
fn the_short_fields_share_one_row() {
    let server = server();
    let page = server.get("/album/Album");
    assert_ok(&page);

    let rijen: Vec<&str> = page.split("class=\"gedeeld__rij\"").skip(1).collect();
    assert_eq!(rijen.len(), 4, "vier regels velden verwacht:\n{page}");

    // Jaar, discnummer en aantal discs delen de derde regel; albumartiest,
    // album en genre krijgen er elk een voor zichzelf.
    let kort = rijen[2];
    for veld in ["gedeeld-year", "gedeeld-disc", "gedeeld-disc_total"] {
        assert!(
            kort.contains(veld),
            "{veld} hoort op de korte regel:\n{kort}"
        );
    }
    assert!(rijen[0].contains("gedeeld-album_artist"), "{page}");
    assert!(rijen[1].contains("gedeeld-album"), "{page}");
    assert!(rijen[3].contains("gedeeld-genre"), "{page}");
}

/// AC #5: de kop boven de lijst, met de telling en de knoppen ernaast.
#[test]
fn the_list_carries_its_own_heading_with_the_count() {
    let server = Server::start_in(library_with_two_discs(), &[]);

    // Zonder discnummers zwijgt de kop erover: "0 schijven" is geen mededeling.
    let page = server.get("/album/Album");
    assert_ok(&page);
    assert!(page.contains("2 van 2 bestanden geselecteerd"), "{page}");
    assert!(!page.contains("schijven"), "{page}");
    assert!(page.contains("lijstkop__naam"), "{page}");

    // De knoppen die de hele selectie zetten, staan in diezelfde kop.
    let kop = page
        .find("class=\"lijstkop\"")
        .expect("de kop boven de lijst");
    let tabel = page.find("class=\"batchtabel\"").expect("de tabel");
    let alles = page.find("value=\"alles\"").expect("Alles selecteren");
    let niets = page.find("value=\"niets\"").expect("Niets selecteren");
    assert!(kop < alles && alles < tabel, "{page}");
    assert!(kop < niets && niets < tabel, "{page}");

    // Zodra de map schijven kent, staat het aantal erbij. De nummers worden via
    // de gewone route gezet: eerst het voorbeeld, dan opslaan.
    let opgeslagen = server.post_form(
        "/album/Album",
        &[("bestand", "een.mp3"), ("disc", "1"), ("actie", "opslaan")],
    );
    assert_ok(&opgeslagen);

    let opgeslagen = server.post_form(
        "/album/Album",
        &[("bestand", "twee.mp3"), ("disc", "2"), ("actie", "opslaan")],
    );
    assert_ok(&opgeslagen);

    // Twee labels naast de naam, zoals het ontwerp ze zet: de telling en de
    // schijven staan naast elkaar en niet als één zin achter de kop.
    let page = server.get("/album/Album");
    assert_ok(&page);
    assert!(page.contains("2 van 2 bestanden geselecteerd"), "{page}");
    assert!(page.contains(">2 schijven<"), "{page}");
}
