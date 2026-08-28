//! Sleeve (`sleeve-tag`) — web-based tag editor voor MP3- en FLAC-bestanden.
//!
//! De weergavenaam van de applicatie is "Sleeve"; `sleeve-tag` is de technische
//! naam (crate, binary, Docker-image, containerhostnaam).

mod art;
mod atomic;
mod batch;
mod browse;
mod casing;
mod checks;
mod config;
mod edit;
mod fs;
mod tags;
mod web;

/// Toegang tot de fixtures onder `tests/fixtures/`, alleen tijdens tests.
#[cfg(test)]
mod testfixtures;

use std::io::IsTerminal;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;

use clap::Parser;

/// Map met de statische assets, relatief aan de werkdirectory.
///
/// Bij `cargo run` is dat de projectroot; in de container de `WORKDIR` waarin
/// dezelfde map wordt meegekopieerd.
const STATIC_DIR: &str = "static";

#[tokio::main]
async fn main() {
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

    // 0.0.0.0 omdat de app in een container draait en van buiten de
    // netwerknamespace bereikbaar moet zijn. Afscherming gebeurt op
    // netwerkniveau (LAN en Tailscale), zoals het PRD vastlegt.
    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.port));
    let app = web::router(config, Path::new(STATIC_DIR));

    let listener = match tokio::net::TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%address, %error, "kan niet op het adres luisteren");
            std::process::exit(1);
        }
    };

    tracing::info!(%address, "webserver luistert");

    if let Err(error) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        tracing::error!(%error, "webserver is met een fout gestopt");
        std::process::exit(1);
    }

    tracing::info!("Sleeve afgesloten");
}

/// Wacht op Ctrl-C of SIGTERM.
///
/// `docker stop` stuurt SIGTERM. Netjes afsluiten betekent dat een lopend
/// verzoek zijn werk afmaakt in plaats van halverwege afgekapt te worden — bij
/// een app die tags naar bestanden schrijft is dat geen luxe.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Ctrl-C-signaal moet te installeren zijn");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM-signaal moet te installeren zijn")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("Ctrl-C ontvangen, afsluiten"),
        () = terminate => tracing::info!("SIGTERM ontvangen, afsluiten"),
    }
}
