//! Verifieert de configuratie via de echte route: omgevingsvariabelen.
//!
//! De binary wordt als subprocess gestart met een volledig lege omgeving
//! (`env_clear`), zodat de test niet afhangt van wat er toevallig in de shell
//! van de ontwikkelaar of de CI-runner staat. Dat is ook de reden dat deze
//! gevallen hier staan en niet als unit-test bij `config`: daar zou clap de
//! omgeving van het testproces meelezen.

use std::path::Path;
use std::process::{Command, Output};

/// Start de gebouwde binary met een schone omgeving plus de opgegeven variabelen.
fn start_met(variabelen: &[(&str, &str)]) -> Output {
    let mut commando = Command::new(env!("CARGO_BIN_EXE_sleeve-tag"));
    commando.env_clear();
    for (naam, waarde) in variabelen {
        commando.env(naam, waarde);
    }
    commando.output().expect("binary moet te starten zijn")
}

/// Maakt een wegwerpmap die als `MUSIC_ROOT` dient.
///
/// Tests draaien nooit tegen de echte bibliotheek; elke test krijgt een eigen
/// tempdir die na afloop verdwijnt.
fn tijdelijke_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir moet aan te maken zijn")
}

fn pad_als_str(pad: &Path) -> &str {
    pad.to_str().expect("tempdir-pad moet UTF-8 zijn")
}

#[test]
fn start_met_alleen_music_root_en_logt_de_standaardwaarden() {
    let root = tijdelijke_root();
    let resultaat = start_met(&[("MUSIC_ROOT", pad_als_str(root.path()))]);

    assert!(
        resultaat.status.success(),
        "start mislukte: {}",
        String::from_utf8_lossy(&resultaat.stderr)
    );

    let log = String::from_utf8_lossy(&resultaat.stdout);
    assert!(log.contains("Configuratie geladen"), "log was: {log}");
    assert!(log.contains("port=8080"), "log was: {log}");
    assert!(log.contains("puid=1000"), "log was: {log}");
    assert!(log.contains("pgid=10"), "log was: {log}");
    assert!(log.contains("max_art_size=1000x1000"), "log was: {log}");
    assert!(log.contains("log_level=info"), "log was: {log}");
    assert!(log.contains("backup_on_write=false"), "log was: {log}");
}

#[test]
fn logt_de_opgegeven_waarden_in_plaats_van_de_standaardwaarden() {
    let root = tijdelijke_root();
    let resultaat = start_met(&[
        ("MUSIC_ROOT", pad_als_str(root.path())),
        ("PORT", "9000"),
        ("PUID", "1001"),
        ("PGID", "20"),
        ("MAX_ART_SIZE", "800x600"),
        ("LOG_LEVEL", "debug"),
        ("BACKUP_ON_WRITE", "true"),
    ]);

    assert!(
        resultaat.status.success(),
        "start mislukte: {}",
        String::from_utf8_lossy(&resultaat.stderr)
    );

    let log = String::from_utf8_lossy(&resultaat.stdout);
    assert!(log.contains("port=9000"), "log was: {log}");
    assert!(log.contains("puid=1001"), "log was: {log}");
    assert!(log.contains("pgid=20"), "log was: {log}");
    assert!(log.contains("max_art_size=800x600"), "log was: {log}");
    assert!(log.contains("backup_on_write=true"), "log was: {log}");
}

#[test]
fn weigert_te_starten_zonder_music_root() {
    let resultaat = start_met(&[]);

    assert!(
        !resultaat.status.success(),
        "zonder MUSIC_ROOT mag de app niet starten"
    );

    let melding = String::from_utf8_lossy(&resultaat.stderr);
    assert!(melding.contains("MUSIC_ROOT"), "melding was: {melding}");
}

#[test]
fn weigert_te_starten_met_niet_bestaande_music_root() {
    let root = tijdelijke_root();
    let ontbreekt = root.path().join("bestaat-niet");
    let resultaat = start_met(&[("MUSIC_ROOT", pad_als_str(&ontbreekt))]);

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
    let root = tijdelijke_root();

    let resultaat = start_met(&[
        ("MUSIC_ROOT", pad_als_str(root.path())),
        ("PORT", "geen-getal"),
    ]);
    assert!(!resultaat.status.success(), "ongeldige PORT moet falen");
    let melding = String::from_utf8_lossy(&resultaat.stderr);
    assert!(melding.contains("PORT"), "melding was: {melding}");
    assert!(melding.contains("geen-getal"), "melding was: {melding}");

    let resultaat = start_met(&[
        ("MUSIC_ROOT", pad_als_str(root.path())),
        ("MAX_ART_SIZE", "groot"),
    ]);
    assert!(
        !resultaat.status.success(),
        "ongeldige MAX_ART_SIZE moet falen"
    );
    let melding = String::from_utf8_lossy(&resultaat.stderr);
    assert!(melding.contains("MAX_ART_SIZE"), "melding was: {melding}");

    let resultaat = start_met(&[
        ("MUSIC_ROOT", pad_als_str(root.path())),
        ("BACKUP_ON_WRITE", "misschien"),
    ]);
    assert!(
        !resultaat.status.success(),
        "ongeldige BACKUP_ON_WRITE moet falen"
    );
    let melding = String::from_utf8_lossy(&resultaat.stderr);
    assert!(
        melding.contains("BACKUP_ON_WRITE"),
        "melding was: {melding}"
    );
}

#[test]
fn lege_log_level_valt_terug_op_info() {
    let root = tijdelijke_root();
    let resultaat = start_met(&[("MUSIC_ROOT", pad_als_str(root.path())), ("LOG_LEVEL", "")]);

    assert!(
        resultaat.status.success(),
        "een lege LOG_LEVEL mag de start niet blokkeren: {}",
        String::from_utf8_lossy(&resultaat.stderr)
    );

    let log = String::from_utf8_lossy(&resultaat.stdout);
    assert!(log.contains("log_level=info"), "log was: {log}");
}
