//! Bewerking van album art: decoderen, verkleinen en opnieuw encoderen.
//!
//! Dit is de enige module die de `image`-crate voor pixelbewerkingen gebruikt.
//! [`crate::tags`] haalt de ruwe bytes uit een audiobestand en beschrijft ze,
//! maar raakt de pixels niet aan; wat er daarna met een afbeelding gebeurt,
//! gebeurt hier.
//!
//! Bestanden worden hier niet geopend en niet geschreven: in en uit gaan bytes.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, ImageReader};

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

    #[error("alleen JPEG en PNG kunnen als hoes worden gebruikt; dit bestand is dat niet")]
    UnsupportedFormat,

    #[error("deze afbeelding is {megabytes} MB en daarmee groter dan de toegestane {limit} MB")]
    TooLarge { megabytes: u64, limit: u32 },
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

/// De grenzen waarbinnen een aangeleverde hoes moet passen (FR-15).
///
/// Komen uit de configuratie: `MAX_ART_SIZE`, `ART_QUALITY` en
/// `MAX_UPLOAD_MB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Bovengrens per as; verkleinen behoudt de beeldverhouding.
    pub max_width: u32,
    pub max_height: u32,

    /// JPEG-kwaliteit waarmee een verkleinde hoes wordt gecodeerd.
    pub quality: u8,

    /// Bovengrens aan de aangeleverde bytes, in megabytes.
    pub max_upload_mb: u32,
}

/// Een hoes zoals hij het audiobestand in mag (FR-15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    /// Het MIME-type van [`Prepared::data`].
    pub mime: String,

    /// De bytes die geëmbed worden.
    pub data: Vec<u8>,

    pub width: u32,
    pub height: u32,

    /// De afmetingen zoals ze werden aangeleverd.
    pub original: (u32, u32),
}

impl Prepared {
    /// Of er verkleind is; bepaalt wat er tegen de gebruiker gezegd wordt.
    pub fn is_resized(&self) -> bool {
        self.original != (self.width, self.height)
    }
}

/// Controleert een aangeleverde afbeelding en maakt hem klaar om te embedden.
///
/// Drie dingen gebeuren hier, en niets meer:
///
/// 1. **Valideren.** Het formaat wordt uit de bytes zelf geraden en niet uit een
///    bestandsnaam of een `Content-Type`: een `.jpg` die in werkelijkheid een
///    zip is, hoort geweigerd te worden en niet in iemands muziekbibliotheek te
///    belanden. Alleen JPEG en PNG komen erdoor.
/// 2. **Verkleinen**, maar alleen wat te groot is. Een 3000×3000 scan in elk van
///    de twaalf tracks blaast een album op; wat al binnen de grens past, wordt
///    niet aangeraakt.
/// 3. **Hercoderen**, en alleen wanneer er verkleind is. Dan wordt het JPEG,
///    tenzij het origineel werkelijk doorzichtige pixels heeft: die zouden
///    zwart worden, en dat is een zichtbare wijziging die niemand vroeg.
///
/// Past de afbeelding al binnen de grenzen, dan komen de bytes **ongewijzigd**
/// terug — geen hercodering, geen kwaliteitsverlies, en een PNG blijft een PNG.
pub fn prepare(data: &[u8], limits: Limits) -> Result<Prepared, ArtError> {
    let allowed = u64::from(limits.max_upload_mb) * 1024 * 1024;
    if data.len() as u64 > allowed {
        return Err(ArtError::TooLarge {
            // Naar boven afronden: "0 MB" zou een onbegrijpelijke melding zijn.
            megabytes: (data.len() as u64).div_ceil(1024 * 1024),
            limit: limits.max_upload_mb,
        });
    }

    let format = image::guess_format(data).map_err(|_| ArtError::UnsupportedFormat)?;
    if !matches!(format, ImageFormat::Jpeg | ImageFormat::Png) {
        return Err(ArtError::UnsupportedFormat);
    }

    let image = reader(data)?.decode().map_err(|_| ArtError::Undecodable)?;
    let original = (image.width(), image.height());

    if image.width() <= limits.max_width && image.height() <= limits.max_height {
        return Ok(Prepared {
            mime: mime_of(format).to_string(),
            data: data.to_vec(),
            width: original.0,
            height: original.1,
            original,
        });
    }

    let small = image.thumbnail(limits.max_width, limits.max_height);
    let size = (small.width(), small.height());

    // Doorzichtigheid overleeft JPEG niet. Alleen wanneer er werkelijk
    // doorzichtige pixels in zitten is dat een reden om PNG te blijven: een
    // alfakanaal waarin alles ondoorzichtig is, kost alleen maar bytes.
    let (data, mime) = if has_transparency(&small) {
        (encode_png(&small)?, mime_of(ImageFormat::Png))
    } else {
        (
            encode_jpeg(&small, limits.quality)?,
            mime_of(ImageFormat::Jpeg),
        )
    };

    Ok(Prepared {
        mime: mime.to_string(),
        data,
        width: size.0,
        height: size.1,
        original,
    })
}

/// Of er werkelijk doorzichtige pixels in de afbeelding zitten.
///
/// Zonder alfakanaal is het antwoord meteen nee; anders wordt er gekeken, want
/// een alfakanaal zegt op zichzelf nog niets.
fn has_transparency(image: &DynamicImage) -> bool {
    if !image.color().has_alpha() {
        return false;
    }

    image.to_rgba8().pixels().any(|pixel| pixel.0[3] < u8::MAX)
}

fn encode_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, ArtError> {
    // JPEG kent geen alfakanaal; zonder deze omzetting weigert de encoder een
    // afbeelding die er wel een heeft.
    let rgb = image.to_rgb8();

    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality)
        .encode_image(&rgb)
        .map_err(|_| ArtError::Unencodable)?;

    Ok(out)
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, ArtError> {
    let mut out = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|_| ArtError::Unencodable)?;

    Ok(out)
}

/// Het MIME-type dat bij een formaat hoort.
fn mime_of(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "image/png",
        // Alleen JPEG en PNG komen door de controle heen; iets anders kan hier
        // niet binnenkomen.
        _ => "image/jpeg",
    }
}

/// Een reader met een geheugengrens en een op de inhoud geraden formaat.
fn reader(data: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, ArtError> {
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|_| ArtError::Undecodable)?;

    let mut limits = image::Limits::default();
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

    /// De grenzen zoals de app ze standaard hanteert.
    fn limits() -> Limits {
        Limits {
            max_width: 1000,
            max_height: 1000,
            quality: 85,
            max_upload_mb: 10,
        }
    }

    /// Een afbeelding van deze afmetingen, als PNG of JPEG.
    ///
    /// Ruis en geen egaal vlak: een egale afbeelding comprimeert zo goed dat
    /// een test over bytes er niets van leert.
    fn image_of(width: u32, height: u32, format: ImageFormat, transparent: bool) -> Vec<u8> {
        let mut canvas = image::RgbaImage::new(width, height);
        for (x, y, pixel) in canvas.enumerate_pixels_mut() {
            let alpha = if transparent && x == 0 && y == 0 {
                0
            } else {
                255
            };
            *pixel = image::Rgba([(x % 256) as u8, (y % 256) as u8, 90, alpha]);
        }

        let image = if transparent {
            DynamicImage::ImageRgba8(canvas)
        } else {
            DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(canvas).to_rgb8())
        };

        let mut out = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut out), format)
            .expect("testafbeelding moet te schrijven zijn");
        out
    }

    #[test]
    fn a_large_jpeg_is_scaled_down_and_stays_a_jpeg() {
        // AC #2 en #6.
        let source = image_of(2400, 2400, ImageFormat::Jpeg, false);

        let prepared = prepare(&source, limits()).expect("dit hoort te lukken");

        assert_eq!((prepared.width, prepared.height), (1000, 1000));
        assert_eq!(prepared.original, (2400, 2400));
        assert!(prepared.is_resized());
        assert_eq!(prepared.mime, "image/jpeg");
        assert_eq!(
            image::guess_format(&prepared.data).expect("formaat moet te raden zijn"),
            ImageFormat::Jpeg
        );
        assert!(
            prepared.data.len() < source.len(),
            "verkleinen hoort bytes te schelen: {} → {}",
            source.len(),
            prepared.data.len()
        );
    }

    #[test]
    fn a_large_png_without_transparency_becomes_a_jpeg() {
        // De keuze uit PRD §12: bij verkleinen wordt er naar JPEG gecodeerd,
        // want dat is wat er in twaalf tracks tegelijk terechtkomt.
        let source = image_of(1600, 1600, ImageFormat::Png, false);

        let prepared = prepare(&source, limits()).expect("dit hoort te lukken");

        assert_eq!(prepared.mime, "image/jpeg");
        assert_eq!((prepared.width, prepared.height), (1000, 1000));
    }

    #[test]
    fn a_large_png_with_transparency_stays_a_png() {
        // Doorzichtige pixels zouden zwart worden in JPEG; dat is een zichtbare
        // wijziging die niemand vroeg.
        let source = image_of(1600, 1600, ImageFormat::Png, true);

        let prepared = prepare(&source, limits()).expect("dit hoort te lukken");

        assert_eq!(prepared.mime, "image/png");
        assert_eq!(
            image::guess_format(&prepared.data).expect("formaat moet te raden zijn"),
            ImageFormat::Png
        );
        assert_eq!((prepared.width, prepared.height), (1000, 1000));
    }

    #[test]
    fn an_alpha_channel_without_transparent_pixels_still_becomes_a_jpeg() {
        // Een alfakanaal waarin alles ondoorzichtig is, kost alleen bytes.
        let mut canvas = image::RgbaImage::new(1200, 1200);
        for (x, y, pixel) in canvas.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x % 256) as u8, (y % 256) as u8, 10, 255]);
        }
        let mut source = Vec::new();
        DynamicImage::ImageRgba8(canvas)
            .write_to(&mut Cursor::new(&mut source), ImageFormat::Png)
            .expect("testafbeelding moet te schrijven zijn");

        assert_eq!(
            prepare(&source, limits())
                .expect("dit hoort te lukken")
                .mime,
            "image/jpeg"
        );
    }

    #[test]
    fn an_image_that_already_fits_comes_back_untouched() {
        // AC #2: wat al binnen de grens past, wordt niet aangeraakt — dus ook
        // niet opnieuw gecomprimeerd, en een PNG blijft een PNG.
        let source = fixture_bytes(testfixtures::COVER_PNG);

        let prepared = prepare(&source, limits()).expect("dit hoort te lukken");

        assert_eq!(prepared.data, source, "de bytes horen ongewijzigd te zijn");
        assert_eq!(prepared.mime, "image/png");
        assert_eq!(
            (prepared.width, prepared.height),
            (FIXTURE_SIZE, FIXTURE_SIZE)
        );
        assert!(!prepared.is_resized());
    }

    #[test]
    fn the_aspect_ratio_survives_the_resize() {
        let source = image_of(2000, 1000, ImageFormat::Jpeg, false);

        let prepared = prepare(&source, limits()).expect("dit hoort te lukken");

        assert_eq!((prepared.width, prepared.height), (1000, 500));
    }

    #[test]
    fn the_quality_setting_changes_the_result() {
        // AC #3: de kwaliteit is werkelijk instelbaar en geen dode knop.
        let source = image_of(2000, 2000, ImageFormat::Jpeg, false);

        let coarse = prepare(
            &source,
            Limits {
                quality: 40,
                ..limits()
            },
        )
        .expect("dit hoort te lukken");
        let fine = prepare(
            &source,
            Limits {
                quality: 95,
                ..limits()
            },
        )
        .expect("dit hoort te lukken");

        assert!(
            coarse.data.len() < fine.data.len(),
            "kwaliteit 40 hoort kleiner uit te vallen dan 95: {} vs {}",
            coarse.data.len(),
            fine.data.len()
        );
    }

    #[test]
    fn something_that_is_not_an_image_is_refused_without_panicking() {
        // AC #1 en #4: er wordt naar de bytes gekeken, niet naar een naam of
        // een aangeleverd content-type.
        for junk in [
            b"dit is gewoon tekst".to_vec(),
            b"PK\x03\x04 een zipbestand".to_vec(),
            Vec::new(),
        ] {
            let error = prepare(&junk, limits()).expect_err("dit hoort geweigerd te worden");
            assert!(
                matches!(error, ArtError::UnsupportedFormat),
                "onverwachte fout: {error}"
            );
        }
    }

    #[test]
    fn a_gif_is_refused_even_though_it_is_a_real_image() {
        // Alleen JPEG en PNG horen in een hoes; een geldige GIF is dat niet.
        // De magic bytes zijn genoeg: er wordt geweigerd vóór er iets wordt
        // uitgepakt.
        let mut source = b"GIF89a".to_vec();
        source.extend_from_slice(&[0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00]);

        assert!(matches!(
            prepare(&source, limits()).expect_err("dit hoort geweigerd te worden"),
            ArtError::UnsupportedFormat
        ));
    }

    #[test]
    fn a_truncated_image_never_panics() {
        // De header klopt, de rest niet. JPEG-decoders zijn tolerant en maken
        // er soms nog een half plaatje van; welke van de twee uitkomsten het
        // wordt maakt niet uit, zolang het geen panic is en wat eruit komt
        // leesbaar blijft.
        for name in [testfixtures::COVER_JPEG, testfixtures::COVER_PNG] {
            let source = fixture_bytes(name);
            let half = source[..source.len() / 2].to_vec();

            if let Ok(prepared) = prepare(&half, limits()) {
                assert!(
                    dimensions(&prepared.data).is_ok(),
                    "{name}: het resultaat is geen leesbare afbeelding"
                );
            }
        }
    }

    #[test]
    fn an_upload_over_the_limit_is_refused_before_it_is_decoded() {
        // AC #5: de grens geldt op de bytes, dus vóór er iets uitgepakt wordt.
        let source = image_of(300, 300, ImageFormat::Png, false);
        let limits = Limits {
            max_upload_mb: 1,
            ..limits()
        };

        let big = vec![0u8; 2 * 1024 * 1024];
        let error = prepare(&big, limits).expect_err("dit hoort geweigerd te worden");

        match error {
            ArtError::TooLarge { megabytes, limit } => {
                assert_eq!(megabytes, 2);
                assert_eq!(limit, 1);
            }
            other => panic!("onverwachte fout: {other}"),
        }

        // En wat er wél onder blijft, komt gewoon door.
        assert!(prepare(&source, limits).is_ok());
    }

    #[test]
    fn error_messages_do_not_leak_paths() {
        for error in [
            ArtError::Undecodable,
            ArtError::Unencodable,
            ArtError::UnsupportedFormat,
            ArtError::TooLarge {
                megabytes: 12,
                limit: 10,
            },
        ] {
            let message = error.to_string();
            assert!(
                !message.contains('/'),
                "melding bevat een pad-achtige tekst: {message}"
            );
        }
    }
}
