//! Toegang tot de testfixtures onder `tests/fixtures/`.
//!
//! Tests draaien **nooit** tegen de echte muziekbibliotheek. Ze kopiëren een
//! fixture naar een tempdir en werken daar; het origineel in de repo blijft
//! ongemoeid, ook als een test schrijft.
//!
//! De fixtures zijn één seconde stilte, gegenereerd met
//! `tests/fixtures/genereer-fixtures.sh`. Wat ze onderscheidt zijn hun tags.

use std::path::{Path, PathBuf};

/// MP3 zonder enige tag: geen ID3v2 aan het begin, geen ID3v1 aan het eind.
pub const MP3_WITHOUT_TAGS: &str = "untagged.mp3";
/// MP3 met de volledige tagset uit het tagmodel, als ID3v2.4.
pub const MP3_WITH_TAGS: &str = "tagged.mp3";
/// MP3 met volledige tags én een embedded front cover.
pub const MP3_WITH_ART: &str = "tagged-with-art.mp3";
/// MP3 met uitsluitend een ID3v1-tag, zonder ID3v2.
pub const MP3_ID3V1_ONLY: &str = "id3v1-only.mp3";
/// MP3 waarvan de ID3v1-tag andere waarden bevat dan de ID3v2-tag.
///
/// Het schrijfpad moet die tegenstrijdigheid opruimen in plaats van hem te laten
/// staan: ID3v2 zegt "Stilte in D", ID3v1 zegt "Oude titel uit ID3v1".
pub const MP3_ID3V1_INCONSISTENT: &str = "id3v1-inconsistent.mp3";

/// FLAC zonder Vorbis-comments.
pub const FLAC_WITHOUT_TAGS: &str = "untagged.flac";
/// FLAC met de volledige tagset uit het tagmodel.
pub const FLAC_WITH_TAGS: &str = "tagged.flac";
/// FLAC met volledige tags én een embedded front cover.
pub const FLAC_WITH_ART: &str = "tagged-with-art.flac";

/// Losse coverafbeeldingen, voor het testen van uploaden en embedden.
pub const COVER_JPEG: &str = "cover.jpg";
pub const COVER_PNG: &str = "cover.png";

/// Alle fixtures, zodat een test kan controleren dat er niets ontbreekt.
pub const ALL_FIXTURES: &[&str] = &[
    MP3_WITHOUT_TAGS,
    MP3_WITH_TAGS,
    MP3_WITH_ART,
    MP3_ID3V1_ONLY,
    MP3_ID3V1_INCONSISTENT,
    FLAC_WITHOUT_TAGS,
    FLAC_WITH_TAGS,
    FLAC_WITH_ART,
    COVER_JPEG,
    COVER_PNG,
];

/// Pad naar een fixture in de repo.
///
/// Paniekt met een bruikbare melding wanneer de fixture ontbreekt. Stilzwijgend
/// overslaan zou een test groen laten die niets meer controleert.
pub fn fixture_path(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);

    assert!(
        path.is_file(),
        "fixture '{name}' ontbreekt op {}. Genereer hem opnieuw met tests/fixtures/genereer-fixtures.sh",
        path.display()
    );

    path
}

/// Kopieert een fixture naar `doelmap` en geeft het pad naar de kopie terug.
pub fn copy_to(doelmap: &Path, name: &str) -> PathBuf {
    let source = fixture_path(name);
    let target = doelmap.join(name);

    std::fs::copy(&source, &target).unwrap_or_else(|error| {
        panic!(
            "fixture '{name}' kon niet naar {} gekopieerd worden: {error}",
            target.display()
        )
    });

    target
}

/// Kopieert een fixture naar een verse tempdir.
///
/// De tempdir wordt teruggegeven en moet in de test in leven blijven: zodra hij
/// wordt opgeruimd, verdwijnt ook het gekopieerde bestand.
pub fn copy_to_tempdir(name: &str) -> (tempfile::TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let path = copy_to(tempdir.path(), name);
    (tempdir, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fixtures_are_present() {
        for name in ALL_FIXTURES {
            let path = fixture_path(name);
            let size = std::fs::metadata(&path)
                .expect("fixture moet leesbaar zijn")
                .len();
            assert!(size > 0, "fixture '{name}' is leeg");
        }
    }

    #[test]
    fn fixtures_stay_small_enough_for_git() {
        let total: u64 = ALL_FIXTURES
            .iter()
            .map(|name| {
                std::fs::metadata(fixture_path(name))
                    .expect("fixture moet leesbaar zijn")
                    .len()
            })
            .sum();

        assert!(
            total < 1024 * 1024,
            "de fixtures zijn samen {total} bytes; de richtlijn is onder 1 MB"
        );
    }

    #[test]
    fn copy_is_identical_to_the_original() {
        let (_tempdir, copy) = copy_to_tempdir(MP3_WITH_TAGS);

        let original =
            std::fs::read(fixture_path(MP3_WITH_TAGS)).expect("origineel moet leesbaar zijn");
        let copied = std::fs::read(&copy).expect("kopie moet leesbaar zijn");

        assert_eq!(original, copied);
    }

    #[test]
    fn writing_to_the_copy_leaves_the_original_untouched() {
        let (_tempdir, copy) = copy_to_tempdir(MP3_WITH_TAGS);
        let before =
            std::fs::read(fixture_path(MP3_WITH_TAGS)).expect("origineel moet leesbaar zijn");

        std::fs::write(&copy, b"overschreven").expect("kopie moet schrijfbaar zijn");

        let after =
            std::fs::read(fixture_path(MP3_WITH_TAGS)).expect("origineel moet leesbaar zijn");
        assert_eq!(
            before, after,
            "de fixture in de repo is gewijzigd door een test"
        );
    }

    #[test]
    fn multiple_fixtures_fit_in_one_directory() {
        // Een albummap bevat straks meerdere tracks; die moeten naast elkaar
        // kunnen staan zonder elkaar te overschrijven.
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");

        let eerste = copy_to(tempdir.path(), MP3_WITH_TAGS);
        let tweede = copy_to(tempdir.path(), FLAC_WITH_TAGS);

        assert!(eerste.is_file());
        assert!(tweede.is_file());
        assert_ne!(eerste, tweede);
    }

    #[test]
    #[should_panic(expected = "ontbreekt")]
    fn a_missing_fixture_fails_the_test() {
        // Zonder deze controle zou een verdwenen fixture pas opvallen als een
        // heel andere test op een raadselachtige manier faalt.
        fixture_path("deze-fixture-bestaat-niet.mp3");
    }
}
