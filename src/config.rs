//! Configuratie van de applicatie, uitsluitend gelezen uit omgevingsvariabelen.
//!
//! Sleeve draait als container zonder configuratiebestand: `MUSIC_ROOT`, `PORT`,
//! `PUID`/`PGID`, `MAX_ART_SIZE`, `LOG_LEVEL` en `BACKUP_ON_WRITE` bepalen samen
//! het volledige gedrag. In de container is `MUSIC_ROOT` altijd `/music`; het
//! pad van de muziekshare op de host is puur een volume-mount en is de app
//! onbekend.
//!
//! Elke waarde heeft een eigen parser die bij een fout de naam van de
//! omgevingsvariabele noemt. clap noemt uit zichzelf alleen de CLI-flag, en bij
//! een container die niet opstart wil je in het logboek de variabele zien die
//! verkeerd staat.

use std::fmt;
use std::path::PathBuf;

use clap::Parser;

/// Standaardpoort waarop de webserver luistert.
const STANDAARD_PORT: &str = "8080";
/// Standaard-UID op de UGREEN NAS.
const STANDAARD_PUID: &str = "1000";
/// Standaard-GID op de UGREEN NAS.
const STANDAARD_PGID: &str = "10";
/// Standaard maximale resolutie voor embedded album art.
const STANDAARD_MAX_ART_SIZE: &str = "1000x1000";
/// Standaard logniveau.
const STANDAARD_LOG_LEVEL: &str = "info";

/// De volledige configuratie van een draaiende Sleeve-instantie.
///
/// Waarden komen uit omgevingsvariabelen; dezelfde velden zijn ook als
/// CLI-flag beschikbaar, wat lokaal ontwikkelen zonder container makkelijker
/// maakt.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "sleeve-tag",
    version,
    about = "Sleeve — web-based tag editor voor MP3 en FLAC"
)]
pub struct Config {
    /// Pad naar de muziekbibliotheek. In de container altijd `/music`.
    #[arg(long, env = "MUSIC_ROOT", value_parser = parse_music_root)]
    pub music_root: PathBuf,

    /// Poort waarop de webserver luistert.
    #[arg(long, env = "PORT", default_value = STANDAARD_PORT, value_parser = parse_port)]
    pub port: u16,

    /// UID waaronder bestanden worden weggeschreven.
    #[arg(long, env = "PUID", default_value = STANDAARD_PUID, value_parser = parse_puid)]
    pub puid: u32,

    /// GID waaronder bestanden worden weggeschreven.
    #[arg(long, env = "PGID", default_value = STANDAARD_PGID, value_parser = parse_pgid)]
    pub pgid: u32,

    /// Maximale resolutie van embedded album art.
    #[arg(long, env = "MAX_ART_SIZE", default_value = STANDAARD_MAX_ART_SIZE, value_parser = parse_max_art_size)]
    pub max_art_size: MaxArtSize,

    /// Logniveau voor `tracing`.
    #[arg(long, env = "LOG_LEVEL", default_value = STANDAARD_LOG_LEVEL, value_parser = parse_log_level)]
    pub log_level: String,

    /// Plaatst bij elke schrijfactie een `.bak` naast het bestand.
    ///
    /// `ArgAction::Set` is nodig omdat clap een `bool` anders als waardeloze
    /// flag behandelt; dan zou `BACKUP_ON_WRITE=misschien` stilzwijgend als
    /// "aan" worden gelezen in plaats van als fout.
    #[arg(
        long,
        env = "BACKUP_ON_WRITE",
        action = clap::ArgAction::Set,
        default_value = "false",
        value_parser = parse_backup_on_write
    )]
    pub backup_on_write: bool,
}

impl Config {
    /// Logt de configuratie waarmee de applicatie daadwerkelijk draait.
    ///
    /// Bij een probleem op de NAS is dit de eerste regel die duidelijk maakt of
    /// de container de bedoelde instellingen heeft meegekregen.
    pub fn log_effective(&self) {
        tracing::info!(
            music_root = %self.music_root.display(),
            port = self.port,
            puid = self.puid,
            pgid = self.pgid,
            max_art_size = %self.max_art_size,
            log_level = %self.log_level,
            backup_on_write = self.backup_on_write,
            "Configuratie geladen"
        );
    }
}

/// Maximale afmetingen waarnaar album art wordt verkleind.
///
/// Verkleinen behoudt de beeldverhouding, dus dit zijn bovengrenzen per as en
/// geen doelafmetingen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxArtSize {
    pub width: u32,
    pub height: u32,
}

impl fmt::Display for MaxArtSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

/// Controleert dat `MUSIC_ROOT` bestaat, een map is, en canonicaliseert het pad.
///
/// De canonicalisatie gebeurt hier zodat de padmodule later een betrouwbaar
/// anker heeft om binnenkomende paden tegen af te zetten.
fn parse_music_root(raw: &str) -> Result<PathBuf, String> {
    let pad = PathBuf::from(raw);

    if !pad.exists() {
        return Err(format!("MUSIC_ROOT: '{raw}' bestaat niet"));
    }
    if !pad.is_dir() {
        return Err(format!("MUSIC_ROOT: '{raw}' is geen map"));
    }

    std::fs::canonicalize(&pad)
        .map_err(|fout| format!("MUSIC_ROOT: '{raw}' is niet te openen ({fout})"))
}

/// Parseert een poortnummer; poort 0 is geen bruikbare luisterpoort.
fn parse_port(raw: &str) -> Result<u16, String> {
    match raw.trim().parse::<u16>() {
        Ok(0) => Err("PORT: 0 is geen geldige poort".to_string()),
        Ok(poort) => Ok(poort),
        Err(_) => Err(format!(
            "PORT: ongeldige waarde '{raw}'; verwacht een getal tussen 1 en 65535"
        )),
    }
}

/// Parseert een numerieke id voor `PUID` of `PGID`.
fn parse_id(variabele: &str, raw: &str) -> Result<u32, String> {
    raw.trim()
        .parse::<u32>()
        .map_err(|_| format!("{variabele}: ongeldige waarde '{raw}'; verwacht een getal"))
}

fn parse_puid(raw: &str) -> Result<u32, String> {
    parse_id("PUID", raw)
}

fn parse_pgid(raw: &str) -> Result<u32, String> {
    parse_id("PGID", raw)
}

/// Parseert `MAX_ART_SIZE` als `N` (vierkant) of `BxH`.
///
/// Het PRD noteert de standaardwaarde als 1000x1000, maar omdat verkleinen de
/// beeldverhouding behoudt is een enkel getal net zo bruikbaar. Beide vormen
/// worden geaccepteerd zodat de compose-file leesbaar mag blijven.
fn parse_max_art_size(raw: &str) -> Result<MaxArtSize, String> {
    const UITLEG: &str = "verwacht 'N' of 'BxH', bijvoorbeeld 1000 of 1000x1000";

    let waarde = raw.trim().to_ascii_lowercase();
    let (breedte, hoogte) = match waarde.split_once('x') {
        Some((b, h)) => (b, h),
        None => (waarde.as_str(), waarde.as_str()),
    };

    let breedte = breedte
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("MAX_ART_SIZE: ongeldige waarde '{raw}'; {UITLEG}"))?;
    let hoogte = hoogte
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("MAX_ART_SIZE: ongeldige waarde '{raw}'; {UITLEG}"))?;

    if breedte == 0 || hoogte == 0 {
        return Err(format!("MAX_ART_SIZE: '{raw}' bevat een afmeting van 0"));
    }

    Ok(MaxArtSize {
        width: breedte,
        height: hoogte,
    })
}

/// Neemt het logniveau over; een lege waarde valt terug op `info`.
///
/// Een lege variabele is in compose-bestanden makkelijk gemaakt (`LOG_LEVEL=`)
/// en mag de container niet laten weigeren te starten.
fn parse_log_level(raw: &str) -> Result<String, String> {
    let niveau = raw.trim();
    if niveau.is_empty() {
        return Ok(STANDAARD_LOG_LEVEL.to_string());
    }
    Ok(niveau.to_string())
}

/// Parseert een booleaanse omgevingsvariabele in de gangbare schrijfwijzen.
fn parse_backup_on_write(raw: &str) -> Result<bool, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" | "on" => Ok(true),
        "false" | "0" | "no" | "n" | "off" => Ok(false),
        _ => Err(format!(
            "BACKUP_ON_WRITE: ongeldige waarde '{raw}'; verwacht true of false"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Maakt een tempdir die als `MUSIC_ROOT` mag dienen.
    ///
    /// Tests raken nooit de echte bibliotheek; elke test die een root nodig
    /// heeft, krijgt hier een eigen wegwerpmap.
    fn tijdelijke_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir moet aan te maken zijn")
    }

    #[test]
    fn music_root_wordt_gecanonicaliseerd() {
        let root = tijdelijke_root();
        let met_omweg = root.path().join("subdir").join("..");
        std::fs::create_dir(root.path().join("subdir")).expect("subdir moet aan te maken zijn");

        let resultaat = parse_music_root(met_omweg.to_str().expect("pad moet UTF-8 zijn"))
            .expect("bestaande map moet geaccepteerd worden");

        assert!(!resultaat.to_string_lossy().contains(".."));
        assert_eq!(
            resultaat,
            std::fs::canonicalize(root.path()).expect("root moet te canonicaliseren zijn")
        );
    }

    #[test]
    fn music_root_weigert_niet_bestaand_pad() {
        let root = tijdelijke_root();
        let ontbreekt = root.path().join("bestaat-niet");

        let fout = parse_music_root(ontbreekt.to_str().expect("pad moet UTF-8 zijn"))
            .expect_err("een niet-bestaand pad moet geweigerd worden");

        assert!(fout.contains("MUSIC_ROOT"), "melding was: {fout}");
        assert!(fout.contains("bestaat niet"), "melding was: {fout}");
    }

    #[test]
    fn music_root_weigert_bestand_in_plaats_van_map() {
        let root = tijdelijke_root();
        let bestand = root.path().join("track.mp3");
        std::fs::write(&bestand, b"geen map").expect("bestand moet te schrijven zijn");

        let fout = parse_music_root(bestand.to_str().expect("pad moet UTF-8 zijn"))
            .expect_err("een bestand mag geen MUSIC_ROOT zijn");

        assert!(fout.contains("MUSIC_ROOT"), "melding was: {fout}");
        assert!(fout.contains("geen map"), "melding was: {fout}");
    }

    #[test]
    fn port_accepteert_geldige_waarde() {
        assert_eq!(parse_port("8080"), Ok(8080));
        assert_eq!(parse_port(" 9000 "), Ok(9000));
    }

    #[test]
    fn port_weigert_niet_numerieke_waarde() {
        let fout = parse_port("abc").expect_err("tekst is geen poort");
        assert!(fout.contains("PORT"), "melding was: {fout}");
        assert!(fout.contains("abc"), "melding was: {fout}");
    }

    #[test]
    fn port_weigert_nul_en_te_grote_waarde() {
        assert!(parse_port("0").is_err());
        assert!(parse_port("70000").is_err());
    }

    #[test]
    fn ids_worden_geparseerd_met_hun_eigen_naam_in_de_fout() {
        assert_eq!(parse_puid("1000"), Ok(1000));
        assert_eq!(parse_pgid("10"), Ok(10));

        let fout = parse_puid("jeroen").expect_err("tekst is geen uid");
        assert!(fout.contains("PUID"), "melding was: {fout}");

        let fout = parse_pgid("staff").expect_err("tekst is geen gid");
        assert!(fout.contains("PGID"), "melding was: {fout}");
    }

    #[test]
    fn max_art_size_accepteert_beide_schrijfwijzen() {
        assert_eq!(
            parse_max_art_size("1000"),
            Ok(MaxArtSize {
                width: 1000,
                height: 1000
            })
        );
        assert_eq!(
            parse_max_art_size("1200x800"),
            Ok(MaxArtSize {
                width: 1200,
                height: 800
            })
        );
        assert_eq!(
            parse_max_art_size("600X600"),
            Ok(MaxArtSize {
                width: 600,
                height: 600
            })
        );
    }

    #[test]
    fn max_art_size_weigert_onzin_en_nul() {
        let fout = parse_max_art_size("groot").expect_err("tekst is geen afmeting");
        assert!(fout.contains("MAX_ART_SIZE"), "melding was: {fout}");

        assert!(parse_max_art_size("0x1000").is_err());
        assert!(parse_max_art_size("1000x").is_err());
    }

    #[test]
    fn max_art_size_wordt_leesbaar_weergegeven() {
        let afmeting = parse_max_art_size("1000").expect("geldige waarde");
        assert_eq!(afmeting.to_string(), "1000x1000");
    }

    #[test]
    fn log_level_valt_terug_op_info() {
        assert_eq!(parse_log_level(""), Ok("info".to_string()));
        assert_eq!(parse_log_level("   "), Ok("info".to_string()));
        assert_eq!(parse_log_level("debug"), Ok("debug".to_string()));
    }

    #[test]
    fn backup_on_write_accepteert_gangbare_schrijfwijzen() {
        for waar in ["true", "TRUE", "1", "yes", "on"] {
            assert_eq!(parse_backup_on_write(waar), Ok(true), "waarde: {waar}");
        }
        for onwaar in ["false", "False", "0", "no", "off"] {
            assert_eq!(parse_backup_on_write(onwaar), Ok(false), "waarde: {onwaar}");
        }
    }

    #[test]
    fn backup_on_write_weigert_onbekende_waarde() {
        let fout = parse_backup_on_write("misschien").expect_err("onbekende waarde");
        assert!(fout.contains("BACKUP_ON_WRITE"), "melding was: {fout}");
    }

    // De standaardwaarden en het ontbreken van MUSIC_ROOT worden bewust niet
    // hier getest: clap leest dan de omgeving van het testproces, waardoor een
    // gezette MUSIC_ROOT of PORT de uitkomst zou bepalen. Die gevallen staan in
    // tests/config_env.rs, die de binary met een lege omgeving start.

    #[test]
    fn overrides_overschrijven_de_standaardwaarden() {
        let root = tijdelijke_root();
        let pad = root.path().to_str().expect("pad moet UTF-8 zijn");

        let config = Config::try_parse_from([
            "sleeve-tag",
            "--music-root",
            pad,
            "--port",
            "9000",
            "--puid",
            "1001",
            "--pgid",
            "20",
            "--max-art-size",
            "800x600",
            "--log-level",
            "debug",
            "--backup-on-write",
            "true",
        ])
        .expect("opgegeven waarden moeten geldig zijn");

        assert_eq!(config.port, 9000);
        assert_eq!(config.puid, 1001);
        assert_eq!(config.pgid, 20);
        assert_eq!(
            config.max_art_size,
            MaxArtSize {
                width: 800,
                height: 600
            }
        );
        assert_eq!(config.log_level, "debug");
        assert!(config.backup_on_write);
    }
}
