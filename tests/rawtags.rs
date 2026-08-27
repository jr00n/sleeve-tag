//! De geavanceerde weergave over HTTP, tegen de echte binary (FR-7).
//!
//! De unit-tests in `src/tags` controleren wat er uit een bestand komt; deze
//! test controleert wat de beheerder werkelijk op het scherm krijgt: de
//! oorspronkelijke sleutelnamen per formaat, een samengevatte hoes, en geen
//! enkele manier om er iets aan te veranderen.
//!
//! De bibliotheek is een tempdir met kopieën van de fixtures. De echte
//! muziekbibliotheek wordt nooit aangeraakt.

mod common;

use common::{Server, place_fixture};

fn library_with_every_variant() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "getagd.mp3", "tagged.mp3");
    place_fixture(&album, "getagd.flac", "tagged.flac");
    place_fixture(&album, "met-hoes.mp3", "tagged-with-art.mp3");
    place_fixture(&album, "kaal.mp3", "untagged.mp3");
    std::fs::write(album.join("notities.txt"), b"tekst").expect("bestand moet te schrijven zijn");

    root
}

fn assert_ok(response: &str) {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
}

#[test]
fn an_mp3_shows_its_id3_frames() {
    let server = Server::start_in(library_with_every_variant(), &[]);
    let response = server.get("/tags/Album/getagd.mp3");
    assert_ok(&response);

    // De oorspronkelijke frame-ID's, niet de genormaliseerde veldnamen.
    for expected in ["ID3v2", "TIT2", "TPE1", "TALB", "TRCK"] {
        assert!(
            response.contains(expected),
            "'{expected}' ontbreekt op de pagina:\n{response}"
        );
    }

    // En de waarden die erbij horen.
    assert!(
        response.contains("Stilte in D"),
        "de titelwaarde ontbreekt:\n{response}"
    );
}

#[test]
fn a_flac_shows_its_vorbis_comments() {
    let server = Server::start_in(library_with_every_variant(), &[]);
    let response = server.get("/tags/Album/getagd.flac");
    assert_ok(&response);

    for expected in ["Vorbis-comments", "TITLE", "ARTIST", "ALBUM"] {
        assert!(
            response.contains(expected),
            "'{expected}' ontbreekt op de pagina:\n{response}"
        );
    }

    // Vorbis-comments dragen geen ID3-frame-ID's; dat onderscheid is precies
    // waar deze weergave voor bedoeld is.
    assert!(
        !response.contains("TIT2"),
        "er staan ID3v2-frames op de pagina van een FLAC:\n{response}"
    );
}

#[test]
fn embedded_art_is_summarised_instead_of_dumped() {
    let server = Server::start_in(library_with_every_variant(), &[]);
    let response = server.get("/tags/Album/met-hoes.mp3");
    assert_ok(&response);

    assert!(
        response.contains("image/jpeg"),
        "het type van de hoes ontbreekt:\n{response}"
    );
    assert!(
        response.contains("bytes"),
        "de grootte van de hoes ontbreekt:\n{response}"
    );
    assert!(
        response.len() < 20_000,
        "het antwoord is {} bytes; dat ruikt naar ruwe afbeeldingsdata",
        response.len()
    );
}

#[test]
fn the_page_offers_no_way_to_change_a_raw_tag() {
    let server = Server::start_in(library_with_every_variant(), &[]);

    for file in ["getagd.mp3", "getagd.flac", "met-hoes.mp3", "kaal.mp3"] {
        let response = server.get(&format!("/tags/Album/{file}"));
        assert_ok(&response);

        for forbidden in ["<form", "<input", "<textarea", "<button", "<select"] {
            assert!(
                !response.contains(forbidden),
                "{file}: '{forbidden}' staat op een alleen-lezen pagina"
            );
        }
    }
}

#[test]
fn a_file_without_a_tag_block_says_so() {
    let server = Server::start_in(library_with_every_variant(), &[]);
    let response = server.get("/tags/Album/kaal.mp3");

    assert_ok(&response);
    assert!(
        response.contains("geen tagblok"),
        "een bestand zonder tags hoort dat te melden:\n{response}"
    );
}

#[test]
fn there_is_a_way_back_to_the_directory() {
    let server = Server::start_in(library_with_every_variant(), &[]);
    let response = server.get("/tags/Album/getagd.mp3");

    assert!(
        response.contains(r#"href="/map/Album""#),
        "er is geen weg terug naar de map:\n{response}"
    );
    assert!(
        response.contains("Bibliotheek"),
        "het broodkruimelpad begint niet bij de bibliotheek:\n{response}"
    );
}

#[test]
fn what_is_not_audio_or_not_in_the_library_is_refused() {
    let server = Server::start_in(library_with_every_variant(), &[]);

    let text = server.get("/tags/Album/notities.txt");
    assert!(
        text.starts_with("HTTP/1.1 415"),
        "antwoord begon met: {}",
        text.lines().next().unwrap_or_default()
    );

    for attempt in ["/tags/../../etc/passwd", "/tags/Album/bestaat-niet.mp3"] {
        let response = server.get(attempt);
        assert!(
            !response.starts_with("HTTP/1.1 200"),
            "'{attempt}' leverde een pagina op:\n{response}"
        );
    }
}
