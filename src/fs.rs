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

// De mapbrowser is de eerste die paden oplost; tot die taak worden `resolve`
// en `is_editable` alleen door de tests aangeroepen. De functionaliteit hoort
// hier al te staan, want elke latere handler leunt erop.
#![allow(dead_code)]

use std::path::{Component, Path, PathBuf};

/// Extensies die de app als bewerkbaar beschouwt.
const EDITABLE_EXTENSIONS: &[&str] = &["mp3", "flac"];

/// Wat er mis kan gaan bij het omzetten van een gebruikerspad.
///
/// De meldingen bevatten bewust geen pad: ze zijn bedoeld voor de browser, en
/// een absoluut pad van de NAS hoort daar niet te belanden. Voor diagnose logt
/// de aanroeper het volledige pad erbij.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("dit pad valt buiten de muziekbibliotheek")]
    OutsideLibrary,

    #[error("dit pad bestaat niet")]
    NotFound,

    #[error("dit bestandstype wordt niet ondersteund")]
    Unsupported,
}

/// De muziekbibliotheek: alles onder de geconfigureerde `MUSIC_ROOT`.
#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
}

impl Library {
    /// Maakt een bibliotheek met `root` als grens.
    ///
    /// `root` moet al gecanonicaliseerd zijn; de configuratielaag doet dat bij
    /// het inlezen van `MUSIC_ROOT`. Is dat niet gebeurd, dan mislukt de
    /// vergelijking met het gecanonicaliseerde doelpad en wordt alles geweigerd
    /// — vervelend, maar de veilige kant.
    pub fn new(root: PathBuf) -> Self {
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
    pub fn resolve(&self, relative: &str) -> Result<PathBuf, PathError> {
        let safe = self.check_components(relative)?;
        let candidate = self.root.join(safe);

        // canonicalize volgt symlinks en lost `.` op. Bestaat het pad niet, dan
        // is er niets te tonen; dat is een 404 en geen beveiligingsprobleem.
        let absolute = std::fs::canonicalize(&candidate).map_err(|_| PathError::NotFound)?;

        // Pas ná het volgen van symlinks is te zien waar het pad écht uitkomt.
        if !absolute.starts_with(&self.root) {
            return Err(PathError::OutsideLibrary);
        }

        Ok(absolute)
    }

    /// Zoals [`Library::resolve`], maar eist dat het resultaat een
    /// bewerkbaar audiobestand is.
    pub fn resolve_editable_file(&self, relative: &str) -> Result<PathBuf, PathError> {
        let absolute = self.resolve(relative)?;

        if !absolute.is_file() || !is_editable(&absolute) {
            return Err(PathError::Unsupported);
        }

        Ok(absolute)
    }

    /// Het pad zoals de UI het mag tonen: relatief aan de root.
    ///
    /// Voorkomt dat het absolute pad van de NAS in de interface of in een URL
    /// belandt.
    pub fn relative_path<'a>(&self, absolute: &'a Path) -> Option<&'a Path> {
        absolute.strip_prefix(&self.root).ok()
    }

    /// Weigert padcomponenten die buiten de bibliotheek kunnen wijzen.
    ///
    /// Dit gebeurt vóór canonicalisatie, dus zonder het filesystem aan te raken.
    /// De controle ná canonicalisatie vangt hetzelfde af, maar twee sloten op
    /// dezelfde deur is hier op zijn plaats: dit is de enige barrière tussen een
    /// URL en de bestanden van de gebruiker.
    fn check_components(&self, relative: &str) -> Result<PathBuf, PathError> {
        let mut clean = PathBuf::new();

        for component in Path::new(relative).components() {
            match component {
                Component::Normal(part) => clean.push(part),
                // `.` mag; het verandert niets aan waar het pad uitkomt.
                Component::CurDir => {}
                // `..`, een leidende `/` of een Windows-prefix wijzen allemaal
                // weg van de root.
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(PathError::OutsideLibrary);
                }
            }
        }

        Ok(clean)
    }
}

/// Bepaalt of een bestand bewerkbaar is: juiste extensie én herkend formaat.
///
/// De extensie is de goedkope voorselectie, het containerformaat het echte
/// oordeel. Alleen op de extensie afgaan zou betekenen dat de app een willekeurig
/// bestand met de naam `track.mp3` als bewerkbaar presenteert — en er straks
/// tags in probeert te schrijven.
pub fn is_editable(path: &Path) -> bool {
    if !has_editable_extension(path) {
        return false;
    }

    crate::tags::is_supported_format(path)
}

/// Controleert alleen de extensie, hoofdletterongevoelig.
///
/// Apart van [`is_editable`], omdat een maplijst hiermee eerst goedkoop kan
/// filteren voordat er bestanden geopend worden.
pub fn has_editable_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .is_some_and(|ext| EDITABLE_EXTENSIONS.contains(&ext.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testfixtures;

    /// Bouwt een bibliotheek met een tempdir als root, met daarin een album.
    ///
    /// De root wordt gecanonicaliseerd omdat macOS `/var` naar `/private/var`
    /// laat wijzen; zonder die stap zou elke vergelijking mislukken.
    fn library_with_album() -> (tempfile::TempDir, Library) {
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        let album = tempdir.path().join("Artiest").join("Album");
        std::fs::create_dir_all(&album).expect("albummap moet aan te maken zijn");

        testfixtures::copy_to(&album, testfixtures::MP3_WITH_TAGS);
        testfixtures::copy_to(&album, testfixtures::FLAC_WITH_TAGS);

        let root =
            std::fs::canonicalize(tempdir.path()).expect("root moet te canonicaliseren zijn");
        (tempdir, Library::new(root))
    }

    #[test]
    fn resolves_an_existing_file() {
        let (_tempdir, library) = library_with_album();

        let path = library
            .resolve("Artiest/Album/tagged.mp3")
            .expect("een bestaand bestand moet oplosbaar zijn");

        assert!(path.is_file());
        assert!(path.starts_with(library.root()));
    }

    #[test]
    fn resolves_a_directory_and_the_root_itself() {
        let (_tempdir, library) = library_with_album();

        let directory = library
            .resolve("Artiest/Album")
            .expect("een bestaande map moet oplosbaar zijn");
        assert!(directory.is_dir());

        for input in ["", ".", "./"] {
            let root = library
                .resolve(input)
                .unwrap_or_else(|error| panic!("'{input}' moet de root opleveren, kreeg {error}"));
            assert_eq!(root, library.root());
        }
    }

    #[test]
    fn rejects_traversal_with_dotdot() {
        let (_tempdir, library) = library_with_album();

        for attempt in [
            "..",
            "../",
            "../../etc/passwd",
            "Artiest/../../buiten",
            "Artiest/Album/../../../etc/hosts",
        ] {
            assert_eq!(
                library.resolve(attempt),
                Err(PathError::OutsideLibrary),
                "'{attempt}' had geweigerd moeten worden"
            );
        }
    }

    #[test]
    fn rejects_an_absolute_path() {
        let (_tempdir, library) = library_with_album();

        for attempt in ["/etc/passwd", "/", "/Users"] {
            assert_eq!(
                library.resolve(attempt),
                Err(PathError::OutsideLibrary),
                "'{attempt}' had geweigerd moeten worden"
            );
        }
    }

    #[test]
    fn allows_a_symlink_inside_the_library() {
        let (_tempdir, library) = library_with_album();

        // Een gebruiker mag best met symlinks werken binnen zijn eigen
        // bibliotheek; alleen naar buiten wijzen is verboden.
        let target = library.root().join("Artiest").join("Album");
        let link = library.root().join("Snelkoppeling");
        std::os::unix::fs::symlink(&target, &link).expect("symlink moet aan te maken zijn");

        let path = library
            .resolve("Snelkoppeling/tagged.mp3")
            .expect("een symlink binnen de bibliotheek moet werken");

        assert!(path.is_file());
        assert!(path.starts_with(library.root()));
    }

    #[test]
    fn rejects_a_symlink_pointing_outside() {
        let (_tempdir, library) = library_with_album();

        let outside = tempfile::tempdir().expect("tweede tempdir moet aan te maken zijn");
        let secret = outside.path().join("geheim.txt");
        std::fs::write(&secret, b"niet voor de app").expect("bestand moet te schrijven zijn");

        let link = library.root().join("ontsnapping");
        std::os::unix::fs::symlink(outside.path(), &link).expect("symlink moet aan te maken zijn");

        // De componentcontrole laat dit door — er staat geen `..` in — dus dit
        // geval wordt uitsluitend gevangen doordat canonicalize de symlink volgt.
        assert_eq!(
            library.resolve("ontsnapping/geheim.txt"),
            Err(PathError::OutsideLibrary)
        );
    }

    #[test]
    fn reports_not_found_for_a_missing_path() {
        let (_tempdir, library) = library_with_album();

        assert_eq!(
            library.resolve("Artiest/Album/bestaat-niet.mp3"),
            Err(PathError::NotFound)
        );
    }

    #[test]
    fn editable_file_requires_extension_and_format() {
        let (_tempdir, library) = library_with_album();

        assert!(
            library
                .resolve_editable_file("Artiest/Album/tagged.mp3")
                .is_ok()
        );
        assert!(
            library
                .resolve_editable_file("Artiest/Album/tagged.flac")
                .is_ok()
        );

        // Verkeerde extensie.
        let text = library.root().join("Artiest").join("notities.txt");
        std::fs::write(&text, b"geen audio").expect("bestand moet te schrijven zijn");
        assert_eq!(
            library.resolve_editable_file("Artiest/notities.txt"),
            Err(PathError::Unsupported)
        );

        // Juiste extensie, verkeerde inhoud: een JPEG die zich voordoet als MP3.
        let fake = library.root().join("Artiest").join("fake.mp3");
        std::fs::copy(testfixtures::fixture_path(testfixtures::COVER_JPEG), &fake)
            .expect("kopie moet lukken");
        assert_eq!(
            library.resolve_editable_file("Artiest/fake.mp3"),
            Err(PathError::Unsupported)
        );

        // Een map is geen bewerkbaar bestand.
        assert_eq!(
            library.resolve_editable_file("Artiest/Album"),
            Err(PathError::Unsupported)
        );
    }

    #[test]
    fn extension_check_is_case_insensitive() {
        assert!(has_editable_extension(Path::new("track.mp3")));
        assert!(has_editable_extension(Path::new("track.MP3")));
        assert!(has_editable_extension(Path::new("track.Flac")));

        assert!(!has_editable_extension(Path::new("track.m4a")));
        assert!(!has_editable_extension(Path::new("cover.jpg")));
        assert!(!has_editable_extension(Path::new("zonder-extensie")));
    }

    #[test]
    fn relative_path_hides_the_root() {
        let (_tempdir, library) = library_with_album();

        let absolute = library
            .resolve("Artiest/Album/tagged.mp3")
            .expect("bestand moet oplosbaar zijn");

        assert_eq!(
            library.relative_path(&absolute),
            Some(Path::new("Artiest/Album/tagged.mp3"))
        );
        assert_eq!(library.relative_path(Path::new("/etc/passwd")), None);
    }

    #[test]
    fn error_messages_do_not_leak_paths() {
        // Deze melding gaat naar de browser. Een absoluut pad van de NAS zou
        // daar informatie prijsgeven over de mapstructuur van de gebruiker.
        for error in [
            PathError::OutsideLibrary,
            PathError::NotFound,
            PathError::Unsupported,
        ] {
            let message = error.to_string();
            assert!(
                !message.contains('/'),
                "melding bevat een pad-achtige tekst: {message}"
            );
        }
    }
}
