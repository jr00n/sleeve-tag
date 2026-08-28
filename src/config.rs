//! Configuratie van de applicatie, uitsluitend gelezen uit omgevingsvariabelen.
//!
//! Sleeve draait als container zonder configuratiebestand: `MUSIC_ROOT`, `PORT`,
//! `PUID`/`PGID`, `MAX_ART_SIZE`, `ART_QUALITY`, `MAX_UPLOAD_MB`, `LOG_LEVEL` en
//! `BACKUP_ON_WRITE` bepalen samen het volledige gedrag. In de container is `MUSIC_ROOT` altijd `/music`; het
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
const DEFAULT_PORT: &str = "8080";
/// Standaard-UID op de UGREEN NAS.
const DEFAULT_PUID: &str = "1000";
/// Standaard-GID op de UGREEN NAS.
const DEFAULT_PGID: &str = "10";
/// Standaard maximale resolutie voor embedded album art.
const DEFAULT_MAX_ART_SIZE: &str = "1000x1000";
/// Standaard JPEG-kwaliteit waarmee verkleinde album art wordt gecodeerd.
///
/// 85 is de gangbare bovenkant van "je ziet het verschil niet": daarboven lopen
/// de bytes hard op zonder dat het op een hoes zichtbaar is, daaronder worden
/// vlakken en kleurovergangen korrelig.
const DEFAULT_ART_QUALITY: &str = "85";
/// Standaard bovengrens aan een geüploade afbeelding, in megabytes.
///
/// Ruim boven wat een hoes ooit nodig heeft, en ruim onder wat een NAS met
/// weinig geheugen in de problemen brengt.
const DEFAULT_MAX_UPLOAD_MB: &str = "10";
/// Standaard logniveau.
const DEFAULT_LOG_LEVEL: &str = "info";

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
    #[arg(long, env = "PORT", default_value = DEFAULT_PORT, value_parser = parse_port)]
    pub port: u16,

    /// UID waaronder bestanden worden weggeschreven.
    #[arg(long, env = "PUID", default_value = DEFAULT_PUID, value_parser = parse_puid)]
    pub puid: u32,

    /// GID waaronder bestanden worden weggeschreven.
    #[arg(long, env = "PGID", default_value = DEFAULT_PGID, value_parser = parse_pgid)]
    pub pgid: u32,

    /// Maximale resolutie van embedded album art.
    #[arg(long, env = "MAX_ART_SIZE", default_value = DEFAULT_MAX_ART_SIZE, value_parser = parse_max_art_size)]
    pub max_art_size: MaxArtSize,

    /// JPEG-kwaliteit waarmee verkleinde album art wordt gecodeerd (1–100).
    #[arg(long, env = "ART_QUALITY", default_value = DEFAULT_ART_QUALITY, value_parser = parse_art_quality)]
    pub art_quality: u8,

    /// Bovengrens aan een geüploade afbeelding, in megabytes.
    #[arg(long, env = "MAX_UPLOAD_MB", default_value = DEFAULT_MAX_UPLOAD_MB, value_parser = parse_max_upload_mb)]
    pub max_upload_mb: u32,

    /// Logniveau voor `tracing`.
    #[arg(long, env = "LOG_LEVEL", default_value = DEFAULT_LOG_LEVEL, value_parser = parse_log_level)]
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
            art_quality = self.art_quality,
            max_upload_mb = self.max_upload_mb,
            log_level = %self.log_level,
            backup_on_write = self.backup_on_write,
            "Configuratie geladen"
        );
    }
}

/// Leest `PORT` uit de omgeving voor de healthcheck-modus.
///
/// Die modus draait vóór clap: hij heeft alleen de poort nodig en mag niet
/// struikelen over een `MUSIC_ROOT` die op dat moment niet gezet is. Toch komt
/// de waarde langs dezelfde parser en dezelfde standaardwaarde als de server,
/// zodat de probe nooit een andere poort kan aanwijzen dan waarop geluisterd
/// wordt.
///
/// Een onleesbare `PORT` valt hier terug op de standaardwaarde in plaats van af
/// te breken: de server zou met diezelfde waarde toch al niet gestart zijn, en
/// een healthcheck hoort dat als "niet gezond" te melden en niet als een crash.
pub fn port_from_env() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|raw| parse_port(&raw).ok())
        .unwrap_or_else(|| {
            DEFAULT_PORT
                .parse()
                .expect("de standaardpoort moet een geldig getal zijn")
        })
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
    let path = PathBuf::from(raw);

    if !path.exists() {
        return Err(format!("MUSIC_ROOT: '{raw}' bestaat niet"));
    }
    if !path.is_dir() {
        return Err(format!("MUSIC_ROOT: '{raw}' is geen map"));
    }

    std::fs::canonicalize(&path)
        .map_err(|error| format!("MUSIC_ROOT: '{raw}' is niet te openen ({error})"))
}

/// Parseert een poortnummer; poort 0 is geen bruikbare luisterpoort.
fn parse_port(raw: &str) -> Result<u16, String> {
    match raw.trim().parse::<u16>() {
        Ok(0) => Err("PORT: 0 is geen geldige poort".to_string()),
        Ok(port) => Ok(port),
        Err(_) => Err(format!(
            "PORT: ongeldige waarde '{raw}'; verwacht een getal tussen 1 en 65535"
        )),
    }
}

/// Parseert een numerieke id voor `PUID` of `PGID`.
fn parse_id(variable: &str, raw: &str) -> Result<u32, String> {
    raw.trim()
        .parse::<u32>()
        .map_err(|_| format!("{variable}: ongeldige waarde '{raw}'; verwacht een getal"))
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

    let value = raw.trim().to_ascii_lowercase();
    let (breedte, hoogte) = match value.split_once('x') {
        Some((b, h)) => (b, h),
        None => (value.as_str(), value.as_str()),
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

/// Parseert `ART_QUALITY` als een JPEG-kwaliteit van 1 tot en met 100.
///
/// 0 bestaat niet als kwaliteit en boven 100 kent de encoder geen betekenis;
/// allebei worden ze geweigerd in plaats van stilzwijgend bijgeknipt, want een
/// verkeerd ingestelde container hoort dat te zeggen.
fn parse_art_quality(raw: &str) -> Result<u8, String> {
    const UITLEG: &str = "verwacht een getal van 1 tot en met 100";

    let value = raw
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("ART_QUALITY: ongeldige waarde '{raw}'; {UITLEG}"))?;

    if !(1..=100).contains(&value) {
        return Err(format!("ART_QUALITY: '{raw}' valt buiten 1–100"));
    }

    Ok(value as u8)
}

/// Parseert `MAX_UPLOAD_MB` als een bovengrens in megabytes.
fn parse_max_upload_mb(raw: &str) -> Result<u32, String> {
    const UITLEG: &str = "verwacht een aantal megabytes, bijvoorbeeld 10";

    let value = raw
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("MAX_UPLOAD_MB: ongeldige waarde '{raw}'; {UITLEG}"))?;

    if value == 0 {
        return Err("MAX_UPLOAD_MB: 0 zou elke upload weigeren".to_string());
    }

    Ok(value)
}

/// Neemt het logniveau over; een lege waarde valt terug op `info`.
///
/// Een lege variabele is in compose-bestanden makkelijk gemaakt (`LOG_LEVEL=`)
/// en mag de container niet laten weigeren te starten.
fn parse_log_level(raw: &str) -> Result<String, String> {
    let level = raw.trim();
    if level.is_empty() {
        return Ok(DEFAULT_LOG_LEVEL.to_string());
    }
    Ok(level.to_string())
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
    fn temporary_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir moet aan te maken zijn")
    }

    #[test]
    fn music_root_is_canonicalized() {
        let root = temporary_root();
        let met_omweg = root.path().join("subdir").join("..");
        std::fs::create_dir(root.path().join("subdir")).expect("subdir moet aan te maken zijn");

        let result = parse_music_root(met_omweg.to_str().expect("pad moet UTF-8 zijn"))
            .expect("bestaande map moet geaccepteerd worden");

        assert!(!result.to_string_lossy().contains(".."));
        assert_eq!(
            result,
            std::fs::canonicalize(root.path()).expect("root moet te canonicaliseren zijn")
        );
    }

    #[test]
    fn music_root_rejects_a_missing_path() {
        let root = temporary_root();
        let missing = root.path().join("bestaat-niet");

        let error = parse_music_root(missing.to_str().expect("pad moet UTF-8 zijn"))
            .expect_err("een niet-bestaand pad moet geweigerd worden");

        assert!(error.contains("MUSIC_ROOT"), "melding was: {error}");
        assert!(error.contains("bestaat niet"), "melding was: {error}");
    }

    #[test]
    fn music_root_rejects_a_file_instead_of_a_directory() {
        let root = temporary_root();
        let file = root.path().join("track.mp3");
        std::fs::write(&file, b"geen map").expect("bestand moet te schrijven zijn");

        let error = parse_music_root(file.to_str().expect("pad moet UTF-8 zijn"))
            .expect_err("een bestand mag geen MUSIC_ROOT zijn");

        assert!(error.contains("MUSIC_ROOT"), "melding was: {error}");
        assert!(error.contains("geen map"), "melding was: {error}");
    }

    #[test]
    fn port_accepts_a_valid_value() {
        assert_eq!(parse_port("8080"), Ok(8080));
        assert_eq!(parse_port(" 9000 "), Ok(9000));
    }

    #[test]
    fn port_rejects_a_non_numeric_value() {
        let error = parse_port("abc").expect_err("tekst is geen poort");
        assert!(error.contains("PORT"), "melding was: {error}");
        assert!(error.contains("abc"), "melding was: {error}");
    }

    #[test]
    fn port_rejects_zero_and_out_of_range() {
        assert!(parse_port("0").is_err());
        assert!(parse_port("70000").is_err());
    }

    #[test]
    fn ids_name_their_own_variable_in_errors() {
        assert_eq!(parse_puid("1000"), Ok(1000));
        assert_eq!(parse_pgid("10"), Ok(10));

        let error = parse_puid("jeroen").expect_err("tekst is geen uid");
        assert!(error.contains("PUID"), "melding was: {error}");

        let error = parse_pgid("staff").expect_err("tekst is geen gid");
        assert!(error.contains("PGID"), "melding was: {error}");
    }

    #[test]
    fn max_art_size_accepts_both_notations() {
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
    fn max_art_size_rejects_nonsense_and_zero() {
        let error = parse_max_art_size("groot").expect_err("tekst is geen afmeting");
        assert!(error.contains("MAX_ART_SIZE"), "melding was: {error}");

        assert!(parse_max_art_size("0x1000").is_err());
        assert!(parse_max_art_size("1000x").is_err());
    }

    #[test]
    fn art_quality_accepts_the_whole_range_and_nothing_else() {
        assert_eq!(parse_art_quality("1"), Ok(1));
        assert_eq!(parse_art_quality("85"), Ok(85));
        assert_eq!(parse_art_quality(" 100 "), Ok(100));

        // 0 bestaat niet als kwaliteit en boven 100 kent de encoder geen
        // betekenis; allebei geweigerd in plaats van stil bijgeknipt.
        let error = parse_art_quality("0").expect_err("0 is geen kwaliteit");
        assert!(error.contains("ART_QUALITY"), "melding was: {error}");
        assert!(parse_art_quality("101").is_err());
        assert!(parse_art_quality("hoog").is_err());
        assert!(parse_art_quality("-1").is_err());
    }

    #[test]
    fn max_upload_mb_refuses_zero_and_nonsense() {
        assert_eq!(parse_max_upload_mb("10"), Ok(10));

        let error = parse_max_upload_mb("0").expect_err("0 zou alles weigeren");
        assert!(error.contains("MAX_UPLOAD_MB"), "melding was: {error}");
        assert!(parse_max_upload_mb("veel").is_err());
    }

    #[test]
    fn max_art_size_displays_readably() {
        let afmeting = parse_max_art_size("1000").expect("geldige waarde");
        assert_eq!(afmeting.to_string(), "1000x1000");
    }

    #[test]
    fn log_level_falls_back_to_info() {
        assert_eq!(parse_log_level(""), Ok("info".to_string()));
        assert_eq!(parse_log_level("   "), Ok("info".to_string()));
        assert_eq!(parse_log_level("debug"), Ok("debug".to_string()));
    }

    #[test]
    fn backup_on_write_accepts_common_notations() {
        for waar in ["true", "TRUE", "1", "yes", "on"] {
            assert_eq!(parse_backup_on_write(waar), Ok(true), "waarde: {waar}");
        }
        for onwaar in ["false", "False", "0", "no", "off"] {
            assert_eq!(parse_backup_on_write(onwaar), Ok(false), "waarde: {onwaar}");
        }
    }

    #[test]
    fn backup_on_write_rejects_unknown_values() {
        let error = parse_backup_on_write("misschien").expect_err("onbekende waarde");
        assert!(error.contains("BACKUP_ON_WRITE"), "melding was: {error}");
    }

    // De standaardwaarden en het ontbreken van MUSIC_ROOT worden bewust niet
    // hier getest: clap leest dan de omgeving van het testproces, waardoor een
    // gezette MUSIC_ROOT of PORT de uitkomst zou bepalen. Die gevallen staan in
    // tests/config_env.rs, die de binary met een lege omgeving start.

    #[test]
    fn overrides_replace_the_defaults() {
        let root = temporary_root();
        let path = root.path().to_str().expect("pad moet UTF-8 zijn");

        let config = Config::try_parse_from([
            "sleeve-tag",
            "--music-root",
            path,
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
