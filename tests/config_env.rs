//! Verifieert de configuratie via de echte route: omgevingsvariabelen.
//!
//! De binary wordt als subprocess gestart met een volledig lege omgeving, zodat
//! de test niet afhangt van wat er toevallig in de shell van de ontwikkelaar of
//! de CI-runner staat. Dat is ook de reden dat deze gevallen hier staan en niet
//! als unit-test bij `config`: daar zou clap de omgeving van het testproces
//! meelezen, en zou een gezette `PORT` de uitkomst bepalen.

mod common;

use common::{Server, start_en_verwacht_afsluiten};

#[test]
fn start_met_alleen_music_root_en_logt_de_standaardwaarden() {
    // De poort wordt door de testhelper gezet omdat tests parallel draaien; de
    // overige waarden komen uit de standaardwaarden van de applicatie.
    let server = Server::start(&[]);
    let log = server.wacht_op_log("Configuratie geladen");

    assert!(log.contains("puid=1000"), "log was: {log}");
    assert!(log.contains("pgid=10"), "log was: {log}");
    assert!(log.contains("max_art_size=1000x1000"), "log was: {log}");
    assert!(log.contains("log_level=info"), "log was: {log}");
    assert!(log.contains("backup_on_write=false"), "log was: {log}");
}

#[test]
fn logt_de_opgegeven_waarden_in_plaats_van_de_standaardwaarden() {
    let server = Server::start(&[
        ("PUID", "1001"),
        ("PGID", "20"),
        ("MAX_ART_SIZE", "800x600"),
        ("LOG_LEVEL", "debug"),
        ("BACKUP_ON_WRITE", "true"),
    ]);
    let log = server.wacht_op_log("Configuratie geladen");

    assert!(log.contains("puid=1001"), "log was: {log}");
    assert!(log.contains("pgid=20"), "log was: {log}");
    assert!(log.contains("max_art_size=800x600"), "log was: {log}");
    assert!(log.contains("backup_on_write=true"), "log was: {log}");
}

#[test]
fn lege_log_level_valt_terug_op_info() {
    let server = Server::start(&[("LOG_LEVEL", "")]);
    let log = server.wacht_op_log("Configuratie geladen");

    assert!(log.contains("log_level=info"), "log was: {log}");
}

#[test]
fn weigert_te_starten_zonder_music_root() {
    let resultaat = start_en_verwacht_afsluiten(&[]);

    assert!(
        !resultaat.status.success(),
        "zonder MUSIC_ROOT mag de app niet starten"
    );

    let melding = String::from_utf8_lossy(&resultaat.stderr);
    assert!(melding.contains("MUSIC_ROOT"), "melding was: {melding}");
}

#[test]
fn weigert_te_starten_met_niet_bestaande_music_root() {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let ontbreekt = root.path().join("bestaat-niet");
    let resultaat = start_en_verwacht_afsluiten(&[(
        "MUSIC_ROOT",
        ontbreekt.to_str().expect("pad moet UTF-8 zijn"),
    )]);

    assert!(
        !resultaat.status.success(),
        "een niet-bestaande MUSIC_ROOT mag de app niet laten starten"
    );

    let melding = String::from_utf8_lossy(&resultaat.stderr);
    assert!(melding.contains("MUSIC_ROOT"), "melding was: {melding}");
    assert!(melding.contains("bestaat niet"), "melding was: {melding}");
}

#[test]
fn noemt_de_variabelenaam_bij_een_ongeldige_waarde() {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let pad = root.path().to_str().expect("pad moet UTF-8 zijn");

    for (variabele, waarde) in [
        ("PORT", "geen-getal"),
        ("MAX_ART_SIZE", "groot"),
        ("BACKUP_ON_WRITE", "misschien"),
        ("PUID", "jeroen"),
    ] {
        let resultaat = start_en_verwacht_afsluiten(&[("MUSIC_ROOT", pad), (variabele, waarde)]);

        assert!(
            !resultaat.status.success(),
            "een ongeldige {variabele} moet de start blokkeren"
        );

        let melding = String::from_utf8_lossy(&resultaat.stderr);
        assert!(
            melding.contains(variabele),
            "melding noemt {variabele} niet: {melding}"
        );
    }
}
