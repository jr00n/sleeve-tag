//! De mapbrowser over HTTP, tegen de echte binary (FR-1 t/m FR-3).
//!
//! De unit-tests in `src/browse.rs` controleren het weergavemodel; deze test
//! controleert wat er werkelijk in de browser terechtkomt: de velden in de HTML,
//! de volgorde van de regels en het filter uit de querystring.
//!
//! De bibliotheek is een tempdir met kopieën van de fixtures. De echte
//! muziekbibliotheek wordt nooit aangeraakt.

mod common;

use std::path::PathBuf;

use common::{Server, place_fixture};

/// De waarden die in `tests/fixtures/genereer-fixtures.sh` in de getagde
/// fixtures zijn gezet.
const TITLE: &str = "Stilte in D";
const ARTIST: &str = "De Testartiest";
const ALBUM: &str = "Fixtures voor Sleeve";
const TRACK_NUMBER: &str = "3";

/// Bouwt een bibliotheek met één album en geeft de tempdir terug.
///
/// De bestandsnamen staan bewust dwars op de tracknummers: `zzz-getagd.mp3`
/// heeft tracknummer 3 en hoort daarom vóór de twee bestanden zonder
/// tracknummer te staan, ook al sorteert zijn naam als laatste.
fn library_with_album() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Artiest").join("Het Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "zzz-getagd.mp3", "tagged.mp3");
    place_fixture(&album, "aaa-kaal.flac", "untagged.flac");
    place_fixture(&album, "mmm-kaal.mp3", "untagged.mp3");

    // Niet-audio en een bestand dat alleen zo heet: geen van beide is een
    // bewerkbaar bestand en geen van beide hoort in de lijst.
    place_fixture(&album, "folder.jpg", "cover.jpg");
    place_fixture(&album, "nep.mp3", "cover.png");
    std::fs::write(album.join("notities.txt"), b"tekst").expect("bestand moet te schrijven zijn");

    root
}

/// Alleen de body van een respons, zodat headers geen assertie kunnen redden.
fn body(response: &str) -> String {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_else(|| panic!("respons had geen body:\n{response}"))
}

/// De positie van een tekst in de pagina; paniekt als hij ontbreekt.
fn position(html: &str, needle: &str) -> usize {
    html.find(needle)
        .unwrap_or_else(|| panic!("'{needle}' staat niet op de pagina:\n{html}"))
}

fn assert_ok(response: &str) {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
}

#[test]
fn a_directory_shows_its_files_with_the_main_tags() {
    let server = Server::start_in(library_with_album(), &[]);
    let response = server.get("/map/Artiest/Het%20Album");
    assert_ok(&response);

    let html = body(&response);

    // FR-2: tracknummer, titel, artiest, album, duur en formaat.
    for expected in [
        TRACK_NUMBER,
        TITLE,
        ARTIST,
        ALBUM,
        "0:01", // de fixtures zijn één seconde stilte
        "MP3",
        "FLAC",
    ] {
        assert!(
            html.contains(expected),
            "'{expected}' ontbreekt op de pagina:\n{html}"
        );
    }

    // De drie audiobestanden staan er; de rest niet.
    for name in ["zzz-getagd.mp3", "aaa-kaal.flac", "mmm-kaal.mp3"] {
        assert!(html.contains(name), "'{name}' ontbreekt:\n{html}");
    }
    for name in ["folder.jpg", "notities.txt", "nep.mp3"] {
        assert!(
            !html.contains(name),
            "'{name}' is geen bewerkbaar bestand en hoort niet in de lijst:\n{html}"
        );
    }
}

#[test]
fn files_are_sorted_by_track_number_then_by_filename() {
    let server = Server::start_in(library_with_album(), &[]);
    let html = body(&server.get("/map/Artiest/Het%20Album"));

    let tagged = position(&html, "zzz-getagd.mp3");
    let first_untagged = position(&html, "aaa-kaal.flac");
    let second_untagged = position(&html, "mmm-kaal.mp3");

    assert!(
        tagged < first_untagged,
        "een bestand mét tracknummer hoort vooraan te staan:\n{html}"
    );
    assert!(
        first_untagged < second_untagged,
        "bestanden zonder tracknummer horen op naam te staan:\n{html}"
    );
}

#[test]
fn the_root_lists_the_top_level_directories() {
    let server = Server::start_in(library_with_album(), &[]);
    let html = body(&server.get("/"));

    assert!(html.contains("Artiest"), "de artiestmap ontbreekt:\n{html}");
    assert!(
        html.contains("/map/Artiest"),
        "er is geen link naar de artiestmap:\n{html}"
    );
}

#[test]
fn breadcrumbs_lead_back_to_the_root() {
    let server = Server::start_in(library_with_album(), &[]);
    let html = body(&server.get("/map/Artiest/Het%20Album"));

    assert!(
        html.contains("Bibliotheek"),
        "het broodkruimelpad begint niet bij de bibliotheek:\n{html}"
    );
    assert!(
        html.contains(r#"href="/map/Artiest""#),
        "er is geen weg terug naar de bovenliggende map:\n{html}"
    );
}

#[test]
fn filtering_works_on_the_title_and_on_the_filename() {
    let server = Server::start_in(library_with_album(), &[]);

    // Op titel: de zoekterm komt niet in de bestandsnaam voor.
    let by_title = body(&server.get("/map/Artiest/Het%20Album?q=stilte"));
    assert!(
        by_title.contains("zzz-getagd.mp3"),
        "filteren op titel vond het bestand niet:\n{by_title}"
    );
    assert!(
        !by_title.contains("aaa-kaal.flac"),
        "een bestand zonder overeenkomst bleef staan:\n{by_title}"
    );

    // Op bestandsnaam: de zoekterm komt in geen enkele tag voor.
    let by_name = body(&server.get("/map/Artiest/Het%20Album?q=aaa"));
    assert!(
        by_name.contains("aaa-kaal.flac"),
        "filteren op bestandsnaam vond het bestand niet:\n{by_name}"
    );
    assert!(
        !by_name.contains("zzz-getagd.mp3"),
        "een bestand zonder overeenkomst bleef staan:\n{by_name}"
    );

    // Niets gevonden is een lege lijst met uitleg, geen fout.
    let response = server.get("/map/Artiest/Het%20Album?q=bestaatniet");
    assert_ok(&response);
    assert!(
        body(&response).contains("Niets in deze map"),
        "een leeg resultaat hoort uitgelegd te worden:\n{response}"
    );
}

#[test]
fn htmx_receives_only_the_list() {
    let server = Server::start_in(library_with_album(), &[]);

    // Het fragment dat HTMX terugkrijgt is dezelfde lijst zonder de omhullende
    // pagina; anders zou de pagina zichzelf in zichzelf nesten.
    let fragment = server.get_with_headers("/map/Artiest/Het%20Album", &[("HX-Request", "true")]);
    assert_ok(&fragment);

    let html = body(&fragment);
    assert!(
        html.contains("zzz-getagd.mp3"),
        "de lijst ontbreekt:\n{html}"
    );
    assert!(
        !html.contains("<!DOCTYPE html>"),
        "HTMX kreeg de hele pagina in plaats van alleen de lijst:\n{html}"
    );
}

#[test]
fn navigating_above_the_root_is_refused() {
    let server = Server::start_in(library_with_album(), &[]);

    for attempt in [
        "/map/../../etc/passwd",
        "/map/Artiest/../../../etc/hosts",
        "/map/..",
    ] {
        let response = server.get(attempt);
        assert!(
            !response.starts_with("HTTP/1.1 200 OK"),
            "'{attempt}' leverde een pagina op:\n{response}"
        );
    }

    // Een map die niet bestaat is een dode link, geen geweigerd verzoek.
    let missing = server.get("/map/Artiest/Bestaat%20Niet");
    assert!(
        missing.starts_with("HTTP/1.1 404"),
        "antwoord begon met: {}",
        missing.lines().next().unwrap_or_default()
    );
}

#[test]
fn the_listing_points_out_what_is_wrong() {
    // De signalering uit FR-4. De afwijkende-waarden-kant (twee albumtitels in
    // één map) is met de ingecheckte fixtures niet te maken — daarvoor moeten
    // er tags geschreven worden — en wordt in `checks::tests` gedekt. Hier
    // gaat het om wat er werkelijk op de pagina belandt.
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Rommelig album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    // Beide getagde fixtures hebben tracknummer 3: een dubbel nummer.
    place_fixture(&album, "een.mp3", "tagged.mp3");
    place_fixture(&album, "twee.flac", "tagged.flac");
    // En één bestand zonder enige tag.
    place_fixture(&album, "drie.mp3", "untagged.mp3");

    let server = Server::start_in(root, &[]);
    let html = body(&server.get("/map/Rommelig%20album"));

    // Op mapniveau (AC #2, #3).
    assert!(
        html.contains("Let op in deze map"),
        "de mapmeldingen ontbreken:\n{html}"
    );
    for expected in [
        "tracknummer 3 komt meer dan eens voor",
        "1 bestand heeft geen tracknummer",
    ] {
        assert!(
            html.contains(expected),
            "'{expected}' ontbreekt op de pagina:\n{html}"
        );
    }

    // Per bestand (AC #1, #4): zichtbare tekst, geen tooltip-alleen.
    for expected in [
        "geen titel",
        "geen artiest",
        "geen album",
        "geen hoes",
        "geen tracknummer",
        "dubbel tracknummer",
    ] {
        assert!(
            html.contains(expected),
            "'{expected}' ontbreekt op de pagina:\n{html}"
        );
    }
}

#[test]
fn a_tidy_directory_shows_no_warnings() {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Net album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");
    place_fixture(&album, "enige.mp3", "tagged-with-art.mp3");

    let server = Server::start_in(root, &[]);
    let html = body(&server.get("/map/Net%20album"));

    assert!(
        !html.contains("Let op in deze map"),
        "een nette map hoort geen waarschuwing te krijgen:\n{html}"
    );
    assert!(
        !html.contains("signaal"),
        "een compleet getagd bestand hoort geen label te krijgen:\n{html}"
    );
}

/// Een map met één net bestand en twee waar van alles aan mankeert.
fn library_with_a_mixed_album() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Gemengd album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "net.mp3", "tagged-with-art.mp3");
    place_fixture(&album, "kaal-een.mp3", "untagged.mp3");
    place_fixture(&album, "kaal-twee.flac", "untagged.flac");

    root
}

#[test]
fn the_directory_counts_what_needs_attention() {
    let server = Server::start_in(library_with_a_mixed_album(), &[]);
    let html = body(&server.get("/map/Gemengd%20album"));

    assert!(
        html.contains("Vraagt aandacht"),
        "de telling ontbreekt in de kopbalk van de map:\n{html}"
    );
    assert!(
        html.contains(r#"class="filterknop__telling">2<"#),
        "twee van de drie bestanden vragen aandacht:\n{html}"
    );

    // Zonder JavaScript: een gewone link naar dezelfde pagina met de stand in
    // de URL, zodat een gefilterde lijst te delen en te bookmarken is.
    assert!(
        html.contains(r#"href="/map/Gemengd%20album?aandacht=1""#),
        "het filter hoort een gewone link met de stand in de URL te zijn:\n{html}"
    );
}

#[test]
fn the_attention_filter_narrows_the_list_and_switches_back() {
    let server = Server::start_in(library_with_a_mixed_album(), &[]);

    let filtered = body(&server.get("/map/Gemengd%20album?aandacht=1"));
    for name in ["kaal-een.mp3", "kaal-twee.flac"] {
        assert!(
            filtered.contains(name),
            "'{name}' vraagt aandacht en hoort te blijven staan:\n{filtered}"
        );
    }
    assert!(
        !filtered.contains("net.mp3"),
        "een bestand zonder signalering hoort weg te vallen:\n{filtered}"
    );

    // De telling blijft over de hele map gaan, ook met het filter aan.
    assert!(
        filtered.contains(r#"class="filterknop__telling">2<"#),
        "de telling hoort bij de map, niet bij wat er overblijft:\n{filtered}"
    );

    // En de knop laat zien dat hij aan staat, met de weg terug erin.
    assert!(
        filtered.contains("filterknop--aan"),
        "de knop hoort te tonen dat het filter aan staat:\n{filtered}"
    );
    assert!(
        filtered.contains(r#"href="/map/Gemengd%20album""#),
        "nog een klik hoort de hele lijst terug te brengen:\n{filtered}"
    );

    // Diezelfde URL zonder de parameter toont weer alles.
    let everything = body(&server.get("/map/Gemengd%20album"));
    assert!(
        everything.contains("net.mp3"),
        "zonder filter hoort alles er te staan:\n{everything}"
    );
}

#[test]
fn the_attention_filter_and_the_search_term_narrow_together() {
    let server = Server::start_in(library_with_a_mixed_album(), &[]);

    // Allebei tegelijk: alleen wat aan beide voldoet blijft over.
    let both = body(&server.get("/map/Gemengd%20album?q=twee&aandacht=1"));
    assert!(
        both.contains("kaal-twee.flac"),
        "het bestand dat aan allebei voldoet ontbreekt:\n{both}"
    );
    for name in ["kaal-een.mp3", "net.mp3"] {
        assert!(
            !both.contains(name),
            "'{name}' voldoet niet aan allebei en hoort weg te vallen:\n{both}"
        );
    }

    // Het zoekveld houdt de stand vast, anders zou zoeken het filter uitzetten.
    assert!(
        both.contains(r#"name="aandacht" value="1""#),
        "het zoekformulier hoort het filter mee te sturen:\n{both}"
    );
    assert!(
        both.contains(r#"href="/map/Gemengd%20album?q=twee""#),
        "de knop hoort de zoekterm te bewaren bij het uitzetten:\n{both}"
    );

    // Geen OR: een zoekterm die alleen het nette bestand vindt, houdt met het
    // aandachtsfilter erbij niets over — met uitleg in plaats van een lege lijst.
    let none = body(&server.get("/map/Gemengd%20album?q=net&aandacht=1"));
    assert!(
        !none.contains("net.mp3"),
        "het aandachtsfilter hoort het nette bestand tegen te houden:\n{none}"
    );
    assert!(
        none.contains("Niets wat aandacht vraagt komt overeen met"),
        "een leeg resultaat hoort uitgelegd te worden:\n{none}"
    );
}

#[test]
fn a_directory_where_everything_is_in_order_says_so() {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Net album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");
    place_fixture(&album, "enige.mp3", "tagged-with-art.mp3");

    let server = Server::start_in(root, &[]);

    // Zonder filter: geen knop die naar een lege lijst leidt, maar het bericht
    // dat er niets te doen is.
    let html = body(&server.get("/map/Net%20album"));
    assert!(
        html.contains("Niets in deze map vraagt aandacht."),
        "een nette map hoort dat met zoveel woorden te zeggen:\n{html}"
    );
    assert!(
        !html.contains("filterknop"),
        "zonder iets om op te filteren hoort er geen knop te staan:\n{html}"
    );

    // En wie er tóch met het filter aan binnenkomt, krijgt uitleg in plaats
    // van een lege lijst.
    let filtered = body(&server.get("/map/Net%20album?aandacht=1"));
    assert!(
        filtered.contains("Niets in deze map vraagt aandacht."),
        "ook met het filter aan hoort er uitleg te staan:\n{filtered}"
    );
    assert!(
        !filtered.contains("enige.mp3"),
        "het filter hoort ook hier te filteren:\n{filtered}"
    );
}

#[test]
fn marking_leaves_the_files_untouched() {
    // AC #5: de signalering is puur informatief. Na het bekijken van een map
    // met van alles mis moeten de bestanden byte voor byte gelijk zijn.
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Rommelig album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "een.mp3", "tagged.mp3");
    place_fixture(&album, "twee.flac", "tagged.flac");
    place_fixture(&album, "drie.mp3", "untagged.mp3");

    let before = fingerprint(&album);

    let server = Server::start_in(root, &[]);
    let _ = server.get("/map/Rommelig%20album");
    let _ = server.get("/map/Rommelig%20album?q=een");

    assert_eq!(
        fingerprint(&album),
        before,
        "de bestanden zijn veranderd door ze te bekijken"
    );
}

/// De naam, grootte en volledige inhoud van elk bestand in een map.
fn fingerprint(directory: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(directory)
        .expect("map moet leesbaar zijn")
        .map(|entry| {
            let path = entry.expect("map-entry moet leesbaar zijn").path();
            let name = path
                .file_name()
                .expect("bestandsnaam")
                .to_string_lossy()
                .into_owned();
            (
                name,
                std::fs::read(&path).expect("bestand moet leesbaar zijn"),
            )
        })
        .collect();

    files.sort();
    files
}

#[test]
fn a_directory_with_thirty_tracks_renders_quickly() {
    // PRD §8.5 eist < 1 s voor dertig tracks. De maatgevende meting gebeurt op
    // de NAS (task-27); deze test bewaakt dat er per bestand niet meer werk
    // wordt gedaan dan één keer openen en lezen.
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album: PathBuf = root.path().join("Groot album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    for number in 1..=30 {
        place_fixture(&album, &format!("track-{number:02}.mp3"), "tagged.mp3");
    }

    let server = Server::start_in(root, &[]);

    let start = std::time::Instant::now();
    let response = server.get("/map/Groot%20album");
    let elapsed = start.elapsed();

    assert_ok(&response);
    let html = body(&response);
    for number in 1..=30 {
        let name = format!("track-{number:02}.mp3");
        assert!(html.contains(&name), "'{name}' ontbreekt in de lijst");
    }

    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "dertig tracks kostten {elapsed:?}"
    );
}

#[test]
fn the_page_stays_within_the_library() {
    // Wat er ook op de pagina staat, het absolute pad van de NAS hoort er niet
    // bij te staan: de gebruiker werkt met paden relatief aan MUSIC_ROOT.
    let root = library_with_album();
    let absolute = std::fs::canonicalize(root.path())
        .expect("root moet te canonicaliseren zijn")
        .display()
        .to_string();

    let server = Server::start_in(root, &[]);
    let html = body(&server.get("/map/Artiest/Het%20Album"));

    assert!(
        !html.contains(&absolute),
        "het absolute pad van de bibliotheek staat op de pagina:\n{html}"
    );
}

/// Bouwt een bibliotheek waarin elk soort kaart voorkomt (task-37).
///
/// `Artiest` bevat één album met drie bewerkbare bestanden in twee formaten en
/// wat rommel die niet meetelt; `Zonder muziek` is leeg.
fn library_with_folders() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Artiest").join("Het Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

    place_fixture(&album, "een.mp3", "tagged.mp3");
    place_fixture(&album, "twee.mp3", "untagged.mp3");
    place_fixture(&album, "drie.flac", "tagged.flac");

    place_fixture(&album, "folder.jpg", "cover.jpg");
    std::fs::write(album.join("notities.txt"), b"tekst").expect("bestand moet te schrijven zijn");

    std::fs::create_dir(root.path().join("Zonder muziek")).expect("map moet aan te maken zijn");

    root
}

/// Het stuk HTML van één kaart, van de link tot en met het sluiten ervan.
fn card(html: &str, name: &str) -> String {
    let start = html
        .find("<ul class=\"mapkaarten\"")
        .unwrap_or_else(|| panic!("er staat geen kaartenraster op de pagina:\n{html}"));

    html[start..]
        .split("<a class=\"mapkaart\"")
        .skip(1)
        .map(|blok| {
            let einde = blok.find("</a>").unwrap_or(blok.len());
            blok[..einde].to_string()
        })
        .find(|blok| blok.contains(name))
        .unwrap_or_else(|| panic!("er is geen kaart voor '{name}':\n{html}"))
}

#[test]
fn the_library_shows_its_folders_as_cards() {
    let server = Server::start_in(library_with_folders(), &[]);
    let html = body(&server.get("/map/Artiest"));

    // AC #2 en AC #4: de kaart noemt het aantal en de formaten, en de hele
    // kaart is de link naar de map.
    let album = card(&html, "Het Album");

    assert!(
        album.contains("href=\"/map/Artiest/Het%20Album\""),
        "de kaart leidt niet naar de map:\n{album}"
    );
    assert!(
        album.contains("3 bestanden"),
        "de kaart noemt het aantal bewerkbare bestanden niet:\n{album}"
    );
    assert!(
        album.contains(">MP3<") && album.contains(">FLAC<"),
        "de kaart noemt de formaten niet:\n{album}"
    );
}

#[test]
fn a_card_of_a_folder_without_files_shows_no_count() {
    let server = Server::start_in(library_with_folders(), &[]);
    let html = body(&server.get("/"));

    // AC #3: een map met alleen submappen telt die submappen, en een lege map
    // zegt dat er niets te bewerken valt. Geen van beide toont "0 bestanden".
    let artist = card(&html, "Artiest");
    assert!(
        artist.contains("1 submap"),
        "de artiestmap noemt zijn albums niet:\n{artist}"
    );

    let empty = card(&html, "Zonder muziek");
    assert!(
        empty.contains("Geen bewerkbare bestanden"),
        "de lege map zegt niet dat er niets te bewerken valt:\n{empty}"
    );

    for blok in [&artist, &empty] {
        assert!(
            !blok.contains("0 bestanden"),
            "een misleidende telling op de kaart:\n{blok}"
        );
        assert!(
            !blok.contains("mapkaart__formaat"),
            "zonder bestanden valt er geen formaat te noemen:\n{blok}"
        );
    }
}

#[test]
fn the_cards_survive_the_search_field() {
    // Het zoekveld filtert ook op mapnaam, en HTMX vervangt alleen de lijst.
    // Het raster hoort dus ook in dat fragment te zitten.
    let server = Server::start_in(library_with_folders(), &[]);
    let fragment = body(&server.get_with_headers("/?q=zonder", &[("HX-Request", "true")]));

    assert!(
        fragment.contains("mapkaarten"),
        "het fragment bevat geen kaarten:\n{fragment}"
    );
    assert!(
        fragment.contains("Zonder muziek") && !fragment.contains(">Artiest<"),
        "het filter werkt niet op de kaarten:\n{fragment}"
    );
}

#[test]
fn showing_the_library_opens_no_file() {
    // AC #5: de kaarten komen uit de mapinhoud. Een bestand met de juiste
    // extensie maar onleesbare inhoud telt dus gewoon mee, en de pagina
    // struikelt er niet over — precies het bewijs dat er niets geopend wordt.
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Artiest").join("Album");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");
    std::fs::write(album.join("nep.mp3"), b"dit is geen audio")
        .expect("bestand moet te schrijven zijn");

    let server = Server::start_in(root, &[]);
    let html = body(&server.get("/map/Artiest"));

    let kaart = card(&html, "Album");
    assert!(
        kaart.contains("1 bestand"),
        "de telling komt niet uit de mapinhoud:\n{kaart}"
    );
}

#[test]
fn a_library_with_many_folders_renders_quickly() {
    // AC #7: een bibliotheek met veel mappen laadt niet merkbaar trager dan de
    // lijst die er stond. Eén `read_dir` per kaart, geen bestand open.
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    for number in 1..=200 {
        let folder = root.path().join(format!("Artiest {number:03}"));
        std::fs::create_dir(&folder).expect("map moet aan te maken zijn");
        std::fs::write(folder.join("track.mp3"), b"placeholder")
            .expect("bestand moet te schrijven zijn");
    }

    let server = Server::start_in(root, &[]);

    let start = std::time::Instant::now();
    let response = server.get("/");
    let elapsed = start.elapsed();

    assert_ok(&response);
    let html = body(&response);
    assert!(html.contains("Artiest 200"), "niet alle mappen staan er");
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "tweehonderd kaarten kostten {elapsed:?}"
    );
}

#[test]
fn the_list_gets_a_heading_per_disc() {
    // AC #1 en #2: de getagde fixture staat op schijf 1, de twee kale hebben
    // geen discnummer en vormen samen de laatste groep.
    let server = Server::start_in(library_with_album(), &[]);
    let html = body(&server.get("/map/Artiest/Het%20Album"));

    let disc = position(&html, "Schijf 1");
    let tagged = position(&html, "zzz-getagd.mp3");
    let rest = position(&html, "Zonder discnummer");
    let untagged = position(&html, "aaa-kaal.flac");

    assert!(
        disc < tagged,
        "de kop hoort boven zijn groep te staan:\n{html}"
    );
    assert!(
        tagged < rest,
        "de groep zonder schijf hoort achteraan:\n{html}"
    );
    assert!(
        rest < untagged,
        "de kop hoort boven zijn groep te staan:\n{html}"
    );

    // AC #3: de telling, en wat er aandacht vraagt.
    assert!(html.contains("2 bestanden"), "{html}");
    assert!(html.contains("vragen aandacht"), "{html}");
}

#[test]
fn a_directory_without_disc_numbers_looks_like_it_always_did() {
    // AC #5: geen enkel discnummer, dus geen kop boven de lijst.
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let album = root.path().join("Kaal");
    std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");
    place_fixture(&album, "een.mp3", "untagged.mp3");
    place_fixture(&album, "twee.flac", "untagged.flac");

    let server = Server::start_in(root, &[]);
    let html = body(&server.get("/map/Kaal"));

    assert!(html.contains("een.mp3"), "{html}");
    assert!(!html.contains("Schijf"), "{html}");
    assert!(!html.contains("Zonder discnummer"), "{html}");
}
