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
fn healthz_returns_200_on_the_configured_port() {
    let server = Server::start(&[]);
    let response = server.get("/healthz");

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord was: {response}"
    );
    assert!(response.ends_with("ok"), "antwoord was: {response}");
}

#[test]
fn index_renders_over_http() {
    let server = Server::start(&[]);
    let response = server.get("/");

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord was: {response}"
    );
    assert!(response.contains("Sleeve"), "antwoord was: {response}");
    assert!(
        response.contains("/static/htmx.min.js"),
        "antwoord was: {response}"
    );
}

#[test]
fn htmx_is_served_locally() {
    let server = Server::start(&[]);
    let response = server.get("/static/htmx.min.js");

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "antwoord begon met: {}",
        response.lines().next().unwrap_or_default()
    );
    assert!(response.contains("htmx"), "htmx-bestand lijkt leeg");
}

#[test]
fn requests_are_logged() {
    let server = Server::start(&[("LOG_LEVEL", "debug")]);
    let _ = server.get("/healthz");

    let log = server.wait_for_log("/healthz");
    assert!(log.contains("/healthz"), "log was: {log}");
}
