//! Atomisch vervangen van de inhoud van een bestand.
//!
//! De harde eis uit het PRD is dat er nooit data verloren gaat. De bibliotheek
//! op de NAS is niet opnieuw op te bouwen, en de container kan midden in een
//! schrijfactie worden afgebroken. Elke schrijfactie in Sleeve loopt daarom via
//! [`replace`], en de volgorde ligt daar vast in plaats van bij de aanroeper:
//!
//! 1. het origineel wordt gekopieerd naar een tijdelijk bestand in dezelfde map;
//! 2. de aanroeper past dat tijdelijke bestand aan;
//! 3. de aanroeper leest het opnieuw in en keurt het goed;
//! 4. eigenaar, groep en rechten van het origineel gaan mee;
//! 5. optioneel komt er een `.bak` naast het origineel;
//! 6. pas dan wordt het tijdelijke bestand over het origineel hernoemd.
//!
//! Gaat er onderweg iets mis, dan blijft het origineel byte-voor-byte zoals het
//! was en verdwijnt het tijdelijke bestand.
//!
//! Deze module kent geen tags en geen afbeeldingen. Ze weet alleen hoe je de
//! inhoud van een bestand vervangt zonder het kwijt te raken.
//!
//! Binnen deze module wordt `std::fs::` altijd volledig gekwalificeerd
//! geschreven, om verwarring met de crate-eigen module [`crate::fs`] te
//! voorkomen.

// Tot het wegschrijven van tags er is (task-13) roepen alleen de tests deze
// module aan. De schrijfstrategie hoort er wél als eerste te staan: alles wat
// daarna komt leunt erop, en omgekeerd zou elke schrijfactie zijn eigen
// veiligheidsnet moeten uitvinden.
#![allow(dead_code)]

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Achtervoegsel van het tijdelijke bestand.
///
/// De naam begint met een punt, zodat de mapbrowser hem overslaat als er er
/// onverhoopt toch een blijft liggen — bijvoorbeeld doordat het proces precies
/// tussen twee stappen wordt afgebroken.
const TEMP_SUFFIX: &str = "sleeve-tmp";

/// Achtervoegsel van de optionele backup.
const BACKUP_SUFFIX: &str = "bak";

/// Hoe er geschreven moet worden.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// Plaatst een `.bak` met de oude inhoud naast het bestand.
    ///
    /// Komt uit `BACKUP_ON_WRITE` en staat standaard uit, om de share niet te
    /// vervuilen.
    pub backup: bool,
}

/// Wat er mis kan gaan bij het vervangen van een bestand.
///
/// De twee fouten van de aanroeper worden uit elkaar gehouden: mislukt het
/// klaarmaken, dan is er niets aan de hand en is er ook niets gebeurd. Mislukt
/// de hervalidatie, dan hebben we zojuist een onbruikbaar bestand geproduceerd
/// — het origineel is nog heel, maar dat is wel iets om te weten.
#[derive(Debug, thiserror::Error)]
pub enum WriteError<E> {
    #[error("het bestand kon niet worden klaargemaakt")]
    Prepare(#[source] E),

    #[error("het geschreven bestand doorstond de controle niet")]
    Validate(#[source] E),

    #[error("het bestand kon niet veilig vervangen worden")]
    Filesystem(#[source] std::io::Error),

    #[error("eigenaar en groep van het originele bestand konden niet behouden blijven")]
    Ownership(#[source] std::io::Error),
}

/// Vervangt de inhoud van `path`, of laat het bestand volledig ongemoeid.
///
/// `prepare` krijgt het pad van een tijdelijk bestand dat een exacte kopie van
/// het origineel is, en mag daar alles aan veranderen. Dat het een kopie is en
/// geen leeg bestand, is geen luxe: tag-I/O heeft een echt audiobestand nodig om
/// mee te beginnen, en zo hoeft de aanroeper alleen te veranderen wat hij wil
/// veranderen.
///
/// `validate` krijgt daarna hetzelfde pad en hoort het bestand opnieuw in te
/// lezen. Zegt het nee, dan gaat er niets over het origineel heen.
///
/// `changes` is een korte omschrijving van wat er is aangepast; die komt in de
/// logregel terecht. Alleen de aanroeper weet welke velden hij heeft geraakt.
pub fn replace<E>(
    path: &Path,
    options: Options,
    changes: &str,
    prepare: impl FnOnce(&Path) -> Result<(), E>,
    validate: impl FnOnce(&Path) -> Result<(), E>,
) -> Result<(), WriteError<E>> {
    let original = std::fs::metadata(path).map_err(WriteError::Filesystem)?;

    // In dezelfde map, want alleen dan is het hernoemen straks atomair: over
    // een filesystem-grens heen doet `rename` een kopieeractie, en juist dat
    // moment van halve inhoud is wat we willen uitsluiten.
    let temp = TempFile::beside(path).map_err(WriteError::Filesystem)?;

    std::fs::copy(path, temp.path()).map_err(WriteError::Filesystem)?;

    prepare(temp.path()).map_err(WriteError::Prepare)?;

    validate(temp.path()).map_err(|error| {
        tracing::error!(
            path = %path.display(),
            "het zojuist geschreven bestand kwam de controle niet door; het origineel is niet aangeraakt"
        );
        WriteError::Validate(error)
    })?;

    // `prepare` kan het bestand opnieuw hebben aangemaakt, dus rechten en
    // eigenaar worden hier gezet en niet vlak na het kopiëren.
    inherit_metadata(&original, temp.path())?;

    if options.backup {
        let backup = with_suffix(path, BACKUP_SUFFIX);
        std::fs::copy(path, &backup).map_err(WriteError::Filesystem)?;
        tracing::info!(path = %backup.display(), "backup geplaatst");
    }

    std::fs::rename(temp.path(), path).map_err(WriteError::Filesystem)?;

    // Vanaf hier bestaat het tijdelijke pad niet meer; de guard mag er niet
    // meer aan zitten, want een later bestand met dezelfde naam is niet het
    // onze.
    temp.keep();

    tracing::info!(path = %path.display(), changes, "bestand geschreven");
    Ok(())
}

/// Zet eigenaar, groep en rechten van het origineel op het nieuwe bestand.
///
/// Het tijdelijke bestand is door dit proces gemaakt en draagt dus de uid en gid
/// van het proces. Zijn die anders dan die van het origineel, dan zou het
/// hernoemen stilletjes de eigenaar van een bestand in de bibliotheek
/// veranderen. Lukt het corrigeren niet, dan mislukt de hele schrijfactie: het
/// PRD verbiedt ongevraagde wijzigingen, en dit is er één.
///
/// Op de NAS draait het proces met dezelfde `PUID`/`PGID` als de share, dus daar
/// is er niets te corrigeren.
fn inherit_metadata<E>(original: &std::fs::Metadata, temp: &Path) -> Result<(), WriteError<E>> {
    std::fs::set_permissions(temp, original.permissions()).map_err(WriteError::Filesystem)?;

    let current = std::fs::metadata(temp).map_err(WriteError::Filesystem)?;
    if current.uid() == original.uid() && current.gid() == original.gid() {
        return Ok(());
    }

    std::os::unix::fs::chown(temp, Some(original.uid()), Some(original.gid())).map_err(|error| {
        tracing::error!(
            path = %temp.display(),
            uid = original.uid(),
            gid = original.gid(),
            %error,
            "eigenaar van het origineel kon niet worden overgenomen; er wordt niets geschreven"
        );
        WriteError::Ownership(error)
    })
}

/// Plakt een achtervoegsel achter de volledige bestandsnaam.
///
/// Achter de naam en niet in plaats van de extensie: `track.mp3.bak` blijft
/// herkenbaar als de backup van `track.mp3`, en valt door de gewijzigde
/// extensie vanzelf buiten de maplijst.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);

    path.with_file_name(name)
}

/// Een tijdelijk bestand dat zichzelf opruimt.
///
/// Zolang de guard leeft, verdwijnt het bestand bij elke vroege terugkeer — ook
/// bij een paniek in de aanroeper. Dat is precies wat de eis "er blijft geen
/// tijdelijk bestand achter" vraagt, zonder dat elk foutpad daar zelf aan hoeft
/// te denken.
struct TempFile {
    path: PathBuf,

    /// Zolang dit waar is, ruimt [`Drop`] het bestand op.
    armed: bool,
}

impl TempFile {
    /// Reserveert een naam naast `original`, in dezelfde map.
    ///
    /// De naam begint met een punt en draagt de pid, zodat twee processen
    /// elkaar niet in de weg zitten en de mapbrowser het bestand overslaat.
    fn beside(original: &Path) -> std::io::Result<Self> {
        let name = original.file_name().unwrap_or_default().to_string_lossy();
        let temp = original.with_file_name(format!(".{name}.{}.{TEMP_SUFFIX}", std::process::id()));

        Ok(Self {
            path: temp,
            armed: true,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    /// Laat het bestand staan; te gebruiken zodra het hernoemd is.
    fn keep(mut self) {
        self.armed = false;
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }

        if let Err(error) = std::fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(
                path = %self.path.display(),
                %error,
                "tijdelijk bestand kon niet worden opgeruimd"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::unix::fs::PermissionsExt;

    /// De inhoud waarmee elke test begint.
    const ORIGINAL: &[u8] = b"de oorspronkelijke inhoud van het bestand";
    const REPLACEMENT: &[u8] = b"de nieuwe inhoud";

    /// Een fout van de aanroeper, zoals `prepare` of `validate` hem geeft.
    #[derive(Debug, thiserror::Error)]
    #[error("de aanroeper gaf een fout: {0}")]
    struct CallerError(&'static str);

    /// Een tempdir met daarin één bestand met de oorspronkelijke inhoud.
    fn file_in_a_directory() -> (tempfile::TempDir, PathBuf) {
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        let path = tempdir.path().join("track.mp3");
        std::fs::write(&path, ORIGINAL).expect("bestand moet te schrijven zijn");
        (tempdir, path)
    }

    /// Alle namen in een map, gesorteerd.
    fn names_in(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(directory)
            .expect("map moet leesbaar zijn")
            .map(|entry| {
                entry
                    .expect("map-entry moet leesbaar zijn")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    /// Schrijft de vervangende inhoud; het geval waarin alles goed gaat.
    fn write_replacement(temp: &Path) -> Result<(), CallerError> {
        std::fs::write(temp, REPLACEMENT).expect("tijdelijk bestand moet schrijfbaar zijn");
        Ok(())
    }

    fn accept(_temp: &Path) -> Result<(), CallerError> {
        Ok(())
    }

    #[test]
    fn a_successful_write_replaces_the_content() {
        let (tempdir, path) = file_in_a_directory();

        replace(
            &path,
            Options::default(),
            "titel",
            write_replacement,
            accept,
        )
        .expect("schrijven moet lukken");

        assert_eq!(std::fs::read(&path).expect("lezen"), REPLACEMENT);
        assert_eq!(
            names_in(tempdir.path()),
            vec!["track.mp3"],
            "er hoort niets anders in de map te staan"
        );
    }

    #[test]
    fn the_temporary_file_lives_next_to_the_original() {
        // Alleen binnen dezelfde map is het hernoemen atomair.
        let (tempdir, path) = file_in_a_directory();
        let directory = tempdir.path().to_path_buf();

        replace(
            &path,
            Options::default(),
            "titel",
            |temp| {
                assert_eq!(
                    temp.parent(),
                    Some(directory.as_path()),
                    "het tijdelijke bestand staat in een andere map: {}",
                    temp.display()
                );
                assert!(temp.exists(), "het tijdelijke bestand bestaat nog niet");
                write_replacement(temp)
            },
            accept,
        )
        .expect("schrijven moet lukken");
    }

    #[test]
    fn the_temporary_file_starts_as_a_copy_of_the_original() {
        // Tag-I/O heeft een echt audiobestand nodig om mee te beginnen.
        let (_tempdir, path) = file_in_a_directory();

        replace(
            &path,
            Options::default(),
            "titel",
            |temp| {
                assert_eq!(
                    std::fs::read(temp).expect("lezen"),
                    ORIGINAL,
                    "het tijdelijke bestand is geen kopie van het origineel"
                );
                write_replacement(temp)
            },
            accept,
        )
        .expect("schrijven moet lukken");
    }

    #[test]
    fn a_failure_while_writing_leaves_the_original_alone() {
        let (tempdir, path) = file_in_a_directory();

        let result = replace(
            &path,
            Options::default(),
            "titel",
            |temp| {
                // Halverwege stukgelopen: er staat al iets in het tijdelijke
                // bestand wanneer de fout optreedt.
                std::fs::write(temp, b"half geschreven rommel").expect("schrijven");
                Err(CallerError("schijf vol"))
            },
            accept,
        );

        assert!(matches!(result, Err(WriteError::Prepare(_))));
        assert_eq!(
            std::fs::read(&path).expect("lezen"),
            ORIGINAL,
            "het origineel is aangetast"
        );
        assert_eq!(
            names_in(tempdir.path()),
            vec!["track.mp3"],
            "er is een tijdelijk bestand blijven staan"
        );
    }

    #[test]
    fn a_failed_validation_never_touches_the_original() {
        // Het geval dat er werkelijk toe doet: het schrijven lukte, maar wat
        // eruit kwam deugt niet. Dan mag het zeker niet over het origineel heen.
        let (tempdir, path) = file_in_a_directory();

        let result = replace(
            &path,
            Options::default(),
            "titel",
            |temp| {
                std::fs::write(temp, b"onleesbaar geworden").expect("schrijven");
                Ok(())
            },
            |_temp| Err(CallerError("kon niet opnieuw ingelezen worden")),
        );

        assert!(matches!(result, Err(WriteError::Validate(_))));
        assert_eq!(std::fs::read(&path).expect("lezen"), ORIGINAL);
        assert_eq!(names_in(tempdir.path()), vec!["track.mp3"]);
    }

    #[test]
    fn a_panic_while_writing_also_leaves_nothing_behind() {
        let (tempdir, path) = file_in_a_directory();
        let directory = tempdir.path().to_path_buf();
        let target = path.clone();

        let outcome = std::panic::catch_unwind(move || {
            let _ = replace(
                &target,
                Options::default(),
                "titel",
                |temp| -> Result<(), CallerError> {
                    std::fs::write(temp, b"tot hier en niet verder").expect("schrijven");
                    panic!("er ging iets grondig mis");
                },
                accept,
            );
        });

        assert!(outcome.is_err(), "de paniek hoort door te komen");
        assert_eq!(std::fs::read(&path).expect("lezen"), ORIGINAL);
        assert_eq!(
            names_in(&directory),
            vec!["track.mp3"],
            "de opruiming bij een paniek is niet gebeurd"
        );
    }

    #[test]
    fn permissions_survive_the_write() {
        let (_tempdir, path) = file_in_a_directory();

        // Een afwijkende modus, zodat de test niet toevallig klopt doordat hij
        // gelijk is aan wat een nieuw bestand toch al krijgt.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640))
            .expect("rechten moeten in te stellen zijn");

        replace(
            &path,
            Options::default(),
            "titel",
            write_replacement,
            accept,
        )
        .expect("schrijven moet lukken");

        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o640, "de rechten zijn veranderd");
    }

    #[test]
    fn owner_and_group_survive_the_write() {
        let (_tempdir, path) = file_in_a_directory();
        let before = std::fs::metadata(&path).expect("metadata");

        replace(
            &path,
            Options::default(),
            "titel",
            write_replacement,
            accept,
        )
        .expect("schrijven moet lukken");

        let after = std::fs::metadata(&path).expect("metadata");
        assert_eq!(after.uid(), before.uid());
        assert_eq!(after.gid(), before.gid());
    }

    #[test]
    fn a_backup_is_only_made_when_asked() {
        let (tempdir, path) = file_in_a_directory();

        // Standaard: geen backup, om de share niet te vervuilen.
        replace(
            &path,
            Options::default(),
            "titel",
            write_replacement,
            accept,
        )
        .expect("schrijven moet lukken");
        assert_eq!(names_in(tempdir.path()), vec!["track.mp3"]);

        // Met de optie aan: een .bak met de inhoud van vóór deze schrijfactie.
        replace(
            &path,
            Options { backup: true },
            "titel",
            |temp| {
                std::fs::write(temp, b"nog nieuwer").expect("schrijven");
                Ok(())
            },
            accept,
        )
        .expect("schrijven moet lukken");

        assert_eq!(names_in(tempdir.path()), vec!["track.mp3", "track.mp3.bak"]);
        assert_eq!(
            std::fs::read(tempdir.path().join("track.mp3.bak")).expect("lezen"),
            REPLACEMENT,
            "de backup hoort de inhoud van vóór deze schrijfactie te bevatten"
        );
        assert_eq!(std::fs::read(&path).expect("lezen"), b"nog nieuwer");
    }

    #[test]
    fn a_failed_write_leaves_no_backup_either() {
        let (tempdir, path) = file_in_a_directory();

        let result = replace(
            &path,
            Options { backup: true },
            "titel",
            write_replacement,
            |_temp| Err(CallerError("deugt niet")),
        );

        assert!(result.is_err());
        assert_eq!(
            names_in(tempdir.path()),
            vec!["track.mp3"],
            "er is een backup gemaakt van een schrijfactie die niet doorging"
        );
    }

    #[test]
    fn a_missing_file_is_a_filesystem_error() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let missing = tempdir.path().join("bestaat-niet.mp3");

        let result = replace(
            &missing,
            Options::default(),
            "titel",
            write_replacement,
            accept,
        );

        assert!(matches!(result, Err(WriteError::Filesystem(_))));
        assert!(names_in(tempdir.path()).is_empty());
    }

    type LogBuffer = std::sync::Arc<std::sync::Mutex<Vec<u8>>>;

    thread_local! {
        /// De buffer waarin de log van déze thread terechtkomt, als er een
        /// test meeluistert.
        static CAPTURE: std::cell::RefCell<Option<LogBuffer>> =
            const { std::cell::RefCell::new(None) };
    }

    /// Schrijft elke logregel naar de buffer van de thread die hem uitstuurt.
    struct PerThreadWriter;

    impl std::io::Write for PerThreadWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            CAPTURE.with(|slot| {
                if let Some(buffer) = slot.borrow().as_ref() {
                    buffer
                        .lock()
                        .expect("logbuffer moet schrijfbaar zijn")
                        .extend_from_slice(bytes);
                }
            });
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for PerThreadWriter {
        type Writer = PerThreadWriter;

        fn make_writer(&'a self) -> Self::Writer {
            PerThreadWriter
        }
    }

    /// Vangt de logregels op die `tracing` tijdens `work` op deze thread
    /// uitstuurt.
    ///
    /// De eis is dat elke schrijfactie gelogd wordt, en dat is alleen te
    /// bewijzen door werkelijk mee te lezen.
    ///
    /// Waarom één globale subscriber en niet `with_default` per test: `tracing`
    /// onthoudt per logregel-in-de-code of er überhaupt iemand luistert, en dat
    /// geheugen is globaal. Draaien er honderd tests naast elkaar, dan komt er
    /// een thread zónder subscriber langs diezelfde regel en blijft "niemand
    /// luistert" hangen — waarna de test mét subscriber een lege buffer
    /// overhoudt. Precies dat gebeurde hier. Eén globale subscriber die naar de
    /// buffer van de juiste thread schrijft, heeft dat probleem niet; regels
    /// van andere threads vallen op de grond.
    fn captured_log(work: impl FnOnce()) -> String {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let _ = tracing_subscriber::fmt()
                .with_writer(PerThreadWriter)
                .with_ansi(false)
                .with_max_level(tracing::Level::TRACE)
                .try_init();
        });

        let buffer = LogBuffer::default();
        CAPTURE.with(|slot| *slot.borrow_mut() = Some(std::sync::Arc::clone(&buffer)));

        work();

        CAPTURE.with(|slot| *slot.borrow_mut() = None);

        let bytes = buffer.lock().expect("logbuffer moet leesbaar zijn").clone();
        String::from_utf8(bytes).expect("de log moet UTF-8 zijn")
    }

    #[test]
    fn a_successful_write_is_logged_with_path_and_fields() {
        let (_tempdir, path) = file_in_a_directory();
        let target = path.clone();

        let log = captured_log(|| {
            replace(
                &target,
                Options::default(),
                "titel, artiest",
                write_replacement,
                accept,
            )
            .expect("schrijven moet lukken");
        });

        assert!(
            log.contains("track.mp3"),
            "het pad ontbreekt in de log:\n{log}"
        );
        assert!(
            log.contains("titel, artiest"),
            "de gewijzigde velden ontbreken in de log:\n{log}"
        );
    }

    #[test]
    fn a_failed_validation_is_logged_as_an_error() {
        // Dit is het geval waarin er een onbruikbaar bestand is geproduceerd;
        // dat hoort niet stilletjes voorbij te gaan.
        let (_tempdir, path) = file_in_a_directory();
        let target = path.clone();

        let log = captured_log(|| {
            let _ = replace(
                &target,
                Options::default(),
                "titel",
                write_replacement,
                |_temp| Err(CallerError("deugt niet")),
            );
        });

        assert!(log.contains("ERROR"), "de fout is niet gelogd:\n{log}");
        assert!(
            log.contains("track.mp3"),
            "het pad ontbreekt in de log:\n{log}"
        );
        assert!(
            log.contains("origineel is niet aangeraakt"),
            "de log zegt niet dat het origineel heel is:\n{log}"
        );
    }

    #[test]
    fn the_backup_keeps_the_original_name_recognisable() {
        assert_eq!(
            with_suffix(Path::new("/muziek/Album/track.mp3"), "bak"),
            Path::new("/muziek/Album/track.mp3.bak")
        );
        assert_eq!(
            with_suffix(Path::new("track.flac"), "bak"),
            Path::new("track.flac.bak")
        );
    }

    #[test]
    fn the_temporary_name_is_hidden_and_unique_per_process() {
        let temp = TempFile::beside(Path::new("/muziek/Album/track.mp3")).expect("naam");
        let name = temp
            .path()
            .file_name()
            .expect("bestandsnaam")
            .to_string_lossy()
            .into_owned();

        assert!(
            name.starts_with('.'),
            "de mapbrowser slaat alleen verborgen bestanden over: {name}"
        );
        assert!(name.contains("track.mp3"), "{name}");
        assert!(name.contains(&std::process::id().to_string()), "{name}");
        assert!(name.ends_with(TEMP_SUFFIX), "{name}");

        // De guard mag niet struikelen over een bestand dat nooit is gemaakt.
        drop(temp);
    }
}
