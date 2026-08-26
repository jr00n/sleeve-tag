//! Sleeve (`sleeve-tag`) — web-based tag editor voor MP3- en FLAC-bestanden.
//!
//! De weergavenaam van de applicatie is "Sleeve"; `sleeve-tag` is de technische
//! naam (crate, binary, Docker-image, containerhostnaam).
//!
//! In deze fase is de binary bewust minimaal: hij zet logging op en sluit af.
//! De HTTP-server komt in de webserver-taak van fase 0.

mod config;
mod fs;
mod tags;
mod web;

/// Bepaalt de filterdirective voor `tracing` op basis van `LOG_LEVEL`.
///
/// Volledige configuratie-afhandeling volgt in de configuratietaak; hier is
/// alleen het logniveau nodig om vanaf de eerste regel bruikbare logging te
/// hebben.
fn log_directive(configured: Option<&str>) -> &str {
    match configured {
        Some(level) if !level.trim().is_empty() => level,
        _ => "info",
    }
}

fn main() {
    let configured = std::env::var("LOG_LEVEL").ok();
    let directive = log_directive(configured.as_deref());

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(directive))
        .init();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Sleeve gestart");
}

#[cfg(test)]
mod tests {
    use super::log_directive;

    #[test]
    fn valt_terug_op_info_zonder_configuratie() {
        assert_eq!(log_directive(None), "info");
    }

    #[test]
    fn valt_terug_op_info_bij_lege_waarde() {
        assert_eq!(log_directive(Some("   ")), "info");
    }

    #[test]
    fn gebruikt_geconfigureerd_niveau() {
        assert_eq!(log_directive(Some("debug")), "debug");
    }
}
