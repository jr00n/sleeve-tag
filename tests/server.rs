//! Start de echte binary en praat er over TCP mee.
//!
//! De router zelf wordt in `src/web` met `oneshot` getest; dat zegt niets over
//! de vraag of de server ook werkelijk op de geconfigureerde poort luistert.
//! Deze test start daarom het gebouwde programma als subprocess. De HTTP-
//! verzoeken worden met de hand over een `TcpStream` geschreven, zodat er geen
//! HTTP-client-crate aan het project hoeft te worden toegevoegd.

mod common;

use common::Server;

#[test]
fn healthz_geeft_200_op_de_geconfigureerde_poort() {
    let server = Server::start(&[]);
    let antwoord = server.get("/healthz");

    assert!(
        antwoord.starts_with("HTTP/1.1 200 OK"),
        "antwoord was: {antwoord}"
    );
    assert!(antwoord.ends_with("ok"), "antwoord was: {antwoord}");
}

#[test]
fn startpagina_rendert_over_http() {
    let server = Server::start(&[]);
    let antwoord = server.get("/");

    assert!(
        antwoord.starts_with("HTTP/1.1 200 OK"),
        "antwoord was: {antwoord}"
    );
    assert!(antwoord.contains("Sleeve"), "antwoord was: {antwoord}");
    assert!(
        antwoord.contains("/static/htmx.min.js"),
        "antwoord was: {antwoord}"
    );
}

#[test]
fn htmx_wordt_lokaal_geserveerd() {
    let server = Server::start(&[]);
    let antwoord = server.get("/static/htmx.min.js");

    assert!(
        antwoord.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        antwoord.lines().next().unwrap_or_default()
    );
    assert!(antwoord.contains("htmx"), "htmx-bestand lijkt leeg");
}

#[test]
fn verzoeken_worden_gelogd() {
    let server = Server::start(&[("LOG_LEVEL", "debug")]);
    let _ = server.get("/healthz");

    let log = server.wacht_op_log("/healthz");
    assert!(log.contains("/healthz"), "log was: {log}");
}
