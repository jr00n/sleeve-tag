//! Tag-I/O en het genormaliseerde tagmodel.
//!
//! Dit is de enige module die `lofty` aanroept en de enige module die
//! audiobestanden muteert. De rest van de applicatie werkt uitsluitend met het
//! genormaliseerde model uit PRD §7 en weet niet of een bestand ID3v2-frames of
//! Vorbis-comments bevat.
//!
//! Vaste regels uit het PRD:
//! - MP3 wordt altijd weggeschreven als ID3v2.4 (UTF-8); ID3v1 wordt verwijderd
//!   of gesynchroniseerd, nooit inconsistent achtergelaten.
//! - Niet-gemodelleerde tags blijven ongewijzigd bewaard.
//! - Een leeg veld betekent "veld verwijderen", niet "lege waarde opslaan".
//!
//! Het schrijven volgt in fase 2; deze module leest.

// De mapbrowser en de bestandsweergave zijn de eerste gebruikers van dit model;
// tot die taken roepen alleen de tests het aan. De regel hoort daar weg te gaan.
#![allow(dead_code)]

use std::fmt;
use std::path::Path;
use std::time::Duration;

use lofty::config::ParseOptions;
use lofty::file::{AudioFile, FileType, TaggedFile, TaggedFileExt};
use lofty::picture::{Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, ItemValue, Tag};

/// Wat er mis kan gaan bij het lezen van een bestand.
///
/// De meldingen bevatten geen pad: ze kunnen in de browser belanden, en het
/// absolute pad van de NAS hoort daar niet thuis. De aanroeper logt het pad
/// erbij.
#[derive(Debug, thiserror::Error)]
pub enum TagError {
    #[error("het bestand kon niet gelezen worden")]
    Unreadable,

    #[error("dit bestandstype wordt niet ondersteund")]
    UnsupportedFormat,
}

/// De containerformaten die Sleeve ondersteunt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mp3,
    Flac,
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Format::Mp3 => write!(f, "MP3"),
            Format::Flac => write!(f, "FLAC"),
        }
    }
}

/// Het genormaliseerde tagmodel uit PRD §7.
///
/// Elk veld is `Option`: `None` betekent dat de tag niet in het bestand staat.
/// Een tag die wél bestaat maar leeg is, komt ook als `None` terug — het PRD
/// behandelt een leeg veld als "verwijderd", dus een lege waarde draagt geen
/// informatie die de app moet bewaren.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub track: Option<u32>,
    pub track_total: Option<u32>,
    pub disc: Option<u32>,
    pub disc_total: Option<u32>,
    /// Als tekst bewaard: `TDRC` en `DATE` mogen een volledige datum bevatten,
    /// en die mag niet stilzwijgend sneuvelen door hem als jaartal te parsen.
    pub year: Option<String>,
    pub genre: Option<String>,
    pub composer: Option<String>,
    pub comment: Option<String>,
}

impl Tags {
    /// Of er überhaupt iets ingevuld is.
    ///
    /// De maplijst gebruikt dit om bestanden zonder tags te markeren.
    pub fn is_empty(&self) -> bool {
        *self == Tags::default()
    }
}

/// Wat er over de embedded front cover bekend is, zonder de afbeelding zelf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtInfo {
    /// MIME-type zoals het in het bestand staat, bijvoorbeeld `image/jpeg`.
    pub mime: String,
    pub width: u32,
    pub height: u32,
    /// Omvang van de afbeeldingsdata in bytes.
    pub bytes: usize,
}

/// Alles wat Sleeve over één audiobestand weet.
#[derive(Debug, Clone)]
pub struct Track {
    pub format: Format,
    pub duration: Duration,
    pub tags: Tags,
    pub art: Option<ArtInfo>,
}

/// Eén ruwe tag zoals die werkelijk in het bestand staat.
///
/// Bedoeld voor de geavanceerde weergave: hier staat `TPE1` of `ALBUMARTIST`,
/// niet de genormaliseerde veldnaam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTag {
    pub key: String,
    pub value: String,
}

/// Leest het volledige model van één bestand.
pub fn read(path: &Path) -> Result<Track, TagError> {
    let file = open(path)?;
    let format = format_of(&file)?;
    let duration = file.properties().duration();

    let tag = primary_tag(&file);
    let tags = tag.map(read_tags).unwrap_or_default();
    let art = tag.and_then(front_cover).and_then(describe_art);

    Ok(Track {
        format,
        duration,
        tags,
        art,
    })
}

/// Geeft de ruwe bytes van de embedded front cover.
///
/// Apart van [`read`], zodat het serveren van een thumbnail niet het hele
/// tagmodel hoeft op te bouwen.
pub fn read_front_cover(path: &Path) -> Result<Option<(String, Vec<u8>)>, TagError> {
    let file = open(path)?;

    let Some(picture) = primary_tag(&file).and_then(front_cover) else {
        return Ok(None);
    };

    Ok(Some((mime_of(picture), picture.data().to_vec())))
}

/// Geeft alle aanwezige tags met hun oorspronkelijke sleutelnaam.
///
/// Binaire waarden worden samengevat in plaats van uitgeschreven: een
/// APIC-frame van veertig kilobyte hoort niet in een HTML-tabel.
pub fn read_raw_tags(path: &Path) -> Result<Vec<RawTag>, TagError> {
    let file = open(path)?;
    let Some(tag) = primary_tag(&file) else {
        return Ok(Vec::new());
    };

    let tag_type = tag.tag_type();
    let mut raw: Vec<RawTag> = tag
        .items()
        .map(|item| {
            let key = item
                .key()
                .map_key(tag_type)
                .unwrap_or("(onbekend)")
                .to_string();

            let value = match item.value() {
                ItemValue::Text(text) | ItemValue::Locator(text) => text.clone(),
                ItemValue::Binary(data) => format!("({} bytes binaire data)", data.len()),
            };

            RawTag { key, value }
        })
        .collect();

    for picture in tag.pictures() {
        raw.push(RawTag {
            key: format!("{:?}", picture.pic_type()),
            value: format!("{}, {} bytes", mime_of(picture), picture.data().len()),
        });
    }

    // Een vaste volgorde maakt de weergave voorspelbaar en de tests stabiel.
    raw.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(raw)
}

/// Bepaalt of het bestand werkelijk een MP3 of FLAC is.
///
/// Kijkt naar de inhoud, niet naar de bestandsnaam: een `.mp3` die in
/// werkelijkheid een JPEG of een tekstbestand is, hoort de app niet als
/// bewerkbaar te presenteren. De aanroeper wil hier alleen een ja of nee, dus
/// een fout wordt tot `false` teruggebracht.
pub fn is_supported_format(path: &Path) -> bool {
    read(path).is_ok()
}

/// Opent en parseert een bestand.
///
/// Raden op basis van de eerste bytes is niet genoeg: `guess_file_type` valt
/// terug op de extensie, en de JPEG-signatuur `FF D8` lijkt genoeg op een MPEG
/// frame-sync om als MP3 door te gaan — een JPEG wordt zo als Mpeg geraden. Pas
/// bij het uitlezen van de audio-eigenschappen valt door de mand dat er geen
/// geldige frames in zitten. `read_properties` is hier dus geen luxe maar de
/// eigenlijke controle.
fn open(path: &Path) -> Result<TaggedFile, TagError> {
    let probe = Probe::open(path)
        .map_err(|_| TagError::Unreadable)?
        .guess_file_type()
        .map_err(|_| TagError::Unreadable)?;

    if !matches!(probe.file_type(), Some(FileType::Mpeg | FileType::Flac)) {
        return Err(TagError::UnsupportedFormat);
    }

    probe
        .options(ParseOptions::new().read_properties(true))
        .read()
        .map_err(|_| TagError::UnsupportedFormat)
}

fn format_of(file: &TaggedFile) -> Result<Format, TagError> {
    match file.file_type() {
        FileType::Mpeg => Ok(Format::Mp3),
        FileType::Flac => Ok(Format::Flac),
        _ => Err(TagError::UnsupportedFormat),
    }
}

/// De tag waar de applicatie mee werkt.
///
/// Voor MP3 is dat ID3v2 en voor FLAC de Vorbis-comments. Een MP3 met alleen
/// een ID3v1-tag heeft geen primaire tag; dan valt de keuze op de eerste die er
/// wel is, zodat die waarden niet onzichtbaar blijven.
fn primary_tag(file: &TaggedFile) -> Option<&Tag> {
    file.primary_tag().or_else(|| file.first_tag())
}

fn read_tags(tag: &Tag) -> Tags {
    Tags {
        title: text(tag.title().as_deref()),
        artist: text(tag.artist().as_deref()),
        album_artist: text(tag.get_string(ItemKey::AlbumArtist)),
        album: text(tag.album().as_deref()),
        track: tag.track(),
        track_total: tag.track_total(),
        disc: tag.disk(),
        disc_total: tag.disk_total(),
        // TDRC en DATE komen als RecordingDate binnen; sommige bestanden
        // hebben alleen een kaal jaartal in een apart veld.
        year: text(tag.get_string(ItemKey::RecordingDate))
            .or_else(|| text(tag.get_string(ItemKey::Year))),
        genre: text(tag.genre().as_deref()),
        composer: text(tag.get_string(ItemKey::Composer)),
        // Vorbis kent zowel COMMENT als DESCRIPTION voor hetzelfde doel; ffmpeg
        // schrijft het tweede, Picard het eerste. Beide lezen scheelt de
        // gebruiker een veld dat onverklaarbaar leeg blijft.
        comment: text(tag.comment().as_deref())
            .or_else(|| text(tag.get_string(ItemKey::Description))),
    }
}

/// Maakt van een tagwaarde een `Option`, waarbij leeg als afwezig telt.
fn text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// De front cover, of anders de eerste afbeelding die er is.
///
/// Het PRD werkt met type 3 (front cover). Een bestand waarin de hoes een ander
/// type heeft gekregen, is geen reden om niets te tonen.
fn front_cover(tag: &Tag) -> Option<&Picture> {
    tag.pictures()
        .iter()
        .find(|picture| picture.pic_type() == PictureType::CoverFront)
        .or_else(|| tag.pictures().first())
}

fn mime_of(picture: &Picture) -> String {
    picture
        .mime_type()
        .map(ToString::to_string)
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

/// Leest de afmetingen uit de header van de afbeelding.
///
/// `into_dimensions` leest alleen de header en decodeert de pixels niet. Dat is
/// wat de maplijst nodig heeft: die toont per bestand of er art is en hoe groot,
/// en mag daarvoor geen dertig afbeeldingen uitpakken.
fn describe_art(picture: &Picture) -> Option<ArtInfo> {
    let data = picture.data();

    let (width, height) = image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;

    Some(ArtInfo {
        mime: mime_of(picture),
        width,
        height,
        bytes: data.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testfixtures;

    fn read_fixture(name: &str) -> Track {
        let path = testfixtures::fixture_path(name);
        read(&path).unwrap_or_else(|error| panic!("{name} moet leesbaar zijn: {error}"))
    }

    #[test]
    fn reads_the_full_model_from_a_tagged_mp3() {
        let track = read_fixture(testfixtures::MP3_WITH_TAGS);

        assert_eq!(track.format, Format::Mp3);
        assert!(
            track.duration.as_millis() > 900,
            "duur: {:?}",
            track.duration
        );

        let tags = track.tags;
        assert_eq!(tags.title.as_deref(), Some("Stilte in D"));
        assert_eq!(tags.artist.as_deref(), Some("De Testartiest"));
        assert_eq!(tags.album_artist.as_deref(), Some("De Albumartiest"));
        assert_eq!(tags.album.as_deref(), Some("Fixtures voor Sleeve"));
        assert_eq!(tags.track, Some(3));
        assert_eq!(tags.track_total, Some(12));
        assert_eq!(tags.disc, Some(1));
        assert_eq!(tags.disc_total, Some(2));
        assert_eq!(tags.genre.as_deref(), Some("Ambient"));
        assert_eq!(tags.composer.as_deref(), Some("De Componist"));
        assert!(
            tags.year
                .as_deref()
                .is_some_and(|year| year.contains("2024")),
            "jaar was: {:?}",
            tags.year
        );
        assert!(
            tags.comment
                .as_deref()
                .is_some_and(|comment| comment.contains("Gegenereerd")),
            "commentaar was: {:?}",
            tags.comment
        );
    }

    #[test]
    fn mp3_and_flac_yield_the_same_model() {
        // Dit is de kern van het genormaliseerde model: de rest van de app mag
        // niet kunnen zien welk containerformaat eronder ligt.
        let mp3 = read_fixture(testfixtures::MP3_WITH_TAGS);
        let flac = read_fixture(testfixtures::FLAC_WITH_TAGS);

        assert_eq!(mp3.tags.title, flac.tags.title);
        assert_eq!(mp3.tags.artist, flac.tags.artist);
        assert_eq!(mp3.tags.album_artist, flac.tags.album_artist);
        assert_eq!(mp3.tags.album, flac.tags.album);
        assert_eq!(mp3.tags.track, flac.tags.track);
        assert_eq!(mp3.tags.track_total, flac.tags.track_total);
        assert_eq!(mp3.tags.disc, flac.tags.disc);
        assert_eq!(mp3.tags.disc_total, flac.tags.disc_total);
        assert_eq!(mp3.tags.genre, flac.tags.genre);
        assert_eq!(mp3.tags.composer, flac.tags.composer);

        assert_ne!(mp3.format, flac.format);
    }

    #[test]
    fn splits_combined_track_and_disc_fields() {
        // TRCK bevat "3/12" in één frame; FLAC heeft er twee velden voor. Beide
        // moeten hetzelfde opleveren.
        for name in [testfixtures::MP3_WITH_TAGS, testfixtures::FLAC_WITH_TAGS] {
            let tags = read_fixture(name).tags;
            assert_eq!(tags.track, Some(3), "{name}");
            assert_eq!(tags.track_total, Some(12), "{name}");
            assert_eq!(tags.disc, Some(1), "{name}");
            assert_eq!(tags.disc_total, Some(2), "{name}");
        }
    }

    #[test]
    fn untagged_files_have_no_fields_at_all() {
        for name in [
            testfixtures::MP3_WITHOUT_TAGS,
            testfixtures::FLAC_WITHOUT_TAGS,
        ] {
            let track = read_fixture(name);

            assert!(
                track.tags.is_empty(),
                "{name} zou geen tags moeten hebben, kreeg {:?}",
                track.tags
            );
            // Afwezig, niet een lege string: het verschil telt bij het schrijven.
            assert_eq!(track.tags.title, None, "{name}");
            assert_eq!(track.tags.artist, None, "{name}");
            assert_eq!(track.art, None, "{name}");
        }
    }

    #[test]
    fn reports_art_metadata_without_decoding_the_image() {
        for name in [testfixtures::MP3_WITH_ART, testfixtures::FLAC_WITH_ART] {
            let art = read_fixture(name)
                .art
                .unwrap_or_else(|| panic!("{name} heeft embedded art"));

            assert_eq!(art.mime, "image/jpeg", "{name}");
            assert_eq!(art.width, 300, "{name}");
            assert_eq!(art.height, 300, "{name}");
            assert!(art.bytes > 0, "{name}");
        }
    }

    #[test]
    fn returns_the_raw_cover_bytes() {
        let path = testfixtures::fixture_path(testfixtures::MP3_WITH_ART);
        let (mime, data) = read_front_cover(&path)
            .expect("lezen moet lukken")
            .expect("er is een cover");

        assert_eq!(mime, "image/jpeg");
        // JPEG begint met FF D8; zo weten we dat het de afbeelding zelf is.
        assert_eq!(&data[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn reports_no_cover_when_there_is_none() {
        let path = testfixtures::fixture_path(testfixtures::MP3_WITH_TAGS);
        assert_eq!(read_front_cover(&path).expect("lezen moet lukken"), None);
    }

    #[test]
    fn raw_tags_use_the_original_key_names() {
        let mp3 = read_raw_tags(&testfixtures::fixture_path(testfixtures::MP3_WITH_TAGS))
            .expect("lezen moet lukken");
        let keys: Vec<&str> = mp3.iter().map(|tag| tag.key.as_str()).collect();

        // ID3v2 gebruikt frame-ID's, geen genormaliseerde veldnamen.
        assert!(keys.contains(&"TIT2"), "sleutels waren: {keys:?}");
        assert!(keys.contains(&"TPE1"), "sleutels waren: {keys:?}");

        let flac = read_raw_tags(&testfixtures::fixture_path(testfixtures::FLAC_WITH_TAGS))
            .expect("lezen moet lukken");
        let keys: Vec<&str> = flac.iter().map(|tag| tag.key.as_str()).collect();

        // Vorbis-comments gebruiken hun eigen namen.
        assert!(keys.contains(&"TITLE"), "sleutels waren: {keys:?}");
        assert!(keys.contains(&"ARTIST"), "sleutels waren: {keys:?}");
    }

    #[test]
    fn raw_tags_summarise_binary_values() {
        let raw = read_raw_tags(&testfixtures::fixture_path(testfixtures::MP3_WITH_ART))
            .expect("lezen moet lukken");

        let art = raw
            .iter()
            .find(|tag| tag.value.contains("bytes"))
            .expect("de cover hoort in de ruwe lijst te staan");

        // De data zelf hoort er niet in; alleen een samenvatting.
        assert!(art.value.contains("image/jpeg"), "waarde: {}", art.value);
        assert!(art.value.len() < 100, "waarde is te lang: {}", art.value);
    }

    #[test]
    fn reads_values_from_an_id3v1_only_file() {
        // Zonder ID3v2 mag de app de ID3v1-waarden niet stilzwijgend negeren.
        let track = read_fixture(testfixtures::MP3_ID3V1_ONLY);
        assert_eq!(track.tags.title.as_deref(), Some("Stilte in D"));
    }

    #[test]
    fn prefers_id3v2_when_id3v1_disagrees() {
        // De fixture heeft "Stilte in D" in ID3v2 en "Oude titel uit ID3v1" in
        // ID3v1. ID3v2 is leidend; het opruimen van die tegenstrijdigheid is
        // werk voor het schrijfpad.
        let track = read_fixture(testfixtures::MP3_ID3V1_INCONSISTENT);
        assert_eq!(track.tags.title.as_deref(), Some("Stilte in D"));
    }

    #[test]
    fn rejects_a_file_that_is_not_audio() {
        let path = testfixtures::fixture_path(testfixtures::COVER_JPEG);

        let error = read(&path).expect_err("een JPEG is geen audiobestand");
        assert!(matches!(error, TagError::UnsupportedFormat), "{error}");
    }

    #[test]
    fn rejects_a_missing_file() {
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        let error = read(&tempdir.path().join("bestaat-niet.mp3"))
            .expect_err("een ontbrekend bestand kan niet gelezen worden");

        assert!(matches!(error, TagError::Unreadable), "{error}");
    }

    #[test]
    fn error_messages_do_not_leak_paths() {
        for error in [TagError::Unreadable, TagError::UnsupportedFormat] {
            assert!(
                !error.to_string().contains('/'),
                "melding bevat een pad: {error}"
            );
        }
    }

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
        let fake = tempdir.path().join("nep.mp3");
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
