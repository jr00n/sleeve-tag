//! Verifieert de startcontrole via de echte route: de binary starten en
//! meelezen met wat er in `docker logs` zou verschijnen.
//!
//! Het gaat hier om de melding die de gebruiker op de NAS te zien krijgt, en
//! die komt pas tot stand als de app werkelijk draait tegen een map met de
//! rechten in kwestie. Dat is met een unit-test op een functie niet te dekken.

mod common;

use std::os::unix::fs::PermissionsExt;

use common::Server;

#[test]
fn a_writable_library_is_reported_as_writable() {
    let server = Server::start(&[]);
    let log = server.wait_for_log("MUSIC_ROOT is schrijfbaar");

    // De gemeten eigenaar hoort in dezelfde regel te staan; zonder uid en gid
    // zegt de melding niets over wat een schrijfactie oplevert.
    assert!(log.contains("uid="), "log was: {log}");
    assert!(log.contains("gid="), "log was: {log}");
}

#[test]
fn a_mismatch_with_puid_and_pgid_is_warned_about() {
    // Waarden die op geen enkele ontwikkelmachine of CI-runner de echte uid en
    // gid van het testproces kunnen zijn.
    let server = Server::start(&[("PUID", "4242"), ("PGID", "4243")]);
    let log = server.wait_for_log("PUID/PGID");

    assert!(log.contains("puid=4242"), "log was: {log}");
    assert!(log.contains("pgid=4243"), "log was: {log}");
    assert!(log.contains("user:"), "log was: {log}");
}

#[test]
fn matching_puid_and_pgid_produce_no_warning() {
    let server = Server::start(&[]);
    let log = server.wait_for_log("MUSIC_ROOT is schrijfbaar");

    // De testhelper geeft geen PUID/PGID mee, dus de standaardwaarden 1000 en
    // 10 gelden. Alleen op een machine waar het testproces toevallig díe uid en
    // gid heeft, hoort er géén waarschuwing te staan.
    let eigen = std::fs::metadata(env!("CARGO_MANIFEST_DIR")).expect("metadata moet leesbaar zijn");
    use std::os::unix::fs::MetadataExt;
    if eigen.uid() != 1000 || eigen.gid() != 10 {
        return;
    }

    assert!(
        !log.contains("PUID/PGID"),
        "bij gelijke waarden hoort geen waarschuwing te staan; log was: {log}"
    );
}

#[test]
fn a_read_only_library_is_reported_at_startup() {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o555))
        .expect("rechten moeten te zetten zijn");

    // Draait de test als root, dan mag er tóch geschreven worden en valt er
    // niets te melden. Op de NAS draait de app juist niet als root.
    if std::fs::write(root.path().join(".proef"), b"").is_ok() {
        std::fs::remove_file(root.path().join(".proef")).expect("proef moet te verwijderen zijn");
        return;
    }

    let server = Server::start_in(root, &[]);
    let log = server.wait_for_log("MUSIC_ROOT is niet schrijfbaar");

    // De melding hoort te zeggen wát er niet zal werken, niet alleen dát er
    // iets mis is.
    assert!(log.contains("opslaan zal mislukken"), "log was: {log}");

    // En de app hoort gewoon door te draaien: bladeren werkt op een read-only
    // mount prima, en een UI die opkomt is makkelijker te diagnosticeren.
    let response = server.get("/healthz");
    assert!(response.contains("200 OK"), "respons was: {response}");
}
