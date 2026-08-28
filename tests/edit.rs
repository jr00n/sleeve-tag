//! Het bewerkformulier over HTTP, tegen de echte binary (FR-5 en FR-6).
//!
//! Dit is de enige test die de hele keten aflegt: formulier invullen, POST,
//! atomisch schrijven, en teruglezen uit het bestand op schijf. De unit-tests
//! dekken de onderdelen; deze test dekt dat ze samen werken.
//!
//! De bibliotheek is een tempdir met kopieën van de fixtures. De echte
//! muziekbibliotheek wordt nooit aangeraakt.

mod common;

use std::path::{Path, PathBuf};

use common::{Server, place_fixture};

/// Bouwt een bibliotheek met één album en geeft de tempdir terug.
fn library_with_a_track() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "track.mp3", "tagged.mp3");
    place_fixture(&album, "track.flac", "tagged.flac");

    root
}

/// De velden zoals de browser ze verstuurt: allemaal, ook de ongewijzigde.
fn fields<'a>(title: &'a str, track: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("title", title),
        ("artist", "Bijgewerkte artiest"),
        ("album_artist", "Bijgewerkte albumartiest"),
        ("album", "Bijgewerkt album"),
        ("track", track),
        ("track_total", "12"),
        ("disc", "1"),
        ("disc_total", "2"),
        ("year", "2024"),
        ("genre", "Ambient"),
        ("composer", "De Componist"),
        ("comment", "Een commentaar"),
    ]
}

fn assert_ok(response: &str) {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
}

/// De titel zoals `ffprobe` hem in het bestand ziet.
///
/// Onafhankelijk van Sleeve zelf: dat de app terugleest wat ze schreef zegt
/// niets als ze allebei dezelfde fout maken. Ontbreekt ffprobe, dan geeft dit
/// `None` en slaat de aanroeper de controle over.
fn title_according_to_ffprobe(path: &Path) -> Option<String> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-show_entries",
            "format_tags=title",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[test]
fn the_form_opens_with_the_values_from_the_file() {
    let server = Server::start_in(library_with_a_track(), &[]);
    let response = server.get("/bewerk/Album/track.mp3");
    assert_ok(&response);

    for expected in [
        r#"value="Stilte in D""#,
        r#"value="De Testartiest""#,
        r#"value="De Albumartiest""#,
        r#"value="Fixtures voor Sleeve""#,
        r#"value="3""#,
    ] {
        assert!(
            response.contains(expected),
            "'{expected}' ontbreekt in het formulier:\n{response}"
        );
    }
}

#[test]
fn saving_changes_the_file_and_shows_what_came_back() {
    let root = library_with_a_track();
    let mp3: PathBuf = root.path().join("Album").join("track.mp3");
    let server = Server::start_in(root, &[]);

    let response = server.post_form("/bewerk/Album/track.mp3", &fields("Nieuwe titel", "7"));
    assert_ok(&response);

    // De bevestiging, en de teruggelezen waarden in het formulier.
    assert!(
        response.contains("Opgeslagen"),
        "er is geen bevestiging:\n{response}"
    );
    assert!(
        response.contains(r#"value="Nieuwe titel""#),
        "de nieuwe titel staat niet in het formulier:\n{response}"
    );
    assert!(
        response.contains(r#"value="7""#),
        "het nieuwe tracknummer staat niet in het formulier:\n{response}"
    );

    // En het bestand op schijf draagt hem werkelijk — gecontroleerd met een
    // tool die niets met Sleeve te maken heeft.
    match title_according_to_ffprobe(&mp3) {
        Some(title) => assert_eq!(title, "Nieuwe titel"),
        None => eprintln!("ffprobe ontbreekt; de onafhankelijke controle is overgeslagen"),
    }
}

#[test]
fn a_flac_can_be_edited_too() {
    let root = library_with_a_track();
    let flac = root.path().join("Album").join("track.flac");
    let server = Server::start_in(root, &[]);

    let response = server.post_form(
        "/bewerk/Album/track.flac",
        &fields("Titel met accenten: Sigur Rós", "9"),
    );
    assert_ok(&response);

    assert!(
        response.contains("Sigur R"),
        "de nieuwe titel staat niet in het formulier:\n{response}"
    );

    if let Some(title) = title_according_to_ffprobe(&flac) {
        assert_eq!(title, "Titel met accenten: Sigur Rós");
    }
}

#[test]
fn an_emptied_field_removes_the_tag() {
    let root = library_with_a_track();
    let server = Server::start_in(root, &[]);

    let mut without_composer = fields("Stilte in D", "3");
    without_composer.retain(|(name, _)| *name != "composer");
    without_composer.push(("composer", ""));

    let response = server.post_form("/bewerk/Album/track.mp3", &without_composer);
    assert_ok(&response);

    // De waarde komt niet meer uit het bestand terug. Het formulier toont wat
    // er ná het schrijven in staat, dus als de componist er nog stond, zou hij
    // hier weer opduiken.
    assert!(
        !response.contains("De Componist"),
        "de componist staat er nog:\n{response}"
    );
    assert!(
        response.contains("Opgeslagen"),
        "er is geen bevestiging:\n{response}"
    );
}

#[test]
fn invalid_input_changes_nothing() {
    let root = library_with_a_track();
    let mp3 = root.path().join("Album").join("track.mp3");
    let before = std::fs::read(&mp3).expect("lezen");
    let server = Server::start_in(root, &[]);

    let response = server.post_form(
        "/bewerk/Album/track.mp3",
        &fields("Deze titel wordt niet opgeslagen", "drie"),
    );
    assert_ok(&response);

    assert!(
        response.contains("Tracknummer moet een getal"),
        "de fout wordt niet uitgelegd:\n{response}"
    );
    assert!(
        response.contains("Deze titel wordt niet opgeslagen"),
        "de ingevulde waarden zijn kwijt:\n{response}"
    );
    assert_eq!(
        std::fs::read(&mp3).expect("lezen"),
        before,
        "er is geschreven ondanks ongeldige invoer"
    );
}

#[test]
fn saving_twice_leaves_the_file_alone_the_second_time() {
    // Er wordt bewust niet doorverwezen na het opslaan, dus een herlaadactie
    // stuurt hetzelfde formulier nog eens. Dat hoort ongevaarlijk te zijn.
    let root = library_with_a_track();
    let mp3 = root.path().join("Album").join("track.mp3");
    let server = Server::start_in(root, &[]);

    let form = fields("Twee keer hetzelfde", "5");
    assert_ok(&server.post_form("/bewerk/Album/track.mp3", &form));
    let after_first = std::fs::read(&mp3).expect("lezen");

    assert_ok(&server.post_form("/bewerk/Album/track.mp3", &form));

    assert_eq!(
        std::fs::read(&mp3).expect("lezen"),
        after_first,
        "de tweede keer versturen heeft het bestand toch aangeraakt"
    );
}

#[test]
fn a_backup_appears_only_when_configured() {
    let root = library_with_a_track();
    let album = root.path().join("Album");
    let server = Server::start_in(root, &[("BACKUP_ON_WRITE", "true")]);

    assert_ok(&server.post_form("/bewerk/Album/track.mp3", &fields("Met backup", "4")));

    assert!(
        album.join("track.mp3.bak").is_file(),
        "er staat geen backup naast het bestand: {:?}",
        std::fs::read_dir(&album)
            .expect("map")
            .map(|entry| entry.expect("entry").file_name())
            .collect::<Vec<_>>()
    );
}

#[test]
fn editing_outside_the_library_is_refused() {
    let server = Server::start_in(library_with_a_track(), &[]);

    for attempt in ["/bewerk/../../etc/passwd", "/bewerk/Album/bestaat-niet.mp3"] {
        let response = server.get(attempt);
        assert!(
            !response.starts_with("HTTP/1.1 200"),
            "'{attempt}' leverde een formulier op:\n{response}"
        );
    }

    let posted = server.post_form("/bewerk/../../etc/passwd", &fields("x", "1"));
    assert!(
        !posted.starts_with("HTTP/1.1 200"),
        "een POST buiten de bibliotheek werd geaccepteerd:\n{posted}"
    );
}

#[test]
fn the_cover_on_the_edit_page_is_a_drop_target() {
    // Een hoes hoort neergezet te kunnen worden waar hij te zien is. Het
    // formulier eromheen post naar dezelfde route als de hoespagina — er komt
    // geen tweede manier bij om een hoes te schrijven.
    let server = Server::start_in(library_with_a_track(), &[]);
    let page = server.get("/bewerk/Album/track.mp3");

    assert!(page.contains("data-neerzetvak"), "pagina was:\n{page}");
    assert!(
        page.contains(r#"action="/hoes/Album/track.mp3""#),
        "het hoesformulier post niet naar de hoespagina:\n{page}"
    );
    assert!(
        page.contains(r#"value="embed-dit""#),
        "de knop ontbreekt:\n{page}"
    );

    // Neerzetten schrijft niets: de knop staat verborgen tot er werkelijk een
    // afbeelding klaarstaat.
    let klaar = page
        .lines()
        .find(|line| line.contains("data-neerzetvak-klaar"))
        .expect("het blok met de knop hoort in de pagina te staan");
    assert!(
        klaar.contains("hidden"),
        "de knop hoort verborgen te beginnen: {klaar}"
    );

    // En de hoespagina blijft bereikbaar voor wat daar meer kan.
    assert!(
        page.contains("de hoespagina"),
        "de verwijzing naar de hoespagina ontbreekt:\n{page}"
    );
}

#[test]
fn a_file_without_art_is_a_drop_target_too() {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");
    common::place_fixture(&album, "kaal.mp3", "untagged.mp3");

    let server = Server::start_in(root, &[]);
    let page = server.get("/bewerk/Album/kaal.mp3");

    assert!(
        page.contains("data-neerzetvak"),
        "juist een bestand zonder hoes wil je er een op kunnen slepen:\n{page}"
    );
}

#[test]
fn the_cover_form_does_not_carry_the_tag_fields() {
    // Geneste formulieren bestaan niet in HTML, en een hoesactie hoort geen
    // tags mee te sturen. Het hoesformulier hoort dus dicht te zijn vóór het
    // tagformulier begint.
    let server = Server::start_in(library_with_a_track(), &[]);
    let page = server.get("/bewerk/Album/track.mp3");

    let hoesform = page
        .find(r#"action="/hoes/Album/track.mp3""#)
        .expect("het hoesformulier hoort er te zijn");
    let einde_hoesform = page[hoesform..]
        .find("</form>")
        .expect("het hoesformulier hoort afgesloten te worden")
        + hoesform;
    let tagform = page
        .find(r#"action="/bewerk/Album/track.mp3""#)
        .expect("het tagformulier hoort er te zijn");

    assert!(
        einde_hoesform < tagform,
        "het hoesformulier omsluit het tagformulier; dan zouden de tags meegaan"
    );
}

#[test]
fn the_cover_form_is_sent_in_the_background() {
    // Navigeren zou de tagvelden wegvagen die de gebruiker misschien net heeft
    // ingevuld maar nog niet heeft opgeslagen. Vandaar dat dit formulier op de
    // achtergrond gaat; de markering is wat `app.js` daarop afgaat.
    let server = Server::start_in(library_with_a_track(), &[]);
    let page = server.get("/bewerk/Album/track.mp3");

    assert!(page.contains("data-inplace"), "pagina was:\n{page}");

    // Het adres van de hoes gaat mee, zodat het hoesje zich kan verversen
    // zonder de pagina te herladen.
    assert!(
        page.contains(r#"data-art-url="/art/Album/track.mp3""#),
        "het adres van de hoes ontbreekt:\n{page}"
    );
}

#[test]
fn embedding_from_the_edit_page_still_works_without_javascript() {
    // De terugvaloptie: zonder JavaScript post het formulier gewoon, en dan
    // komt de gebruiker op de hoespagina uit met het volledige rapport. Dat is
    // dezelfde route die de achtergrondversie gebruikt — er is er maar één.
    let root = library_with_a_track();
    let track = root.path().join("Album").join("track.mp3");
    let server = Server::start_in(root, &[]);

    let cover = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("cover.jpg"),
    )
    .expect("de fixture moet leesbaar zijn");

    let response = server.post_multipart(
        "/hoes/Album/track.mp3",
        &[("actie", "embed-dit")],
        Some(("afbeelding", "cover.jpg", &cover)),
    );

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
    assert!(
        response.contains("resultaat__uitkomst"),
        "het rapport ontbreekt:\n{response}"
    );

    // En de hoes zit er werkelijk in.
    let bytes = std::fs::read(&track).expect("het bestand moet leesbaar zijn");
    assert!(
        bytes
            .windows(4)
            .any(|window| window == [0xFF, 0xD8, 0xFF, 0xE0]),
        "er zit geen JPEG in het bestand"
    );
}

#[test]
fn the_way_back_leads_where_the_user_came_from() {
    // Wie uit de albumweergave komt, heeft daar net een selectie gemaakt en wil
    // daarheen terug — niet naar de kale maplijst.
    let server = Server::start_in(library_with_a_track(), &[]);

    let vanuit_album = server.get("/bewerk/Album/track.mp3?terug=album");
    assert!(
        vanuit_album.contains(r#"href="/album/Album""#),
        "de weg terug wijst niet naar de albumweergave:\n{vanuit_album}"
    );
    assert!(
        vanuit_album.contains("Terug naar de albumweergave"),
        "het opschrift klopt niet:\n{vanuit_album}"
    );

    // En zonder die herkomst blijft alles zoals het was.
    let vanuit_map = server.get("/bewerk/Album/track.mp3");
    assert!(
        vanuit_map.contains(r#"href="/map/Album""#),
        "de weg terug wijst niet naar de map:\n{vanuit_map}"
    );
    assert!(
        vanuit_map.contains("Terug naar de map"),
        "het opschrift klopt niet:\n{vanuit_map}"
    );
}

#[test]
fn the_way_back_survives_a_save() {
    // Het formulier post naar dezelfde URL, dus de herkomst hoort daarin te
    // blijven staan; anders sta je na het opslaan ineens op de maplijst.
    let server = Server::start_in(library_with_a_track(), &[]);

    let page = server.get("/bewerk/Album/track.mp3?terug=album");
    assert!(
        page.contains(r#"action="/bewerk/Album/track.mp3?terug=album""#),
        "het formulier verliest de herkomst:\n{page}"
    );

    let saved = server.post_form(
        "/bewerk/Album/track.mp3?terug=album",
        &fields("Nieuwe titel", "3"),
    );
    assert!(
        saved.contains("Terug naar de albumweergave"),
        "na het opslaan is de weg terug kwijt:\n{saved}"
    );
}

#[test]
fn the_album_view_links_back_to_itself() {
    // De link in de albumtabel draagt de herkomst mee; anders weet het
    // bewerkformulier niet waar het vandaan komt.
    let server = Server::start_in(library_with_a_track(), &[]);
    let page = server.get("/album/Album");

    assert!(
        page.contains("/bewerk/Album/track.mp3?terug=album"),
        "de albumweergave wijst niet terug naar zichzelf:\n{page}"
    );
}
