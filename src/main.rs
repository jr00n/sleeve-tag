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
mod cover;
mod edit;
mod fs;
mod health;
mod startup;
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

/// Vlag waarmee de container zijn eigen healthcheck draait.
const HEALTH_FLAG: &str = "--health";

#[tokio::main]
async fn main() {
    // De healthcheck vóór alles: distroless heeft geen shell en geen curl, dus
    // draait de container deze binary opnieuw om zichzelf te bevragen. Die
    // modus heeft alleen PORT nodig en mag niet vastlopen op een MUSIC_ROOT die
    // op dat moment niet gezet is — vandaar dat clap er niet aan te pas komt.
    if std::env::args().skip(1).any(|arg| arg == HEALTH_FLAG) {
        let port = config::port_from_env();
        std::process::exit(if health::probe(port) { 0 } else { 1 });
    }

    // Eerst de configuratie: die bepaalt het logniveau. Gaat het parsen mis, dan
    // print clap zelf een melding op stderr en stopt het proces met een
    // foutcode — precies wat je wilt als een container verkeerd is ingesteld.
    let config = config::Config::parse();

    // Kleuren alleen wanneer een mens meekijkt: in `docker logs` of een
    // logbestand leveren ANSI-codes onleesbare rommel op.
    tracing_subscriber::fmt()
        .with_env_filter(log_filter(&config.log_level))
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Sleeve gestart");
    config.log_effective();

    // Meteen na de configuratie, want dit is de plek waar een verkeerd gezette
    // `user:` of een read-only mount zichtbaar hoort te worden — niet pas bij de
    // eerste bewerking die de gebruiker probeert op te slaan.
    startup::check(&config);

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

/// Bouwt het logfilter uit `LOG_LEVEL`, met de tagbibliotheek standaard stiller.
///
/// Die bibliotheek waarschuwt bij élk inlezen van een FLAC met een ID3-blok dat
/// ze die tag niet kan herschrijven. Op een bibliotheek waar een ripper hele
/// albums zo heeft achtergelaten, levert het openen van één map tientallen
/// identieke regels op — en die verdringen wat er wél toe doet. Dat een bestand
/// zo'n blok draagt, meldt Sleeve zelf: in de maplijst, op de pagina met ruwe
/// tags, en in het rapport zodra het is opgeruimd.
///
/// Wie de meldingen tóch wil zien, noemt het doel zelf in `LOG_LEVEL` — de
/// naam staat in [`tags::LOG_TARGET`] en in de README; dan blijft die keuze
/// staan en wordt er hier niets meer aan toegevoegd. Hoe
/// dat doel heet, weet alleen `tags::` — de rest van de app hoort niet te weten
/// met welke crate daar tags gelezen worden.
fn log_filter(log_level: &str) -> tracing_subscriber::EnvFilter {
    let filter = tracing_subscriber::EnvFilter::new(log_level);
    let target = tags::LOG_TARGET;

    if log_level.contains(target) {
        return filter;
    }

    filter.add_directive(
        format!("{target}=error")
            .parse()
            .expect("het vaste filter voor de tagbibliotheek moet geldig zijn"),
    )
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
