//! Sleeve (`sleeve-tag`) — web-based tag editor voor MP3- en FLAC-bestanden.
//!
//! De weergavenaam van de applicatie is "Sleeve"; `sleeve-tag` is de technische
//! naam (crate, binary, Docker-image, containerhostnaam).
//!
//! In deze fase is de binary bewust minimaal: hij leest zijn configuratie, zet
//! logging op en sluit af. De HTTP-server komt in de webserver-taak van fase 0.

mod config;
mod fs;
mod tags;
mod web;

use std::io::IsTerminal;

use clap::Parser;

fn main() {
    // Eerst de configuratie: die bepaalt het logniveau. Gaat het parsen mis, dan
    // print clap zelf een melding op stderr en stopt het proces met een
    // foutcode — precies wat je wilt als een container verkeerd is ingesteld.
    let config = config::Config::parse();

    // Kleuren alleen wanneer een mens meekijkt: in `docker logs` of een
    // logbestand leveren ANSI-codes onleesbare rommel op.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(&config.log_level))
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Sleeve gestart");
    config.log_effective();
}
