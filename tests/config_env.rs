//! Verifieert de configuratie via de echte route: omgevingsvariabelen.
//!
//! De binary wordt als subprocess gestart met een volledig lege omgeving, zodat
//! de test niet afhangt van wat er toevallig in de shell van de ontwikkelaar of
//! de CI-runner staat. Dat is ook de reden dat deze gevallen hier staan en niet
//! als unit-test bij `config`: daar zou clap de omgeving van het testproces
//! meelezen, en zou een gezette `PORT` de uitkomst bepalen.

mod common;

use common::{Server, start_and_expect_exit};

#[test]
fn starts_with_only_music_root_and_logs_defaults() {
    // De poort wordt door de testhelper gezet omdat tests parallel draaien; de
    // overige waarden komen uit de standaardwaarden van de applicatie.
    let server = Server::start(&[]);
    let log = server.wait_for_log("Configuratie geladen");

    assert!(log.contains("puid=1000"), "log was: {log}");
    assert!(log.contains("pgid=10"), "log was: {log}");
    assert!(log.contains("max_art_size=1000x1000"), "log was: {log}");
    assert!(log.contains("log_level=info"), "log was: {log}");
    assert!(log.contains("backup_on_write=false"), "log was: {log}");
}

#[test]
fn logs_given_values_instead_of_defaults() {
    let server = Server::start(&[
        ("PUID", "1001"),
        ("PGID", "20"),
        ("MAX_ART_SIZE", "800x600"),
        ("LOG_LEVEL", "debug"),
        ("BACKUP_ON_WRITE", "true"),
    ]);
    let log = server.wait_for_log("Configuratie geladen");

    assert!(log.contains("puid=1001"), "log was: {log}");
    assert!(log.contains("pgid=20"), "log was: {log}");
    assert!(log.contains("max_art_size=800x600"), "log was: {log}");
    assert!(log.contains("backup_on_write=true"), "log was: {log}");
}

#[test]
fn empty_log_level_falls_back_to_info() {
    let server = Server::start(&[("LOG_LEVEL", "")]);
    let log = server.wait_for_log("Configuratie geladen");

    assert!(log.contains("log_level=info"), "log was: {log}");
}

#[test]
fn refuses_to_start_without_music_root() {
    let result = start_and_expect_exit(&[]);

    assert!(
        !result.status.success(),
        "zonder MUSIC_ROOT mag de app niet starten"
    );

    let message = String::from_utf8_lossy(&result.stderr);
    assert!(message.contains("MUSIC_ROOT"), "melding was: {message}");
}

#[test]
fn refuses_to_start_with_a_missing_music_root() {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let missing = root.path().join("bestaat-niet");
    let result =
        start_and_expect_exit(&[("MUSIC_ROOT", missing.to_str().expect("pad moet UTF-8 zijn"))]);

    assert!(
        !result.status.success(),
        "een niet-bestaande MUSIC_ROOT mag de app niet laten starten"
    );

    let message = String::from_utf8_lossy(&result.stderr);
    assert!(message.contains("MUSIC_ROOT"), "melding was: {message}");
    assert!(message.contains("bestaat niet"), "melding was: {message}");
}

#[test]
fn names_the_variable_on_an_invalid_value() {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let path = root.path().to_str().expect("pad moet UTF-8 zijn");

    for (variable, value) in [
        ("PORT", "geen-getal"),
        ("MAX_ART_SIZE", "groot"),
        ("BACKUP_ON_WRITE", "misschien"),
        ("PUID", "jeroen"),
    ] {
        let result = start_and_expect_exit(&[("MUSIC_ROOT", path), (variable, value)]);

        assert!(
            !result.status.success(),
            "een ongeldige {variable} moet de start blokkeren"
        );

        let message = String::from_utf8_lossy(&result.stderr);
        assert!(
            message.contains(variable),
            "melding noemt {variable} niet: {message}"
        );
    }
}
