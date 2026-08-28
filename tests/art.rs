//! Het album-art-endpoint en de hoesweergave over HTTP, tegen de echte binary.
//!
//! De unit-tests in `src/art.rs` controleren het verkleinen en die in `src/web`
//! de routering; deze test controleert wat er werkelijk over de lijn gaat: de
//! statusregel, de `Content-Type` en de bytes zelf. Daarna de pagina eromheen
//! (FR-12): wat er over een hoes te zien is zonder hem te downloaden.
//!
//! De bibliotheek is een tempdir met kopieën van de fixtures. De echte
//! muziekbibliotheek wordt nooit aangeraakt.

mod common;

use common::{Server, place_fixture};

/// De ingecheckte coverafbeeldingen zijn 300×300.
const FIXTURE_SIZE: usize = 300;

fn library_with_and_without_art() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "met-hoes.mp3", "tagged-with-art.mp3");
    place_fixture(&album, "met-hoes.flac", "tagged-with-art.flac");
    place_fixture(&album, "zonder-hoes.mp3", "tagged.mp3");

    root
}

/// Splitst een respons in de statusregel, de headers en de body als bytes.
///
/// `Server::get` levert de hele respons als tekst aan; voor een afbeelding is
/// dat onbruikbaar, dus hier wordt de body als bytes teruggegeven. De headers
/// zijn ASCII en mogen wel als tekst.
fn parse(response: &[u8]) -> (String, String, Vec<u8>) {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("respons had geen lege regel tussen headers en body"));

    let head = String::from_utf8_lossy(&response[..split]).into_owned();
    let (status, headers) = head
        .split_once("\r\n")
        .map(|(status, rest)| (status.to_string(), rest.to_ascii_lowercase()))
        .unwrap_or((head.clone(), String::new()));

    (status, headers, response[split + 4..].to_vec())
}

/// De afmetingen van een afbeelding, uit de header gelezen.
///
/// De integratietests draaien buiten de binary-crate en kunnen `art::` dus niet
/// aanroepen. JPEG en PNG hebben allebei hun afmetingen op een vaste plek, en
/// meer is hier niet nodig.
fn dimensions(data: &[u8]) -> (usize, usize) {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        let width = u32::from_be_bytes(data[16..20].try_into().expect("PNG-header")) as usize;
        let height = u32::from_be_bytes(data[20..24].try_into().expect("PNG-header")) as usize;
        return (width, height);
    }

    assert!(data.starts_with(&[0xFF, 0xD8]), "geen JPEG en geen PNG");

    // Door de JPEG-segmenten lopen tot de Start-Of-Frame; daarin staan de
    // afmetingen. Elk segment noemt zijn eigen lengte, dus overslaan kan.
    let mut index = 2;
    while index + 9 < data.len() {
        assert_eq!(data[index], 0xFF, "segmentmarkering verwacht");

        let marker = data[index + 1];
        let length = u16::from_be_bytes([data[index + 2], data[index + 3]]) as usize;

        // SOF0 tot en met SOF3; de overige FFC?-markeringen zijn geen frame.
        if (0xC0..=0xC3).contains(&marker) {
            let height = u16::from_be_bytes([data[index + 5], data[index + 6]]) as usize;
            let width = u16::from_be_bytes([data[index + 7], data[index + 8]]) as usize;
            return (width, height);
        }

        index += 2 + length;
    }

    panic!("geen afmetingen gevonden in de JPEG");
}

#[test]
fn the_embedded_cover_is_served_unchanged() {
    let server = Server::start_in(library_with_and_without_art(), &[]);

    for file in ["met-hoes.mp3", "met-hoes.flac"] {
        let (status, headers, body) = parse(&server.get_bytes(&format!("/art/Album/{file}")));

        assert!(status.starts_with("HTTP/1.1 200 OK"), "{file}: {status}");
        assert!(
            headers.contains("content-type: image/jpeg"),
            "{file}: verkeerd content-type in\n{headers}"
        );
        assert_eq!(
            dimensions(&body),
            (FIXTURE_SIZE, FIXTURE_SIZE),
            "{file}: de hoes hoort ongewijzigd terug te komen"
        );
    }
}

#[test]
fn the_thumbnail_variant_is_scaled_down() {
    let server = Server::start_in(library_with_and_without_art(), &[]);

    let (status, headers, body) = parse(&server.get_bytes("/art/Album/met-hoes.mp3?size=thumb"));

    assert!(status.starts_with("HTTP/1.1 200 OK"), "{status}");
    assert!(
        headers.contains("content-type: image/jpeg"),
        "verkeerd content-type in\n{headers}"
    );

    let (width, height) = dimensions(&body);
    assert!(
        width < FIXTURE_SIZE && height < FIXTURE_SIZE,
        "de thumbnail is {width}x{height} en dus niet verkleind"
    );
}

#[test]
fn a_file_without_art_gives_a_404() {
    let server = Server::start_in(library_with_and_without_art(), &[]);

    let (status, _, body) = parse(&server.get_bytes("/art/Album/zonder-hoes.mp3"));

    assert!(status.starts_with("HTTP/1.1 404"), "{status}");
    assert!(
        String::from_utf8_lossy(&body).contains("geen album art"),
        "de melding hoort uit te leggen wat er aan de hand is: {}",
        String::from_utf8_lossy(&body)
    );
}

#[test]
fn art_outside_the_library_is_refused() {
    let server = Server::start_in(library_with_and_without_art(), &[]);

    for attempt in [
        "/art/../../etc/passwd",
        "/art/Album/../../../etc/hosts",
        "/art/..",
    ] {
        let (status, _, _) = parse(&server.get_bytes(attempt));
        assert!(
            !status.starts_with("HTTP/1.1 200"),
            "'{attempt}' leverde een afbeelding op: {status}"
        );
    }
}

#[test]
fn the_listing_asks_for_thumbnails_and_skips_files_without_art() {
    let server = Server::start_in(library_with_and_without_art(), &[]);

    let html = server.get("/map/Album");

    assert!(
        html.contains("src=\"/art/Album/met-hoes.mp3?size=thumb\""),
        "de hoes wordt niet als thumbnail opgevraagd:\n{html}"
    );
    assert!(
        html.contains("loading=\"lazy\""),
        "zonder lazy loading blokkeren de hoezen het renderen:\n{html}"
    );
    assert!(
        !html.contains("src=\"/art/Album/zonder-hoes.mp3"),
        "een bestand zonder hoes hoort geen verzoek uit te lokken:\n{html}"
    );
}

#[test]
fn the_cover_page_shows_format_dimensions_and_size() {
    // AC #5: de eigenschappen die op een thumbnail niet te zien zijn.
    let server = Server::start_in(library_with_and_without_art(), &[]);

    let page = server.get("/hoes/Album/met-hoes.mp3");

    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("JPEG"), "{page}");
    assert!(page.contains("image/jpeg"), "{page}");
    assert!(
        page.contains(&format!("{FIXTURE_SIZE} × {FIXTURE_SIZE} pixels")),
        "{page}"
    );
    assert!(page.contains("kB"), "{page}");

    // En de afbeelding zelf staat erop, op ware grootte.
    assert!(page.contains("src=\"/art/Album/met-hoes.mp3\""), "{page}");
    assert!(!page.contains("size=thumb"), "{page}");
}

#[test]
fn a_file_without_art_says_so_on_its_cover_page() {
    // AC #3: geen 404, maar de mededeling dat er niets in zit.
    let server = Server::start_in(library_with_and_without_art(), &[]);

    let page = server.get("/hoes/Album/zonder-hoes.mp3");

    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("geen embedded hoes"), "{page}");
    assert!(!page.contains("<img"), "{page}");
}

#[test]
fn the_edit_page_leads_to_the_cover_page() {
    let server = Server::start_in(library_with_and_without_art(), &[]);

    for file in ["met-hoes.mp3", "zonder-hoes.mp3"] {
        let page = server.get(&format!("/bewerk/Album/{file}"));
        assert!(
            page.contains(&format!("/hoes/Album/{file}")),
            "{file} verwijst niet naar zijn hoespagina:\n{page}"
        );
    }
}

#[test]
fn a_cover_page_outside_the_library_is_refused() {
    let server = Server::start_in(library_with_and_without_art(), &[]);

    let response = server.get("/hoes/../../etc/passwd");

    assert!(
        response.starts_with("HTTP/1.1 403") || response.starts_with("HTTP/1.1 404"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
}

#[test]
fn different_covers_in_one_folder_are_visible_in_the_listing() {
    // AC #4: de maplijst wijst het aan, zodat je niet elk bestand hoeft te
    // openen om te ontdekken dat de hoezen uiteenlopen.
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "een.mp3", "tagged-with-art.mp3");
    place_fixture(&album, "twee.mp3", "tagged-with-other-art.mp3");

    let server = Server::start_in(root, &[]);
    let html = server.get("/map/Album");

    assert!(
        html.contains("2 verschillende hoezen in deze map"),
        "{html}"
    );
}

#[test]
fn one_cover_for_the_whole_folder_says_nothing() {
    // Dezelfde hoes in MP3 en FLAC mag geen valse melding opleveren.
    let server = Server::start_in(library_with_and_without_art(), &[]);

    let html = server.get("/map/Album");

    assert!(!html.contains("verschillende hoezen"), "{html}");
}

/// De bytes van een ingecheckte coverafbeelding.
fn cover_bytes(name: &str) -> Vec<u8> {
    std::fs::read(common::fixture_path(name)).expect("fixture moet leesbaar zijn")
}

/// De afmetingen van de hoes die nu in dit bestand zit.
fn embedded_cover(server: &Server, file: &str) -> (String, Vec<u8>) {
    let (status, headers, body) = parse(&server.get_bytes(&format!("/art/Album/{file}")));
    assert!(status.starts_with("HTTP/1.1 200 OK"), "{file}: {status}");

    let mime = headers
        .lines()
        .find_map(|line| line.strip_prefix("content-type: "))
        .unwrap_or_default()
        .trim()
        .to_string();

    (mime, body)
}

#[test]
fn a_cover_can_be_uploaded_into_one_file() {
    // AC #1 en #4: embedden in het geopende bestand, en daarna tonen wat er
    // werkelijk in zit.
    let server = Server::start_in(library_with_and_without_art(), &[]);
    let png = cover_bytes("cover.png");

    let page = server.post_multipart(
        "/hoes/Album/zonder-hoes.mp3",
        &[("actie", "embed-dit")],
        Some(("afbeelding", "cover.png", &png)),
    );

    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("1 bestand bijgewerkt."), "{page}");
    assert!(page.contains("Bijgewerkt: Hoes"), "{page}");
    // De opnieuw ingelezen situatie: PNG, en de afmetingen van de fixture.
    assert!(page.contains("PNG"), "{page}");
    assert!(
        page.contains(&format!("{FIXTURE_SIZE} × {FIXTURE_SIZE} pixels")),
        "{page}"
    );

    // En het bestand geeft de afbeelding ook werkelijk terug.
    let (mime, body) = embedded_cover(&server, "zonder-hoes.mp3");
    assert_eq!(mime, "image/png");
    assert_eq!(body, png, "de bytes horen ongewijzigd geëmbed te zijn");
}

#[test]
fn a_cover_can_be_uploaded_into_a_flac_too() {
    let server = Server::start_in(library_with_and_without_art(), &[]);
    let png = cover_bytes("andere-cover.png");

    let page = server.post_multipart(
        "/hoes/Album/met-hoes.flac",
        &[("actie", "embed-dit")],
        Some(("afbeelding", "andere-cover.png", &png)),
    );

    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("500 × 500 pixels"), "{page}");

    let (mime, body) = embedded_cover(&server, "met-hoes.flac");
    assert_eq!(mime, "image/png");
    assert_eq!(body, png);
}

#[test]
fn the_same_cover_can_go_into_every_track_at_once() {
    // AC #2 en #5: in één keer het hele album, met een uitkomst per bestand.
    let server = Server::start_in(library_with_and_without_art(), &[]);
    let png = cover_bytes("andere-cover.png");

    let page = server.post_multipart(
        "/hoes/Album/zonder-hoes.mp3",
        &[("actie", "embed-alle")],
        Some(("afbeelding", "andere-cover.png", &png)),
    );

    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("3 bestanden bijgewerkt."), "{page}");

    for file in ["met-hoes.mp3", "met-hoes.flac", "zonder-hoes.mp3"] {
        assert!(
            page.contains(file),
            "{file} ontbreekt in het rapport:\n{page}"
        );

        let (mime, body) = embedded_cover(&server, file);
        assert_eq!(mime, "image/png", "{file}");
        assert_eq!(body, png, "{file}");
    }
}

#[test]
fn embedding_leaves_the_other_tags_alone() {
    // AC #6.
    let server = Server::start_in(library_with_and_without_art(), &[]);
    let before = server.get("/bewerk/Album/zonder-hoes.mp3");

    server.post_multipart(
        "/hoes/Album/zonder-hoes.mp3",
        &[("actie", "embed-dit")],
        Some(("afbeelding", "cover.jpg", &cover_bytes("cover.jpg"))),
    );

    let after = server.get("/bewerk/Album/zonder-hoes.mp3");

    for value in ["Stilte in D", "De Testartiest", "Fixtures voor Sleeve"] {
        assert!(before.contains(value), "de fixture is veranderd: {value}");
        assert!(after.contains(value), "{value} is verdwenen:\n{after}");
    }
}

#[test]
fn a_cover_can_be_removed_from_one_file_and_from_all_of_them() {
    // AC #3.
    let server = Server::start_in(library_with_and_without_art(), &[]);

    let one = server.post_multipart(
        "/hoes/Album/met-hoes.mp3",
        &[("actie", "verwijder-dit")],
        None,
    );
    assert!(one.starts_with("HTTP/1.1 200 OK"), "{one}");
    assert!(one.contains("Bijgewerkt: Hoes verwijderd"), "{one}");
    assert!(one.contains("geen embedded hoes"), "{one}");

    // De FLAC heeft er nog een; die gaat er in de tweede ronde ook uit.
    let all = server.post_multipart(
        "/hoes/Album/met-hoes.flac",
        &[("actie", "verwijder-alle")],
        None,
    );
    assert!(all.starts_with("HTTP/1.1 200 OK"), "{all}");

    for file in ["met-hoes.mp3", "met-hoes.flac", "zonder-hoes.mp3"] {
        let (status, _, _) = parse(&server.get_bytes(&format!("/art/Album/{file}")));
        assert!(
            status.starts_with("HTTP/1.1 404"),
            "{file} heeft nog een hoes: {status}"
        );
    }

    // Wat niets had, is niet aangeraakt; dat staat er ook.
    assert!(all.contains("Er viel niets te wijzigen"), "{all}");
}

#[test]
fn something_that_is_not_an_image_is_refused_and_changes_nothing() {
    let root = library_with_and_without_art();
    let album = root.path().join("Album");
    let before = std::fs::read(album.join("zonder-hoes.mp3")).expect("bestand moet leesbaar zijn");

    let server = Server::start_in(root, &[]);
    let page = server.post_multipart(
        "/hoes/Album/zonder-hoes.mp3",
        &[("actie", "embed-dit")],
        Some(("afbeelding", "hoes.jpg", b"dit is gewoon tekst")),
    );

    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("Er is niets gewijzigd"), "{page}");
    assert!(page.contains("alleen JPEG en PNG"), "{page}");

    let after = std::fs::read(album.join("zonder-hoes.mp3")).expect("bestand moet leesbaar zijn");
    assert_eq!(after, before, "er is geschreven en dat hoort niet");
}

#[test]
fn a_cover_that_is_too_large_is_scaled_down_before_it_is_embedded() {
    // De grens komt uit MAX_ART_SIZE; de fixture is 500×500.
    let server = Server::start_in(library_with_and_without_art(), &[("MAX_ART_SIZE", "200")]);

    let page = server.post_multipart(
        "/hoes/Album/zonder-hoes.mp3",
        &[("actie", "embed-dit")],
        Some((
            "afbeelding",
            "andere-cover.png",
            &cover_bytes("andere-cover.png"),
        )),
    );

    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(
        page.contains("verkleind van 500 × 500 naar 200 × 200 pixels"),
        "{page}"
    );

    let (mime, body) = embedded_cover(&server, "zonder-hoes.mp3");
    // Verkleind én zonder doorzichtigheid, dus JPEG.
    assert_eq!(mime, "image/jpeg");
    assert_eq!(dimensions(&body), (200, 200));
}

#[test]
fn the_cover_page_offers_the_upload_form() {
    let server = Server::start_in(library_with_and_without_art(), &[]);

    let page = server.get("/hoes/Album/zonder-hoes.mp3");

    assert!(page.contains("multipart/form-data"), "{page}");
    assert!(page.contains("name=\"afbeelding\""), "{page}");
    assert!(page.contains("value=\"embed-dit\""), "{page}");
    assert!(page.contains("value=\"embed-alle\""), "{page}");
    // Zonder hoes valt er niets te verwijderen.
    assert!(!page.contains("value=\"verwijder-dit\""), "{page}");
}
