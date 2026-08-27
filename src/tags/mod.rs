//! Tag-I/O en het genormaliseerde tagmodel.
//!
//! Dit is de enige module die `lofty` aanroept en de enige module die
//! audiobestanden muteert. De rest van de applicatie werkt uitsluitend met het
//! genormaliseerde model en weet niet of een bestand ID3v2-frames of
//! Vorbis-comments bevat.
//!
//! Vaste regels uit het PRD:
//! - MP3 wordt altijd weggeschreven als ID3v2.4 (UTF-8); ID3v1 wordt verwijderd
//!   of gesynchroniseerd, nooit inconsistent achtergelaten.
//! - Niet-gemodelleerde tags blijven ongewijzigd bewaard.
//! - Een leeg veld betekent "veld verwijderen", niet "lege waarde opslaan".
//!
//! Het lezen en schrijven van het tagmodel volgt in de taken van fase 1 en 2.

use std::path::Path;

use lofty::config::ParseOptions;
use lofty::file::FileType;
use lofty::probe::Probe;

/// Bepaalt of het bestand werkelijk een MP3 of FLAC is.
///
/// Kijkt naar de inhoud, niet naar de bestandsnaam: een `.mp3` die in
/// werkelijkheid een JPEG of een tekstbestand is, hoort de app niet als
/// bewerkbaar te presenteren. Een onleesbaar of onbekend bestand levert `false`
/// op in plaats van een fout — de aanroeper wil hier alleen een ja of nee.
///
/// Raden op basis van de eerste bytes is hiervoor niet genoeg: `guess_file_type`
/// valt terug op de extensie, en de JPEG-signatuur `FF D8` lijkt genoeg op een
/// MPEG frame-sync om als MP3 door te gaan — een JPEG wordt zo als Mpeg geraden.
/// Pas bij het uitlezen van de audio-eigenschappen valt door de mand dat er geen
/// geldige frames in zitten. Die stap is hier dus geen luxe maar de eigenlijke
/// controle.
pub fn is_supported_format(path: &Path) -> bool {
    let Ok(probe) = Probe::open(path) else {
        return false;
    };
    let Ok(probe) = probe.guess_file_type() else {
        return false;
    };

    if !matches!(probe.file_type(), Some(FileType::Mpeg | FileType::Flac)) {
        return false;
    }

    probe
        .options(ParseOptions::new().read_properties(true))
        .read()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testfixtures;

    #[test]
    fn recognizes_mp3_and_flac_fixtures() {
        for name in [
            testfixtures::MP3_WITH_TAGS,
            testfixtures::MP3_WITHOUT_TAGS,
            testfixtures::MP3_WITH_ART,
            testfixtures::FLAC_WITH_TAGS,
            testfixtures::FLAC_WITHOUT_TAGS,
            testfixtures::FLAC_WITH_ART,
        ] {
            let path = testfixtures::fixture_path(name);
            assert!(
                is_supported_format(&path),
                "{name} zou herkend moeten worden"
            );
        }
    }

    #[test]
    fn rejects_an_image() {
        let path = testfixtures::fixture_path(testfixtures::COVER_JPEG);
        assert!(!is_supported_format(&path));
    }

    #[test]
    fn rejects_a_file_with_a_misleading_extension() {
        // Een tekstbestand dat zich voordoet als MP3. De extensie klopt, de
        // inhoud niet; alleen naar de naam kijken is dus niet genoeg.
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        let fake = tempdir.path().join("fake.mp3");
        std::fs::write(&fake, b"dit is geen audio").expect("bestand moet te schrijven zijn");

        assert!(!is_supported_format(&fake));
    }

    #[test]
    fn rejects_a_missing_path() {
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        assert!(!is_supported_format(
            &tempdir.path().join("bestaat-niet.mp3")
        ));
    }
}
