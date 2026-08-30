//! Een reeks bestanden selecteren met de muis.
//!
//! Het selecteren zelf draait in de browser: klikken op een regel selecteert
//! dat bestand, shift-klikken alles ertussen, ctrl of cmd haalt er één bij of
//! weg. Dat is muisgedrag, en een test zonder browser ziet er niets van.
//!
//! Wat een test wél kan vaststellen, is of de server aanlevert waar dat gedrag
//! op steunt, en of het niets kapotmaakt van wat er zonder script overblijft:
//!
//! * elke regel draagt de bestandsnaam waar ze over gaat, in de volgorde
//!   waarin de lijst op het scherm staat — die volgorde ís de reeks die een
//!   shift-klik selecteert, ook wanneer de lijst per schijf gegroepeerd is;
//! * de vinkjes zijn onveranderd en blijven het werk doen: ze posten hetzelfde
//!   formulier, en een POST met een handvol `bestand`-waarden levert precies
//!   die selectie op;
//! * de pagina belooft niets wat ze zonder script niet waarmaakt: de markering
//!   waaraan te zien is dat er op een regel te klikken valt, zet `app.js` en
//!   staat niet in de template.
//!
//! Wat hier dus niet staat en alleen in een browser te zien is: het uitstrekken
//! van een reeks met shift, het anker dat op zijn plaats blijft bij een tweede
//! shift-klik, het erbij halen met ctrl of cmd, en dat een klik in een
//! invoerveld de selectie met rust laat.

mod common;

use common::{Server, place_fixture};

/// Het attribuut waaraan `app.js` ziet om welk bestand een regel gaat.
const GREEP: &str = "data-bestand";

/// Bouwt een bibliotheek met één album van drie bestanden.
///
/// Drie en niet twee: een reeks van begin tot eind zegt pas iets wanneer er
/// ook iets tussenin ligt.
fn library() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "een.mp3", "tagged.mp3");
    place_fixture(&album, "twee.mp3", "untagged.mp3");
    place_fixture(&album, "drie.mp3", "untagged.mp3");

    root
}

fn server() -> Server {
    Server::start_in(library(), &[])
}

fn assert_ok(response: &str) {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
}

/// Of het vinkje van dit bestand aan staat.
fn is_ticked(page: &str, name: &str) -> bool {
    page.contains(&format!("value=\"{name}\" checked"))
}

/// De bestandsnamen van de regels, in de volgorde waarin ze in de tabel staan.
fn rows(page: &str) -> Vec<String> {
    page.match_indices(GREEP)
        .filter_map(|(at, _)| {
            let rest = page[at..].strip_prefix(&format!("{GREEP}=\""))?;
            let end = rest.find('"')?;
            Some(rest[..end].to_string())
        })
        .collect()
}

#[test]
fn every_row_says_which_file_it_holds() {
    // Zonder die greep weet het script van geen enkele regel waar ze over gaat,
    // en valt er niets te selecteren.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    assert_eq!(
        rows(&page).len(),
        3,
        "elke regel hoort de naam van haar bestand te dragen:\n{page}"
    );

    for name in ["een.mp3", "twee.mp3", "drie.mp3"] {
        assert!(
            page.contains(&format!("{GREEP}=\"{name}\"")),
            "{name} ontbreekt:\n{page}"
        );
    }
}

#[test]
fn the_rows_stand_in_the_order_a_range_follows() {
    // Een reeks loopt van de ene regel naar de andere zoals de lijst er op dat
    // moment uitziet. De koppen per schijf zetten die volgorde: wat op schijf 1
    // staat komt eerst, wat geen discnummer heeft achteraan. Staat de greep in
    // een andere volgorde dan de lijst, dan selecteert een shift-klik iets
    // anders dan wat er tussen de twee regels te zien is.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);

    // `een.mp3` is de getagde fixture en staat op schijf 1; de twee kale
    // bestanden hebben geen discnummer en vormen samen de laatste groep.
    let ticked = rows(&page);
    assert_eq!(
        ticked,
        vec!["een.mp3", "drie.mp3", "twee.mp3"],
        "de regels horen in de volgorde van de lijst te staan:\n{page}"
    );

    let disc_one = page.find("Schijf 1").expect("kop van schijf 1");
    let without = page
        .find("Zonder discnummer")
        .expect("kop zonder discnummer");
    let first = page.find(&format!("{GREEP}=\"een.mp3\"")).expect("een.mp3");
    let last = page
        .find(&format!("{GREEP}=\"twee.mp3\""))
        .expect("twee.mp3");

    assert!(
        disc_one < first && first < without && without < last,
        "elke regel hoort onder de kop van haar eigen groep te staan:\n{page}"
    );
}

#[test]
fn the_checkboxes_are_untouched() {
    // Ze blijven de weg voor wie geen muis gebruikt, en het enige dat er zonder
    // script overblijft: dezelfde naam, dezelfde waarde, dezelfde post.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    assert!(
        page.contains("type=\"checkbox\" name=\"bestand\""),
        "{page}"
    );
    assert!(page.contains("value=\"een.mp3\" checked"), "{page}");
    assert!(page.contains("hx-include=\"closest form\""), "{page}");
}

#[test]
fn without_javascript_the_checkboxes_do_the_work() {
    // Precies wat een browser verstuurt wanneer er twee vinkjes aan staan. Het
    // script zet diezelfde vinkjes en post hetzelfde formulier; er is dus maar
    // één manier waarop een selectie de server bereikt.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[("bestand", "een.mp3"), ("bestand", "drie.mp3")],
    );

    assert_ok(&page);
    assert!(page.contains("2 van 3 bestanden geselecteerd"), "{page}");
    assert!(is_ticked(&page, "een.mp3"), "{page}");
    assert!(is_ticked(&page, "drie.mp3"), "{page}");
    assert!(!is_ticked(&page, "twee.mp3"), "{page}");
}

#[test]
fn the_page_promises_nothing_without_the_script() {
    // De markering waaraan de opmaak ziet dat er op een regel te klikken valt,
    // komt uit `app.js`. Stond ze in de template, dan zou een regel er zonder
    // script uitzien alsof een klik erop iets doet.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);
    assert!(
        !page.contains("batchtabel--selecteerbaar"),
        "die klasse hoort het script te zetten:\n{page}"
    );
    assert!(
        !page.contains("onclick"),
        "gedrag hoort niet in de template te staan:\n{page}"
    );
}

#[test]
fn the_script_carries_the_selecting() {
    let server = server();
    let script = server.get("/static/app.js");

    assert_ok(&script);
    assert!(script.contains(GREEP), "app.js kent de greep niet");
    assert!(script.contains("shiftKey"), "app.js kent shift niet");
    assert!(
        script.contains("metaKey") && script.contains("ctrlKey"),
        "app.js kent ctrl of cmd niet"
    );
}

#[test]
fn the_style_only_marks_a_row_that_can_be_clicked() {
    // De hover hangt onder de klasse die het script zet; zonder script gebeurt
    // er niets, en ziet de tabel eruit zoals hij deed.
    let server = server();
    let css = server.get("/static/app.css");

    assert_ok(&css);
    assert!(
        css.contains(".batchtabel--selecteerbaar .batchtabel__rij:hover"),
        "de opmaak hoort onder de klasse van het script te hangen"
    );
}
