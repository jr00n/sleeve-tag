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
pub const MP3_ZONDER_TAGS: &str = "untagged.mp3";
/// MP3 met de volledige tagset uit het tagmodel, als ID3v2.4.
pub const MP3_MET_TAGS: &str = "tagged.mp3";
/// MP3 met volledige tags én een embedded front cover.
pub const MP3_MET_ART: &str = "tagged-with-art.mp3";
/// MP3 met uitsluitend een ID3v1-tag, zonder ID3v2.
pub const MP3_ID3V1_ONLY: &str = "id3v1-only.mp3";
/// MP3 waarvan de ID3v1-tag andere waarden bevat dan de ID3v2-tag.
///
/// Het schrijfpad moet die tegenstrijdigheid opruimen in plaats van hem te laten
/// staan: ID3v2 zegt "Stilte in D", ID3v1 zegt "Oude titel uit ID3v1".
pub const MP3_ID3V1_INCONSISTENT: &str = "id3v1-inconsistent.mp3";

/// FLAC zonder Vorbis-comments.
pub const FLAC_ZONDER_TAGS: &str = "untagged.flac";
/// FLAC met de volledige tagset uit het tagmodel.
pub const FLAC_MET_TAGS: &str = "tagged.flac";
/// FLAC met volledige tags én een embedded front cover.
pub const FLAC_MET_ART: &str = "tagged-with-art.flac";

/// Losse coverafbeeldingen, voor het testen van uploaden en embedden.
pub const COVER_JPEG: &str = "cover.jpg";
pub const COVER_PNG: &str = "cover.png";

/// Alle fixtures, zodat een test kan controleren dat er niets ontbreekt.
pub const ALLE_FIXTURES: &[&str] = &[
    MP3_ZONDER_TAGS,
    MP3_MET_TAGS,
    MP3_MET_ART,
    MP3_ID3V1_ONLY,
    MP3_ID3V1_INCONSISTENT,
    FLAC_ZONDER_TAGS,
    FLAC_MET_TAGS,
    FLAC_MET_ART,
    COVER_JPEG,
    COVER_PNG,
];

/// Pad naar een fixture in de repo.
///
/// Paniekt met een bruikbare melding wanneer de fixture ontbreekt. Stilzwijgend
/// overslaan zou een test groen laten die niets meer controleert.
pub fn fixture_pad(naam: &str) -> PathBuf {
    let pad = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(naam);

    assert!(
        pad.is_file(),
        "fixture '{naam}' ontbreekt op {}. Genereer hem opnieuw met tests/fixtures/genereer-fixtures.sh",
        pad.display()
    );

    pad
}

/// Kopieert een fixture naar `doelmap` en geeft het pad naar de kopie terug.
pub fn kopieer_naar(doelmap: &Path, naam: &str) -> PathBuf {
    let bron = fixture_pad(naam);
    let doel = doelmap.join(naam);

    std::fs::copy(&bron, &doel).unwrap_or_else(|fout| {
        panic!(
            "fixture '{naam}' kon niet naar {} gekopieerd worden: {fout}",
            doel.display()
        )
    });

    doel
}

/// Kopieert een fixture naar een verse tempdir.
///
/// De tempdir wordt teruggegeven en moet in de test in leven blijven: zodra hij
/// wordt opgeruimd, verdwijnt ook het gekopieerde bestand.
pub fn kopieer_naar_tempdir(naam: &str) -> (tempfile::TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    let pad = kopieer_naar(tempdir.path(), naam);
    (tempdir, pad)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alle_fixtures_zijn_aanwezig() {
        for naam in ALLE_FIXTURES {
            let pad = fixture_pad(naam);
            let omvang = std::fs::metadata(&pad)
                .expect("fixture moet leesbaar zijn")
                .len();
            assert!(omvang > 0, "fixture '{naam}' is leeg");
        }
    }

    #[test]
    fn de_fixtures_blijven_klein_genoeg_voor_git() {
        let totaal: u64 = ALLE_FIXTURES
            .iter()
            .map(|naam| {
                std::fs::metadata(fixture_pad(naam))
                    .expect("fixture moet leesbaar zijn")
                    .len()
            })
            .sum();

        assert!(
            totaal < 1024 * 1024,
            "de fixtures zijn samen {totaal} bytes; de richtlijn is onder 1 MB"
        );
    }

    #[test]
    fn kopie_is_identiek_aan_het_origineel() {
        let (_tempdir, kopie) = kopieer_naar_tempdir(MP3_MET_TAGS);

        let origineel =
            std::fs::read(fixture_pad(MP3_MET_TAGS)).expect("origineel moet leesbaar zijn");
        let gekopieerd = std::fs::read(&kopie).expect("kopie moet leesbaar zijn");

        assert_eq!(origineel, gekopieerd);
    }

    #[test]
    fn schrijven_naar_de_kopie_laat_het_origineel_ongemoeid() {
        let (_tempdir, kopie) = kopieer_naar_tempdir(MP3_MET_TAGS);
        let voor = std::fs::read(fixture_pad(MP3_MET_TAGS)).expect("origineel moet leesbaar zijn");

        std::fs::write(&kopie, b"overschreven").expect("kopie moet schrijfbaar zijn");

        let na = std::fs::read(fixture_pad(MP3_MET_TAGS)).expect("origineel moet leesbaar zijn");
        assert_eq!(voor, na, "de fixture in de repo is gewijzigd door een test");
    }

    #[test]
    fn meerdere_fixtures_passen_in_dezelfde_map() {
        // Een albummap bevat straks meerdere tracks; die moeten naast elkaar
        // kunnen staan zonder elkaar te overschrijven.
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");

        let eerste = kopieer_naar(tempdir.path(), MP3_MET_TAGS);
        let tweede = kopieer_naar(tempdir.path(), FLAC_MET_TAGS);

        assert!(eerste.is_file());
        assert!(tweede.is_file());
        assert_ne!(eerste, tweede);
    }

    #[test]
    #[should_panic(expected = "ontbreekt")]
    fn een_ontbrekende_fixture_laat_de_test_falen() {
        // Zonder deze controle zou een verdwenen fixture pas opvallen als een
        // heel andere test op een raadselachtige manier faalt.
        fixture_pad("deze-fixture-bestaat-niet.mp3");
    }
}
