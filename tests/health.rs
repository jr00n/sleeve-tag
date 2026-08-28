//! Verifieert de healthcheck-modus zoals Docker hem gebruikt: dezelfde binary,
//! met `--health`, en alleen de exitcode telt.
//!
//! Dat de app zelf `/healthz` beantwoordt, staat elders getest. Hier gaat het om
//! de tweede bedrijfsmodus — de enige manier waarop een distroless-container
//! zichzelf kan bevragen.

mod common;

use std::net::{Ipv4Addr, TcpListener};
use std::process::Command;

use common::Server;

/// Draait `sleeve-tag --health` tegen `port` en geeft de exitcode terug.
fn health_check(port: u16) -> i32 {
    let output = Command::new(env!("CARGO_BIN_EXE_sleeve-tag"))
        .env_clear()
        .env("PORT", port.to_string())
        .arg("--health")
        .output()
        .expect("binary moet te starten zijn");

    output
        .status
        .code()
        .expect("de healthcheck hoort met een exitcode te eindigen, niet met een signaal")
}

#[test]
fn a_running_server_reports_healthy() {
    let server = Server::start(&[]);

    assert_eq!(
        health_check(server.address.port()),
        0,
        "een draaiende server hoort gezond te heten. Log was:\n{}",
        server.log()
    );
}

#[test]
fn nothing_listening_reports_unhealthy() {
    // Een poort die het besturingssysteem net heeft vrijgegeven: daar luistert
    // gegarandeerd niets meer.
    let listener =
        TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("poort moet vrij te vinden zijn");
    let port = listener
        .local_addr()
        .expect("adres moet leesbaar zijn")
        .port();
    drop(listener);

    assert_ne!(
        health_check(port),
        0,
        "zonder server hoort de healthcheck te falen"
    );
}

#[test]
fn the_health_check_does_not_need_music_root() {
    // In de container is MUSIC_ROOT wel gezet, maar de healthcheck mag er niet
    // van afhangen: hij draait vóór de configuratie en heeft alleen PORT nodig.
    // Zonder deze eigenschap zou een verkeerd gezette MUSIC_ROOT de container
    // niet alleen laten falen, maar ook ongezond laten *lijken* om de verkeerde
    // reden.
    let server = Server::start(&[]);

    let output = Command::new(env!("CARGO_BIN_EXE_sleeve-tag"))
        .env_clear()
        .env("PORT", server.address.port().to_string())
        .arg("--health")
        .output()
        .expect("binary moet te starten zijn");

    assert!(
        output.status.success(),
        "de healthcheck struikelde over een ontbrekende MUSIC_ROOT: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
