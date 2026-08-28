//! De startcontrole: klopt het wat de container aan rechten heeft meegekregen?
//!
//! Sleeve draait als niet-root proces. Een niet-root proces kan zijn eigen UID
//! en GID niet veranderen, dus `PUID`/`PGID` worden niet dóór de app toegepast
//! maar dóór de container-runtime (`user:` in compose, of `USER` in het image).
//! Wat deze module doet, is toetsen of dat ook werkelijk zo is uitgepakt, en dat
//! bij start melden — in plaats van de gebruiker er pas achter te laten komen
//! als zijn eerste bewerking niet opgeslagen wordt.
//!
//! De toets gebeurt met één sondebestand in `MUSIC_ROOT`, dat meteen weer
//! verdwijnt. Dat is bewust geen controle op de mode-bits: die liegen zodra er
//! ACL's op de share staan of de map setgid is, en juist op een NAS is dat
//! eerder regel dan uitzondering. Een bestand daadwerkelijk aanmaken en de
//! eigenaar ervan teruglezen beantwoordt beide vragen op de enige manier die
//! telt: mág er geschreven worden, en wie is straks de eigenaar van wat er
//! geschreven wordt.
//!
//! Deze module schrijft niets aan de bibliotheek: het sondebestand is het enige
//! wat ze aanmaakt, en het wordt binnen dezelfde functie weer opgeruimd.
//!
//! Binnen deze module wordt `std::fs::` altijd volledig gekwalificeerd
//! geschreven, om verwarring met de crate-eigen module [`crate::fs`] te
//! voorkomen.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::config::Config;

/// Voorvoegsel van het sondebestand.
///
/// De naam begint met een punt zodat hij, mocht hij onverhoopt toch blijven
/// staan, in de mapbrowser niet opduikt; het proces-id erachter voorkomt dat
/// twee instanties op dezelfde share elkaars sonde weghalen.
const PROBE_PREFIX: &str = ".sleeve-startcontrole";

/// Eigenaar en groep waarmee de app bestanden op de bibliotheek wegzet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ownership {
    pub uid: u32,
    pub gid: u32,
}

/// Voert de startcontrole uit en logt de uitkomst.
///
/// Deze functie stopt de applicatie nooit. Een `MUSIC_ROOT` waar niet op
/// geschreven mag worden is vervelend, maar bladeren en tags bekijken werkt
/// gewoon — en een draaiende UI met een duidelijke logregel is makkelijker te
/// diagnosticeren dan een container die alleen maar herstart. Configuratie die
/// niet klopt blijft wél fataal; dat gebeurt een stap eerder, in [`Config`].
pub fn check(config: &Config) {
    match write_access(&config.music_root) {
        Ok(ownership) => {
            tracing::info!(
                uid = ownership.uid,
                gid = ownership.gid,
                "MUSIC_ROOT is schrijfbaar"
            );

            if ownership.uid != config.puid || ownership.gid != config.pgid {
                tracing::warn!(
                    uid = ownership.uid,
                    gid = ownership.gid,
                    puid = config.puid,
                    pgid = config.pgid,
                    "geschreven bestanden krijgen een andere eigenaar dan PUID/PGID voorschrijven; \
                     zet in de container `user:` op dezelfde waarden als PUID en PGID"
                );
            }
        }
        Err(error) => {
            tracing::error!(
                path = %config.music_root.display(),
                %error,
                "MUSIC_ROOT is niet schrijfbaar; opslaan zal mislukken. \
                 Controleer de rechten op de share en of `user:` overeenkomt met PUID en PGID"
            );
        }
    }
}

/// Zet één bestand in `root` neer, leest de eigenaar terug en ruimt het op.
///
/// De sonde wordt met `create_new` aangemaakt: gaat er ooit iets mis met de
/// naamgeving, dan mislukt de controle liever dan dat ze een bestaand bestand
/// overschrijft.
pub fn write_access(root: &Path) -> std::io::Result<Ownership> {
    let probe = probe_path(root);

    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)?;

    // De metadata via de open handle, niet via het pad: dan beschrijft ze
    // gegarandeerd het bestand dat we net hebben aangemaakt.
    let ownership = file.metadata().map(|metadata| Ownership {
        uid: metadata.uid(),
        gid: metadata.gid(),
    });

    // Ook als het uitlezen mislukte moet de sonde weg; hem laten liggen is
    // erger dan de fout die we terugmelden.
    drop(file);
    if let Err(error) = std::fs::remove_file(&probe) {
        tracing::warn!(
            path = %probe.display(),
            %error,
            "het sondebestand van de startcontrole kon niet worden opgeruimd"
        );
    }

    ownership
}

/// Pad van het sondebestand binnen `root`.
fn probe_path(root: &Path) -> PathBuf {
    root.join(format!("{PROBE_PREFIX}-{}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Maakt een wegwerpmap; tests raken nooit de echte bibliotheek.
    fn temporary_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir moet aan te maken zijn")
    }

    #[test]
    fn a_writable_root_reports_the_owner_of_a_new_file() {
        let root = temporary_root();

        let ownership = write_access(root.path()).expect("een tempdir hoort schrijfbaar te zijn");

        // De referentie is een bestand dat we op dezelfde manier aanmaken: dat
        // is per definitie de eigenaar die een schrijfactie zou opleveren.
        let reference = root.path().join("referentie");
        std::fs::write(&reference, b"").expect("bestand moet te schrijven zijn");
        let metadata = std::fs::metadata(&reference).expect("metadata moet leesbaar zijn");

        assert_eq!(ownership.uid, metadata.uid());
        assert_eq!(ownership.gid, metadata.gid());
    }

    #[test]
    fn the_probe_leaves_nothing_behind() {
        let root = temporary_root();

        write_access(root.path()).expect("een tempdir hoort schrijfbaar te zijn");

        let overgebleven: Vec<_> = std::fs::read_dir(root.path())
            .expect("map moet leesbaar zijn")
            .map(|entry| entry.expect("map-entry moet leesbaar zijn").file_name())
            .collect();

        assert!(
            overgebleven.is_empty(),
            "de startcontrole liet iets achter: {overgebleven:?}"
        );
    }

    #[test]
    fn a_read_only_root_is_reported_as_an_error() {
        use std::os::unix::fs::PermissionsExt;

        let root = temporary_root();
        let readonly = root.path().join("alleen-lezen");
        std::fs::create_dir(&readonly).expect("map moet aan te maken zijn");
        std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o555))
            .expect("rechten moeten te zetten zijn");

        let uitkomst = write_access(&readonly);

        // Draait de test als root, dan mag er tóch geschreven worden en zegt
        // dit geval niets. Op de NAS draait de app juist niet als root; daar is
        // dit het geval dat de gebruiker te zien krijgt.
        if uitkomst.is_ok() {
            std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o755))
                .expect("rechten moeten te herstellen zijn");
            return;
        }

        let error = uitkomst.expect_err("een map zonder schrijfrecht mag niet slagen");
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);

        // Zonder schrijfrecht terugzetten kan de tempdir zichzelf niet opruimen.
        std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o755))
            .expect("rechten moeten te herstellen zijn");
    }

    #[test]
    fn the_probe_name_is_hidden_and_unique_per_process() {
        let path = probe_path(Path::new("/music"));
        let naam = path
            .file_name()
            .expect("sonde moet een naam hebben")
            .to_string_lossy()
            .into_owned();

        assert!(naam.starts_with('.'), "naam was: {naam}");
        assert!(
            naam.ends_with(&std::process::id().to_string()),
            "naam was: {naam}"
        );
    }
}
