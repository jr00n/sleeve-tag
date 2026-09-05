//! Een FLAC met een ID3-blok, over HTTP en tegen de echte binary.
//!
//! Zulke bestanden bestaan echt: op de bibliotheek van de NAS staan hele albums
//! die een oudere ripper met een ID3v2-blok vóór de FLAC heeft achtergelaten.
//! De FLAC-standaard kent alleen Vorbis-comments, dus zo'n blok wordt niet
//! gelezen en niet bijgewerkt — het zegt na de eerste bewerking iets anders dan
//! de tag die er wél toe doet.
//!
//! De unit-tests in `src/tags` dekken het opruimen zelf. Deze test dekt wat de
//! gebruiker ervan merkt: dat het opvalt vóór het bewerken, dat het te zien is,
//! en dat het opruimen gemeld wordt in plaats van stilzwijgend te gebeuren.

mod common;

use common::{Server, place_fixture};

/// Bouwt een album met één FLAC die zo'n blok draagt, en één die schoon is.
fn library_with_a_stray_id3() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "met-id3.flac", "id3-in-flac.flac");
    place_fixture(&album, "schoon.flac", "tagged.flac");

    root
}

fn server() -> Server {
    Server::start_in(library_with_a_stray_id3(), &[])
}

fn assert_ok(response: &str) {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
}

/// De velden zoals de browser ze verstuurt: allemaal, ook de ongewijzigde.
fn fields(title: &str) -> Vec<(&str, &str)> {
    vec![
        ("title", title),
        ("artist", "De Testartiest"),
        ("album_artist", "De Albumartiest"),
        ("album", "Fixtures voor Sleeve"),
        ("track", "3"),
        ("track_total", "12"),
        ("disc", "1"),
        ("disc_total", "2"),
        ("year", "2024"),
        ("genre", "Ambient"),
        ("composer", "De Componist"),
        ("comment", "Gegenereerd voor de tests van Sleeve"),
    ]
}

#[test]
fn the_folder_listing_points_at_the_file() {
    // Vóór het bewerken, en zonder dat de gebruiker het bestand hoeft te openen.
    let server = server();
    let page = server.get("/map/Album");

    assert_ok(&page);
    assert!(
        page.contains("tagblok dat er niet hoort"),
        "de lijst meldt het niet:\n{page}"
    );
}

#[test]
fn the_raw_page_shows_both_blocks_and_says_which_one_counts() {
    let server = server();
    let page = server.get("/tags/Album/met-id3.flac");

    assert_ok(&page);

    // Beide blokken staan er, met hun soort erbij.
    assert!(page.contains("Vorbis-comments"), "pagina was:\n{page}");
    assert!(page.contains("ID3v2"), "pagina was:\n{page}");

    // De inhoud van het vreemde blok is te zien: het spreekt de Vorbis-comments
    // tegen, en dat is precies wat hier vastgesteld moet kunnen worden.
    assert!(
        page.contains("Titel uit het ID3-blok"),
        "de inhoud van het ID3-blok ontbreekt:\n{page}"
    );
    assert!(page.contains("Stilte in D"), "pagina was:\n{page}");

    // En er staat bij wat eraan mankeert.
    assert!(
        page.contains("hoort niet in een FLAC-bestand"),
        "de waarschuwing ontbreekt:\n{page}"
    );
}

#[test]
fn a_clean_file_shows_only_one_block_and_no_warning() {
    let server = server();
    let page = server.get("/tags/Album/schoon.flac");

    assert_ok(&page);
    assert!(page.contains("Vorbis-comments"), "pagina was:\n{page}");
    assert!(
        !page.contains("hoort niet in een"),
        "een schoon bestand hoort geen waarschuwing te krijgen:\n{page}"
    );
}

#[test]
fn saving_removes_the_block_and_says_so() {
    let server = server();

    let page = server.post_form("/bewerk/Album/met-id3.flac", &fields("Nieuwe titel"));
    assert_ok(&page);

    // Het opruimen gebeurt niet stilzwijgend: er wordt metadata uit het bestand
    // gehaald, en dat hoort de gebruiker te lezen.
    assert!(page.contains("Opgeslagen."), "pagina was:\n{page}");
    assert!(
        page.contains("ID3v2-blok"),
        "de melding over het verwijderde blok ontbreekt:\n{page}"
    );

    // En daarna is het weg: de pagina met ruwe tags toont nog één blok.
    let raw = server.get("/tags/Album/met-id3.flac");
    assert!(
        !raw.contains("Titel uit het ID3-blok"),
        "het blok staat er nog:\n{raw}"
    );
    assert!(
        !raw.contains("hoort niet in een FLAC-bestand"),
        "er wordt nog gewaarschuwd terwijl het blok weg is:\n{raw}"
    );

    // De maplijst meldt het ook niet meer.
    let listing = server.get("/map/Album");
    assert!(
        !listing.contains("tagblok dat er niet hoort"),
        "de lijst meldt het nog steeds:\n{listing}"
    );
}

#[test]
fn saving_nothing_leaves_the_block_alone() {
    // Een bestand van gigabytes herschrijven om iets op te ruimen wat de
    // gebruiker niet heeft aangeraakt, is de ongevraagde wijziging die het PRD
    // verbiedt. Het blok verdwijnt alleen bij een echte bewerking of wanneer de
    // gebruiker de aparte opruimactie kiest, niet door deze lege save.
    let server = server();

    let page = server.post_form("/bewerk/Album/met-id3.flac", &fields("Stilte in D"));
    assert_ok(&page);

    assert!(
        page.contains("ID3v2-blok verwijderen"),
        "het gebleven blok en de aparte opruimactie moeten zichtbaar zijn:\n{page}"
    );
    assert!(
        !page.contains("het is bij het opslaan verwijderd"),
        "de pagina mag niet beweren dat het blok al verwijderd is:\n{page}"
    );

    let raw = server.get("/tags/Album/met-id3.flac");
    assert!(
        raw.contains("Titel uit het ID3-blok"),
        "het blok had moeten blijven staan:\n{raw}"
    );
}

#[test]
fn the_log_does_not_repeat_itself_for_every_file() {
    // De tagbibliotheek waarschuwt bij élk inlezen van zo'n bestand. Op een map
    // met tientallen van deze albums verdringt dat alles wat er wél toe doet;
    // Sleeve meldt het zelf, in de lijst en op de pagina met ruwe tags.
    let server = server();
    let _ = server.get("/map/Album");
    let _ = server.get("/bewerk/Album/met-id3.flac");

    let log = server.log();
    assert!(
        !log.contains("cannot be rewritten"),
        "de waarschuwing van de tagbibliotheek staat nog in de log:\n{log}"
    );
}

#[test]
fn the_warnings_come_back_when_asked_for() {
    // Gedempt is niet hetzelfde als weggegooid: wie ze wil zien, zet het zelf
    // in LOG_LEVEL.
    let server = Server::start_in(
        library_with_a_stray_id3(),
        &[("LOG_LEVEL", "info,lofty=warn")],
    );
    let _ = server.get("/map/Album");

    server.wait_for_log("cannot be rewritten");
}
