//! De bezig-weergave bij een schrijfactie.
//!
//! Op de NAS duurt een tagwijziging in een FLAC van enkele gigabytes minuten:
//! `atomic::replace` kopieert eerst het hele bestand. Zolang dat loopt hoort de
//! knop te laten zien dat er gewerkt wordt en geen tweede klik meer aan te
//! nemen — anders start een ongeduldige gebruiker een tweede schrijfactie op
//! hetzelfde bestand.
//!
//! Het gedrag zelf zit in `static/app.js` en draait in de browser; wat hier
//! getest wordt is wat de server aanlevert. Dat is precies de grens van wat een
//! test zonder browser kan vaststellen: staat het script er, wordt het lokaal
//! geserveerd, en dragen de knoppen die schrijven de markering waar het script
//! op afgaat — en de knoppen die niets schrijven juist niet.

mod common;

use common::{Server, place_fixture};

/// De markering waaraan het script een schrijvende knop herkent.
const MARKERING: &str = "data-bezig";

/// Bouwt een bibliotheek met één album van twee bestanden.
fn library() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "een.mp3", "tagged.mp3");
    place_fixture(&album, "twee.mp3", "untagged.mp3");

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

#[test]
fn the_script_is_served_locally() {
    // Net als htmx: van schijf, want de NAS heeft geen internetverbinding.
    let server = server();
    let response = server.get("/static/app.js");

    assert_ok(&response);
    assert!(response.contains(MARKERING), "app.js lijkt leeg of anders");
}

#[test]
fn every_page_loads_the_script() {
    let server = server();
    let page = server.get("/");

    assert_ok(&page);
    assert!(page.contains("/static/app.js"), "pagina was:\n{page}");
}

#[test]
fn the_save_button_of_the_edit_form_is_marked() {
    let server = server();
    let page = server.get("/bewerk/Album/een.mp3");

    assert_ok(&page);
    assert!(page.contains(MARKERING), "pagina was:\n{page}");

    // De tekst die de knop tijdens het schrijven toont hoort erbij te staan;
    // zonder die tekst zou het script niets te tonen hebben.
    assert!(page.contains("Bezig met opslaan"), "pagina was:\n{page}");
}

#[test]
fn the_cover_form_marks_what_writes() {
    let server = server();
    let page = server.get("/hoes/Album/een.mp3");

    assert_ok(&page);

    // Embedden schrijft: dat is de knop die minuten kan duren.
    assert!(page.contains("Bezig met embedden"), "pagina was:\n{page}");
}

#[test]
fn the_batch_marks_only_the_button_that_writes() {
    // De voorbeeldweergave is de enige route die een batch wegschrijft. Daar
    // staan twee knoppen naast elkaar: opslaan schrijft, annuleren niet.
    let server = server();
    let page = server.post_form(
        "/album/Album",
        &[
            ("bestand", "een.mp3"),
            ("bestand", "twee.mp3"),
            ("album", "Een heel ander album"),
            ("actie", "voorbeeld"),
        ],
    );

    assert_ok(&page);

    let opslaan = page
        .find("value=\"opslaan\"")
        .expect("de opslaanknop hoort op de voorbeeldweergave te staan");
    let annuleren = page
        .find("value=\"terug\"")
        .expect("de annuleerknop hoort op de voorbeeldweergave te staan");

    // Beide knoppen staan in hetzelfde blok; de markering hoort bij de eerste
    // te staan en niet bij de tweede.
    let bij_opslaan = &page[opslaan..annuleren];
    let vanaf_annuleren = &page[annuleren..];

    assert!(
        bij_opslaan.contains(MARKERING),
        "de opslaanknop mist de markering:\n{bij_opslaan}"
    );
    assert!(
        !vanaf_annuleren.contains(MARKERING),
        "annuleren schrijft niets en hoort geen bezig-weergave te krijgen:\n{vanaf_annuleren}"
    );
}

#[test]
fn helper_actions_are_not_marked() {
    // Hernummeren, selecteren en de rest vullen alleen invoervelden. Die zijn
    // meteen klaar; een spinner zou daar alleen maar in de weg zitten.
    let server = server();
    let page = server.get("/album/Album");

    assert_ok(&page);

    for action in [
        "alles",
        "niets",
        "hernummer",
        "artiest",
        "hoofdletters",
        "herstel",
    ] {
        let knop = format!("value=\"{action}\"");
        let positie = page
            .find(&knop)
            .unwrap_or_else(|| panic!("hulpactie '{action}' ontbreekt op de pagina"));

        // Vanaf de waarde tot het einde van de knop: daar hoort de markering
        // niet te staan.
        let staart = &page[positie..];
        let einde = staart.find('>').unwrap_or(staart.len());
        assert!(
            !staart[..einde].contains(MARKERING),
            "hulpactie '{action}' hoort geen bezig-weergave te krijgen"
        );
    }
}

#[test]
fn hidden_really_hides() {
    // Het attribuut `hidden` is niet meer dan `display: none` uit de
    // standaardstijl van de browser, en élke eigen `display`-regel wint
    // daarvan. Zonder deze regel stond de knop "In dit bestand zetten" op de
    // bewerkpagina in beeld zonder dat er iets was neergezet — het blok eromheen
    // had `display: flex`.
    //
    // Dit is niet in een test zonder browser te zien, en juist daarom staat hij
    // hier: wie de regel weghaalt, laat elk `hidden` in de templates stukgaan.
    let server = server();
    let css = server.get("/static/app.css");

    assert_ok(&css);

    let regel = css
        .split("[hidden]")
        .nth(1)
        .expect("app.css hoort `hidden` af te dwingen");
    assert!(
        regel.trim_start().starts_with('{') && regel.contains("display: none !important"),
        "de regel voor `hidden` klopt niet: {}",
        &regel[..regel.len().min(120)]
    );
}
