//! Padafhandeling: de enige plek waar een door de gebruiker aangeleverd pad naar
//! een filesystem-pad wordt vertaald.
//!
//! Elk binnenkomend pad wordt gecanonicaliseerd en gecontroleerd tegen
//! `MUSIC_ROOT`; paden buiten die root en symlinks die eruit wijzen worden
//! geweigerd. Ook de vraag of een bestand bewerkbaar is (`.mp3`/`.flac` én een
//! herkend containerformaat) hoort hier thuis.
//!
//! Handlers bouwen nooit zelf een pad op. Zonder die regel is één vergeten
//! controle genoeg om buiten de muziekbibliotheek te kunnen lezen of schrijven.
//!
//! Binnen deze module wordt `std::fs::` altijd volledig gekwalificeerd
//! geschreven, om verwarring met deze crate-eigen module te voorkomen.

// De mapbrowser is de eerste die paden oplost; tot die taak worden `resolveer`
// en `is_bewerkbaar` alleen door de tests aangeroepen. De functionaliteit hoort
// hier al te staan, want elke latere handler leunt erop.
#![allow(dead_code)]

use std::path::{Component, Path, PathBuf};

/// Extensies die de app als bewerkbaar beschouwt.
const BEWERKBARE_EXTENSIES: &[&str] = &["mp3", "flac"];

/// Wat er mis kan gaan bij het omzetten van een gebruikerspad.
///
/// De meldingen bevatten bewust geen pad: ze zijn bedoeld voor de browser, en
/// een absoluut pad van de NAS hoort daar niet te belanden. Voor diagnose logt
/// de aanroeper het volledige pad erbij.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PadFout {
    #[error("dit pad valt buiten de muziekbibliotheek")]
    BuitenBibliotheek,

    #[error("dit pad bestaat niet")]
    NietGevonden,

    #[error("dit bestandstype wordt niet ondersteund")]
    NietOndersteund,
}

/// De muziekbibliotheek: alles onder de geconfigureerde `MUSIC_ROOT`.
#[derive(Debug, Clone)]
pub struct Bibliotheek {
    root: PathBuf,
}

impl Bibliotheek {
    /// Maakt een bibliotheek met `root` als grens.
    ///
    /// `root` moet al gecanonicaliseerd zijn; de configuratielaag doet dat bij
    /// het inlezen van `MUSIC_ROOT`. Is dat niet gebeurd, dan mislukt de
    /// vergelijking met het gecanonicaliseerde doelpad en wordt alles geweigerd
    /// — vervelend, maar de veilige kant.
    pub fn nieuw(root: PathBuf) -> Self {
        Self { root }
    }

    /// De wortel van de bibliotheek.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Zet een door de gebruiker aangeleverd relatief pad om naar een absoluut
    /// pad binnen de bibliotheek.
    ///
    /// Een lege invoer (of `.`) levert de root zelf op, wat de mapbrowser als
    /// startpunt gebruikt.
    pub fn resolveer(&self, relatief: &str) -> Result<PathBuf, PadFout> {
        let veilig = self.controleer_componenten(relatief)?;
        let kandidaat = self.root.join(veilig);

        // canonicalize volgt symlinks en lost `.` op. Bestaat het pad niet, dan
        // is er niets te tonen; dat is een 404 en geen beveiligingsprobleem.
        let absoluut = std::fs::canonicalize(&kandidaat).map_err(|_| PadFout::NietGevonden)?;

        // Pas ná het volgen van symlinks is te zien waar het pad écht uitkomt.
        if !absoluut.starts_with(&self.root) {
            return Err(PadFout::BuitenBibliotheek);
        }

        Ok(absoluut)
    }

    /// Zoals [`Bibliotheek::resolveer`], maar eist dat het resultaat een
    /// bewerkbaar audiobestand is.
    pub fn resolveer_bewerkbaar_bestand(&self, relatief: &str) -> Result<PathBuf, PadFout> {
        let absoluut = self.resolveer(relatief)?;

        if !absoluut.is_file() || !is_bewerkbaar(&absoluut) {
            return Err(PadFout::NietOndersteund);
        }

        Ok(absoluut)
    }

    /// Het pad zoals de UI het mag tonen: relatief aan de root.
    ///
    /// Voorkomt dat het absolute pad van de NAS in de interface of in een URL
    /// belandt.
    pub fn relatief_pad<'a>(&self, absoluut: &'a Path) -> Option<&'a Path> {
        absoluut.strip_prefix(&self.root).ok()
    }

    /// Weigert padcomponenten die buiten de bibliotheek kunnen wijzen.
    ///
    /// Dit gebeurt vóór canonicalisatie, dus zonder het filesystem aan te raken.
    /// De controle ná canonicalisatie vangt hetzelfde af, maar twee sloten op
    /// dezelfde deur is hier op zijn plaats: dit is de enige barrière tussen een
    /// URL en de bestanden van de gebruiker.
    fn controleer_componenten(&self, relatief: &str) -> Result<PathBuf, PadFout> {
        let mut schoon = PathBuf::new();

        for component in Path::new(relatief).components() {
            match component {
                Component::Normal(deel) => schoon.push(deel),
                // `.` mag; het verandert niets aan waar het pad uitkomt.
                Component::CurDir => {}
                // `..`, een leidende `/` of een Windows-prefix wijzen allemaal
                // weg van de root.
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(PadFout::BuitenBibliotheek);
                }
            }
        }

        Ok(schoon)
    }
}

/// Bepaalt of een bestand bewerkbaar is: juiste extensie én herkend formaat.
///
/// De extensie is de goedkope voorselectie, het containerformaat het echte
/// oordeel. Alleen op de extensie afgaan zou betekenen dat de app een willekeurig
/// bestand met de naam `track.mp3` als bewerkbaar presenteert — en er straks
/// tags in probeert te schrijven.
pub fn is_bewerkbaar(pad: &Path) -> bool {
    if !heeft_bewerkbare_extensie(pad) {
        return false;
    }

    crate::tags::herkent_formaat(pad)
}

/// Controleert alleen de extensie, hoofdletterongevoelig.
///
/// Apart van [`is_bewerkbaar`], omdat een maplijst hiermee eerst goedkoop kan
/// filteren voordat er bestanden geopend worden.
pub fn heeft_bewerkbare_extensie(pad: &Path) -> bool {
    pad.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .is_some_and(|ext| BEWERKBARE_EXTENSIES.contains(&ext.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testfixtures;

    /// Bouwt een bibliotheek met een tempdir als root, met daarin een album.
    ///
    /// De root wordt gecanonicaliseerd omdat macOS `/var` naar `/private/var`
    /// laat wijzen; zonder die stap zou elke vergelijking mislukken.
    fn bibliotheek_met_album() -> (tempfile::TempDir, Bibliotheek) {
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        let album = tempdir.path().join("Artiest").join("Album");
        std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

        testfixtures::kopieer_naar(&album, testfixtures::MP3_MET_TAGS);
        testfixtures::kopieer_naar(&album, testfixtures::FLAC_MET_TAGS);

        let root =
            std::fs::canonicalize(tempdir.path()).expect("root moet te canonicaliseren zijn");
        (tempdir, Bibliotheek::nieuw(root))
    }

    #[test]
    fn resolveert_een_bestaand_bestand() {
        let (_tempdir, bibliotheek) = bibliotheek_met_album();

        let pad = bibliotheek
            .resolveer("Artiest/Album/tagged.mp3")
            .expect("een bestaand bestand moet oplosbaar zijn");

        assert!(pad.is_file());
        assert!(pad.starts_with(bibliotheek.root()));
    }

    #[test]
    fn resolveert_een_map_en_de_root_zelf() {
        let (_tempdir, bibliotheek) = bibliotheek_met_album();

        let map = bibliotheek
            .resolveer("Artiest/Album")
            .expect("een bestaande map moet oplosbaar zijn");
        assert!(map.is_dir());

        for invoer in ["", ".", "./"] {
            let root = bibliotheek
                .resolveer(invoer)
                .unwrap_or_else(|fout| panic!("'{invoer}' moet de root opleveren, kreeg {fout}"));
            assert_eq!(root, bibliotheek.root());
        }
    }

    #[test]
    fn weigert_traversal_met_dubbele_punt() {
        let (_tempdir, bibliotheek) = bibliotheek_met_album();

        for poging in [
            "..",
            "../",
            "../../etc/passwd",
            "Artiest/../../buiten",
            "Artiest/Album/../../../etc/hosts",
        ] {
            assert_eq!(
                bibliotheek.resolveer(poging),
                Err(PadFout::BuitenBibliotheek),
                "'{poging}' had geweigerd moeten worden"
            );
        }
    }

    #[test]
    fn weigert_een_absoluut_pad() {
        let (_tempdir, bibliotheek) = bibliotheek_met_album();

        for poging in ["/etc/passwd", "/", "/Users"] {
            assert_eq!(
                bibliotheek.resolveer(poging),
                Err(PadFout::BuitenBibliotheek),
                "'{poging}' had geweigerd moeten worden"
            );
        }
    }

    #[test]
    fn staat_een_symlink_binnen_de_bibliotheek_toe() {
        let (_tempdir, bibliotheek) = bibliotheek_met_album();

        // Een gebruiker mag best met symlinks werken binnen zijn eigen
        // bibliotheek; alleen naar buiten wijzen is verboden.
        let doel = bibliotheek.root().join("Artiest").join("Album");
        let link = bibliotheek.root().join("Snelkoppeling");
        std::os::unix::fs::symlink(&doel, &link).expect("symlink moet aan te maken zijn");

        let pad = bibliotheek
            .resolveer("Snelkoppeling/tagged.mp3")
            .expect("een symlink binnen de bibliotheek moet werken");

        assert!(pad.is_file());
        assert!(pad.starts_with(bibliotheek.root()));
    }

    #[test]
    fn weigert_een_symlink_die_de_bibliotheek_uit_wijst() {
        let (_tempdir, bibliotheek) = bibliotheek_met_album();

        let buiten = tempfile::tempdir().expect("tweede tempdir moet aan te maken zijn");
        let geheim = buiten.path().join("geheim.txt");
        std::fs::write(&geheim, b"niet voor de app").expect("bestand moet te schrijven zijn");

        let link = bibliotheek.root().join("ontsnapping");
        std::os::unix::fs::symlink(buiten.path(), &link).expect("symlink moet aan te maken zijn");

        // De componentcontrole laat dit door — er staat geen `..` in — dus dit
        // geval wordt uitsluitend gevangen doordat canonicalize de symlink volgt.
        assert_eq!(
            bibliotheek.resolveer("ontsnapping/geheim.txt"),
            Err(PadFout::BuitenBibliotheek)
        );
    }

    #[test]
    fn geeft_niet_gevonden_voor_een_onbestaand_pad() {
        let (_tempdir, bibliotheek) = bibliotheek_met_album();

        assert_eq!(
            bibliotheek.resolveer("Artiest/Album/bestaat-niet.mp3"),
            Err(PadFout::NietGevonden)
        );
    }

    #[test]
    fn bewerkbaar_bestand_moet_juiste_extensie_en_formaat_hebben() {
        let (_tempdir, bibliotheek) = bibliotheek_met_album();

        assert!(
            bibliotheek
                .resolveer_bewerkbaar_bestand("Artiest/Album/tagged.mp3")
                .is_ok()
        );
        assert!(
            bibliotheek
                .resolveer_bewerkbaar_bestand("Artiest/Album/tagged.flac")
                .is_ok()
        );

        // Verkeerde extensie.
        let tekst = bibliotheek.root().join("Artiest").join("notities.txt");
        std::fs::write(&tekst, b"geen audio").expect("bestand moet te schrijven zijn");
        assert_eq!(
            bibliotheek.resolveer_bewerkbaar_bestand("Artiest/notities.txt"),
            Err(PadFout::NietOndersteund)
        );

        // Juiste extensie, verkeerde inhoud: een JPEG die zich voordoet als MP3.
        let nep = bibliotheek.root().join("Artiest").join("nep.mp3");
        std::fs::copy(testfixtures::fixture_pad(testfixtures::COVER_JPEG), &nep)
            .expect("kopie moet lukken");
        assert_eq!(
            bibliotheek.resolveer_bewerkbaar_bestand("Artiest/nep.mp3"),
            Err(PadFout::NietOndersteund)
        );

        // Een map is geen bewerkbaar bestand.
        assert_eq!(
            bibliotheek.resolveer_bewerkbaar_bestand("Artiest/Album"),
            Err(PadFout::NietOndersteund)
        );
    }

    #[test]
    fn extensiecontrole_is_hoofdletterongevoelig() {
        assert!(heeft_bewerkbare_extensie(Path::new("track.mp3")));
        assert!(heeft_bewerkbare_extensie(Path::new("track.MP3")));
        assert!(heeft_bewerkbare_extensie(Path::new("track.Flac")));

        assert!(!heeft_bewerkbare_extensie(Path::new("track.m4a")));
        assert!(!heeft_bewerkbare_extensie(Path::new("cover.jpg")));
        assert!(!heeft_bewerkbare_extensie(Path::new("zonder-extensie")));
    }

    #[test]
    fn relatief_pad_verbergt_de_root() {
        let (_tempdir, bibliotheek) = bibliotheek_met_album();

        let absoluut = bibliotheek
            .resolveer("Artiest/Album/tagged.mp3")
            .expect("bestand moet oplosbaar zijn");

        assert_eq!(
            bibliotheek.relatief_pad(&absoluut),
            Some(Path::new("Artiest/Album/tagged.mp3"))
        );
        assert_eq!(bibliotheek.relatief_pad(Path::new("/etc/passwd")), None);
    }

    #[test]
    fn foutmeldingen_lekken_geen_paden() {
        // Deze melding gaat naar de browser. Een absoluut pad van de NAS zou
        // daar informatie prijsgeven over de mapstructuur van de gebruiker.
        for fout in [
            PadFout::BuitenBibliotheek,
            PadFout::NietGevonden,
            PadFout::NietOndersteund,
        ] {
            let melding = fout.to_string();
            assert!(
                !melding.contains('/'),
                "melding bevat een pad-achtige tekst: {melding}"
            );
        }
    }
}
