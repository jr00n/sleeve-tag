//! Bewerking van album art: decoderen, verkleinen en opnieuw encoderen.
//!
//! Dit is de enige module die de `image`-crate voor pixelbewerkingen gebruikt.
//! [`crate::tags`] haalt de ruwe bytes uit een audiobestand en beschrijft ze,
//! maar raakt de pixels niet aan; wat er daarna met een afbeelding gebeurt,
//! gebeurt hier.
//!
//! Bestanden worden hier niet geopend en niet geschreven: in en uit gaan bytes.

// Behalve `thumbnail` wachten de functies hier op de taken die album art
// uploaden en embedden.
#![allow(dead_code)]

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{ImageReader, Limits};

/// Kwaliteit waarmee een thumbnail wordt geëncodeerd.
///
/// Op een vakje van enkele tientallen pixels is het verschil met een hogere
/// waarde niet te zien, terwijl de bytes wel oplopen.
const THUMBNAIL_QUALITY: u8 = 75;

/// Bovengrens aan wat een decoder mag alloceren, in bytes.
///
/// Een afbeelding van 20000×20000 past in een paar honderd kilobyte aan
/// gecomprimeerde data, maar vraagt bij het uitpakken meer dan een gigabyte.
/// Op een NAS met weinig geheugen is dat het verschil tussen een lelijke
/// foutmelding en een gestorven container.
const MAX_DECODED_BYTES: u64 = 256 * 1024 * 1024;

/// Wat er mis kan gaan bij het bewerken van een afbeelding.
///
/// De melding bevat geen pad: hij kan in de browser belanden.
#[derive(Debug, thiserror::Error)]
pub enum ArtError {
    #[error("deze afbeelding kon niet gelezen worden")]
    Undecodable,

    #[error("deze afbeelding kon niet opgeslagen worden")]
    Unencodable,
}

/// Beschrijft een afbeelding zonder de pixels uit te pakken.
///
/// Leest alleen de header. Bedoeld voor het tonen van formaat en afmetingen.
pub fn dimensions(data: &[u8]) -> Result<(u32, u32), ArtError> {
    reader(data)?
        .into_dimensions()
        .map_err(|_| ArtError::Undecodable)
}

/// Verkleint een afbeelding tot hoogstens `max` pixels per as en geeft er een
/// JPEG van terug.
///
/// De beeldverhouding blijft behouden en er wordt alleen omlaag geschaald: een
/// afbeelding die al binnen `max` past, houdt zijn afmetingen. Het resultaat is
/// altijd JPEG: een thumbnail heeft geen transparantie nodig en JPEG levert de
/// kleinste bytes op. Het origineel in het audiobestand blijft hierbij
/// ongemoeid — dit is puur een weergavekopie.
pub fn thumbnail(data: &[u8], max: u32) -> Result<Vec<u8>, ArtError> {
    let image = reader(data)?.decode().map_err(|_| ArtError::Undecodable)?;

    // `thumbnail` past de afbeelding in het opgegeven vierkant met behoud van
    // de beeldverhouding, maar vergroot ook wanneer hij eronder zit. Een hoes
    // van 100 px opblazen naar 160 px kost bytes en levert geen scherpte op,
    // dus schalen gebeurt alleen omlaag.
    let small = if image.width() > max || image.height() > max {
        image.thumbnail(max, max)
    } else {
        image
    };

    // JPEG kent geen alfakanaal. Zonder deze omzetting weigert de encoder een
    // PNG met transparantie in plaats van hem plat te slaan.
    let rgb = small.to_rgb8();

    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, THUMBNAIL_QUALITY)
        .encode_image(&rgb)
        .map_err(|_| ArtError::Unencodable)?;

    Ok(out)
}

/// Een reader met een geheugengrens en een op de inhoud geraden formaat.
fn reader(data: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, ArtError> {
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|_| ArtError::Undecodable)?;

    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODED_BYTES);
    reader.limits(limits);

    Ok(reader)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testfixtures;

    /// De ingecheckte coverafbeeldingen zijn 300×300.
    const FIXTURE_SIZE: u32 = 300;

    fn fixture_bytes(name: &str) -> Vec<u8> {
        std::fs::read(testfixtures::fixture_path(name)).expect("fixture moet leesbaar zijn")
    }

    #[test]
    fn reads_the_dimensions_of_both_formats() {
        for name in [testfixtures::COVER_JPEG, testfixtures::COVER_PNG] {
            assert_eq!(
                dimensions(&fixture_bytes(name)).expect("afmetingen moeten leesbaar zijn"),
                (FIXTURE_SIZE, FIXTURE_SIZE),
                "fixture: {name}"
            );
        }
    }

    #[test]
    fn scales_a_larger_image_down() {
        let original = fixture_bytes(testfixtures::COVER_JPEG);

        let small = thumbnail(&original, 64).expect("verkleinen moet lukken");

        assert_eq!(
            dimensions(&small).expect("het resultaat moet leesbaar zijn"),
            (64, 64)
        );
    }

    #[test]
    fn keeps_the_aspect_ratio() {
        // Een liggende afbeelding hoort binnen het vierkant te passen, niet
        // uitgerekt te worden.
        let wide = image::RgbImage::new(400, 100);
        let mut source = Vec::new();
        image::DynamicImage::ImageRgb8(wide)
            .write_to(&mut Cursor::new(&mut source), image::ImageFormat::Png)
            .expect("testafbeelding moet te schrijven zijn");

        let small = thumbnail(&source, 100).expect("verkleinen moet lukken");

        assert_eq!(
            dimensions(&small).expect("het resultaat moet leesbaar zijn"),
            (100, 25)
        );
    }

    #[test]
    fn does_not_enlarge_a_smaller_image() {
        let original = fixture_bytes(testfixtures::COVER_JPEG);

        let same = thumbnail(&original, 1000).expect("verkleinen moet lukken");

        assert_eq!(
            dimensions(&same).expect("het resultaat moet leesbaar zijn"),
            (FIXTURE_SIZE, FIXTURE_SIZE),
            "een kleine hoes hoort niet opgeblazen te worden"
        );
    }

    #[test]
    fn always_produces_a_jpeg() {
        let png = fixture_bytes(testfixtures::COVER_PNG);

        let small = thumbnail(&png, 64).expect("verkleinen moet lukken");

        assert_eq!(
            image::guess_format(&small).expect("formaat moet te raden zijn"),
            image::ImageFormat::Jpeg,
            "een thumbnail is altijd JPEG, ook uit een PNG"
        );
    }

    #[test]
    fn a_thumbnail_is_far_smaller_than_a_real_cover() {
        // De hele reden voor het verkleinen: dertig hoezen van een halve
        // megabyte naar een telefoon sturen is geen thumbnail.
        //
        // De ingecheckte fixtures zijn hiervoor onbruikbaar — een egaal vlak
        // van 300×300 past in 1288 bytes, en daar valt niets meer af te halen.
        // Deze test maakt daarom een afbeelding die zich als een echte hoes
        // gedraagt: groot en met genoeg detail om niet weg te comprimeren.
        let original = noisy_png(1200);

        let small = thumbnail(&original, 160).expect("verkleinen moet lukken");

        assert_eq!(
            dimensions(&small).expect("het resultaat moet leesbaar zijn"),
            (160, 160)
        );
        assert!(
            small.len() * 10 < original.len(),
            "thumbnail is {} bytes, origineel {}",
            small.len(),
            original.len()
        );
    }

    /// Een vierkante PNG met detail in elke pixel, als PNG-bytes.
    ///
    /// Het patroon is deterministisch, zodat de test niet van toeval afhangt,
    /// maar rommelig genoeg om niet in een paar honderd bytes te passen.
    fn noisy_png(size: u32) -> Vec<u8> {
        let image = image::RgbImage::from_fn(size, size, |x, y| {
            image::Rgb([
                (x ^ y) as u8,
                (x.wrapping_mul(7) ^ y.wrapping_mul(13)) as u8,
                (x.wrapping_add(y).wrapping_mul(31)) as u8,
            ])
        });

        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("testafbeelding moet te schrijven zijn");
        bytes
    }

    #[test]
    fn refuses_data_that_is_not_an_image() {
        let nonsense = b"dit is geen afbeelding maar gewoon tekst";

        assert!(matches!(
            thumbnail(nonsense, 64),
            Err(ArtError::Undecodable)
        ));
        assert!(matches!(dimensions(nonsense), Err(ArtError::Undecodable)));
    }

    #[test]
    fn survives_a_truncated_image() {
        // Een half geschreven bestand mag nooit een paniek opleveren. Of het
        // een fout wordt, ligt aan de decoder: die van JPEG vult een afgekapte
        // afbeelding aan en levert alsnog pixels, die van PNG weigert. Beide
        // uitkomsten zijn goed, zolang wat eruit komt bruikbaar is.
        for name in [testfixtures::COVER_JPEG, testfixtures::COVER_PNG] {
            let original = fixture_bytes(name);
            let half = &original[..original.len() / 2];

            if let Ok(small) = thumbnail(half, 64) {
                assert!(
                    dimensions(&small).is_ok(),
                    "{name}: het resultaat is geen leesbare afbeelding"
                );
            }
        }
    }

    #[test]
    fn error_messages_do_not_leak_paths() {
        for error in [ArtError::Undecodable, ArtError::Unencodable] {
            let message = error.to_string();
            assert!(
                !message.contains('/'),
                "melding bevat een pad-achtige tekst: {message}"
            );
        }
    }
}
