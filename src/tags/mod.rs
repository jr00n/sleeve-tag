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
//! Het schrijven loopt via [`write`], en die gaat op zijn beurt door
//! [`crate::atomic::replace`]: een half geschreven bestand is daarmee onmogelijk.

// De bestandsweergave gebruikt nog niet alles uit dit model; het bewerkformulier
// (task-14) is de eerste die ook schrijft.
#![allow(dead_code)]

use std::fmt;
use std::path::Path;
use std::time::Duration;

use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::{AudioFile, FileType, TaggedFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, ItemValue, Tag, TagExt, TagType};

use crate::atomic;

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

    #[error("de tags konden niet weggeschreven worden")]
    Unwritable,

    /// Wat er teruggelezen werd, is niet wat er bedoeld was.
    ///
    /// Dit is de vangnetfout van de hervalidatie. Hij hoort niet voor te komen;
    /// gebeurt het toch, dan blijft het originele bestand ongemoeid.
    #[error("de weggeschreven tags kwamen niet terug zoals bedoeld")]
    Mismatch,
}

/// Wat er mis kan gaan bij het schrijven van tags.
///
/// Het onderscheid uit [`crate::atomic`] blijft zichtbaar: een fout tijdens het
/// klaarmaken betekent dat er niets is gebeurd, een fout tijdens de
/// hervalidatie betekent dat er zojuist een onbruikbaar bestand is gemaakt —
/// dat nooit over het origineel heen is gegaan.
pub type WriteError = atomic::WriteError<TagError>;

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

    /// Dezelfde tags, maar zoals ze in een bestand terecht mogen komen.
    ///
    /// Elke waarde wordt getrimd, en wat dan leeg blijft wordt `None`. Zo staat
    /// het in PRD §7: een leeg gemaakt veld betekent "verwijderen" en niet
    /// "een lege waarde opslaan". Door dat hier af te handelen hoeft geen enkel
    /// formulier of handler er nog aan te denken.
    ///
    /// [`read`] doet hetzelfde aan de leeszijde, waardoor een gelezen model
    /// altijd al genormaliseerd is en teruglezen na schrijven exact hetzelfde
    /// oplevert.
    pub fn normalized(&self) -> Tags {
        Tags {
            title: normalize(&self.title),
            artist: normalize(&self.artist),
            album_artist: normalize(&self.album_artist),
            album: normalize(&self.album),
            track: self.track,
            track_total: self.track_total,
            disc: self.disc,
            disc_total: self.disc_total,
            year: normalize(&self.year),
            genre: normalize(&self.genre),
            composer: normalize(&self.composer),
            comment: normalize(&self.comment),
        }
    }
}

/// Trimt een waarde en maakt er `None` van wanneer er niets overblijft.
fn normalize(value: &Option<String>) -> Option<String> {
    text(value.as_deref())
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

/// Alles wat er ruw in één bestand staat.
///
/// De tagsoort hoort erbij: dezelfde titel heet `TIT2` in een ID3v2-frame en
/// `TITLE` in een Vorbis-comment, en zonder te vermelden waar je naar kijkt is
/// zo'n lijst raadselachtig in plaats van diagnostisch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTags {
    pub format: Format,

    /// De tagsoort zoals hij in het bestand staat, bijvoorbeeld `ID3v2`.
    /// `None` wanneer het bestand helemaal geen tag heeft.
    pub kind: Option<String>,

    /// Alle sleutel-waardeparen, op sleutel gesorteerd.
    pub items: Vec<RawTag>,
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
pub fn read_raw_tags(path: &Path) -> Result<RawTags, TagError> {
    let file = open(path)?;
    let format = format_of(&file)?;

    let Some(tag) = primary_tag(&file) else {
        return Ok(RawTags {
            format,
            kind: None,
            items: Vec::new(),
        });
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

    Ok(RawTags {
        format,
        kind: Some(name_of(tag_type)),
        items: raw,
    })
}

/// De naam van een tagsoort zoals de weergave hem toont.
///
/// Sleeve ondersteunt alleen MP3 en FLAC, dus in de praktijk zijn dit de eerste
/// drie. De rest staat er voor het geval een bestand een tag draagt die er niet
/// hoort te zitten — dan is de naam nog steeds informatiever dan niets.
fn name_of(tag_type: TagType) -> String {
    match tag_type {
        TagType::Id3v2 => "ID3v2".to_string(),
        TagType::Id3v1 => "ID3v1".to_string(),
        TagType::VorbisComments => "Vorbis-comments".to_string(),
        TagType::Ape => "APE".to_string(),
        other => format!("{other:?}"),
    }
}

/// Schrijft het genormaliseerde tagmodel naar een bestand.
///
/// Alleen de velden uit het model worden aangeraakt. Alles wat Sleeve niet
/// modelleert blijft staan, want er wordt uitgegaan van de tag die al in het
/// bestand zit en niet van een lege.
///
/// Een veld dat `None` is (of dat na trimmen leeg blijkt) wordt uit het bestand
/// **verwijderd**, niet als lege waarde opgeslagen — zo staat het in PRD §7.
///
/// Verandert er niets, dan wordt het bestand niet aangeraakt. Een bestand
/// herschrijven dat gelijk blijft is een ongevraagde wijziging: de
/// wijzigingsdatum verspringt en Navidrome gaat er opnieuw naar kijken, zonder
/// dat er iets te zien valt.
///
/// Het schrijven zelf loopt via [`crate::atomic::replace`]: naar een tijdelijk
/// bestand, hervalideren door opnieuw in te lezen, en pas dan over het origineel
/// heen.
pub fn write(path: &Path, wanted: &Tags, options: atomic::Options) -> Result<(), WriteError> {
    let wanted = wanted.normalized();

    let current = read(path).map_err(atomic::WriteError::Prepare)?;
    let changes = changed_fields(&current.tags, &wanted);

    if changes.is_empty() {
        tracing::debug!(
            path = %path.display(),
            "geen wijzigingen; het bestand wordt niet aangeraakt"
        );
        return Ok(());
    }

    atomic::replace(
        path,
        options,
        &changes.join(", "),
        |temp| apply(temp, &wanted),
        |temp| {
            let after = read(temp)?;
            if after.tags == wanted {
                Ok(())
            } else {
                Err(TagError::Mismatch)
            }
        },
    )
}

/// Zet of verwijdert de front cover van een bestand (FR-13 en FR-16).
///
/// `cover` is het MIME-type met de bytes, of `None` om de hoes te verwijderen.
/// De bytes komen van [`crate::art::prepare`]: valideren en verkleinen gebeurt
/// daar, hier gaan ze ongewijzigd het bestand in.
///
/// Alleen de afbeelding verandert. De tekstuele tags blijven onaangeraakt, en
/// andere afbeeldingen dan de front cover — een achterkant, een bandfoto —
/// blijven staan: er wordt gericht op `CoverFront` gewisseld en niet met een
/// schone tag begonnen.
///
/// Levert `true` wanneer het bestand werkelijk is aangepast. Zit dezelfde hoes
/// er al in, of is er niets te verwijderen, dan wordt het bestand niet
/// aangeraakt: een herschrijving die niets verandert is een ongevraagde
/// wijziging.
///
/// Het schrijven loopt door [`crate::atomic::replace`], met dezelfde
/// hervalidatie als [`write`]: pas als de hoes teruggelezen is zoals bedoeld,
/// gaat het tijdelijke bestand over het origineel heen.
pub fn write_art(
    path: &Path,
    cover: Option<(&str, &[u8])>,
    options: atomic::Options,
) -> Result<bool, WriteError> {
    let current = read_front_cover(path).map_err(atomic::WriteError::Prepare)?;

    let unchanged = match (&current, cover) {
        (None, None) => true,
        (Some((mime, data)), Some((wanted_mime, wanted_data))) => {
            mime == wanted_mime && data == wanted_data
        }
        _ => false,
    };

    if unchanged {
        tracing::debug!(
            path = %path.display(),
            "de hoes is al zoals bedoeld; het bestand wordt niet aangeraakt"
        );
        return Ok(false);
    }

    let changes = if cover.is_some() {
        "hoes"
    } else {
        "hoes verwijderd"
    };

    atomic::replace(
        path,
        options,
        changes,
        |temp| apply_art(temp, cover),
        |temp| {
            let after = read_front_cover(temp)?;

            let ok = match (&after, cover) {
                (None, None) => true,
                (Some((mime, data)), Some((wanted_mime, wanted_data))) => {
                    mime == wanted_mime && data == wanted_data
                }
                _ => false,
            };

            if ok { Ok(()) } else { Err(TagError::Mismatch) }
        },
    )?;

    Ok(true)
}

/// Wisselt de front cover in de tag van het tijdelijke bestand.
fn apply_art(path: &Path, cover: Option<(&str, &[u8])>) -> Result<(), TagError> {
    let file = open(path)?;
    let target = tag_type_for(format_of(&file)?);

    let mut tag = match file.tag(target) {
        Some(existing) => existing.clone(),
        None => match primary_tag(&file) {
            Some(other) => {
                let mut converted = other.clone();
                converted.re_map(target);
                converted
            }
            None => Tag::new(target),
        },
    };

    // Eerst weg wat er stond. Zonder dit zou een tweede hoes naast de eerste
    // belanden, en welke van de twee een speler dan kiest, is niet te zeggen.
    tag.remove_picture_type(PictureType::CoverFront);

    if let Some((mime, data)) = cover {
        // `unchecked`: de afbeelding is al door `art::prepare` gehaald, en die
        // heeft strengere eisen dan lofty — alleen JPEG en PNG, en werkelijk
        // gedecodeerd.
        tag.push_picture(
            Picture::unchecked(data.to_vec())
                .pic_type(PictureType::CoverFront)
                .mime_type(MimeType::from_str(mime))
                .build(),
        );
    }

    tag.save_to_path(path, WriteOptions::new())
        .map_err(|error| {
            tracing::error!(%error, "de hoes kon niet weggeschreven worden");
            TagError::Unwritable
        })?;

    remove_stale_tags(path, target)
}

/// Zet het model in de tag van het tijdelijke bestand en schrijft die weg.
///
/// Er wordt begonnen bij de tag die er al staat, zodat niet-gemodelleerde
/// velden blijven bestaan. Heeft het bestand nog geen tag van het juiste soort
/// — een MP3 met alleen ID3v1 bijvoorbeeld — dan wordt de bestaande omgezet.
/// Daarbij gaat niets verloren: ID3v1 draagt uitsluitend velden die het model
/// kent, en die zijn al ingelezen.
fn apply(path: &Path, wanted: &Tags) -> Result<(), TagError> {
    let file = open(path)?;
    let target = tag_type_for(format_of(&file)?);

    let mut tag = match file.tag(target) {
        Some(existing) => existing.clone(),
        None => match primary_tag(&file) {
            Some(other) => {
                let mut converted = other.clone();
                converted.re_map(target);
                converted
            }
            None => Tag::new(target),
        },
    };

    set_tags(&mut tag, wanted);

    // lofty schrijft ID3v2.4 met UTF-8; `use_id3v23` staat standaard uit en
    // wordt hier bewust niet gezet.
    tag.save_to_path(path, WriteOptions::new())
        .map_err(|error| {
            tracing::error!(%error, "tags konden niet weggeschreven worden");
            TagError::Unwritable
        })?;

    remove_stale_tags(path, target)
}

/// Verwijdert tagsoorten die naast de geschreven tag niet horen te bestaan.
///
/// Voor MP3 is dat ID3v1: die kan maar dertig tekens per veld en zou na een
/// wijziging iets anders zeggen dan ID3v2. Het PRD verbiedt zo'n
/// tegenstrijdigheid; verwijderen maakt hem onmogelijk en is veiliger dan
/// synchroniseren, want dan is er niets meer om uit de pas te lopen.
///
/// Twee omwegen om lofty 0.25.1 heen zijn hier nodig:
///
/// - `WriteOptions::remove_others` zou dit moeten doen, maar die vlag wordt
///   nergens uitgelezen: hij bestaat wel en doet niets.
/// - `TagType::remove_from_path` opent het bestand alleen-lezen en probeert er
///   vervolgens in te schrijven, wat altijd mislukt. Daarom wordt het bestand
///   hier zelf lees-schrijf geopend en `remove_from` gebruikt.
fn remove_stale_tags(path: &Path, kept: TagType) -> Result<(), TagError> {
    if kept != TagType::Id3v2 {
        return Ok(());
    }

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            tracing::error!(%error, "bestand kon niet geopend worden om ID3v1 te verwijderen");
            TagError::Unwritable
        })?;

    TagType::Id3v1
        .remove_from(&mut file, WriteOptions::new())
        .map_err(|error| {
            tracing::error!(%error, "de ID3v1-tag kon niet verwijderd worden");
            TagError::Unwritable
        })
}

/// De tagsoort waarin Sleeve voor dit formaat schrijft.
fn tag_type_for(format: Format) -> TagType {
    match format {
        Format::Mp3 => TagType::Id3v2,
        Format::Flac => TagType::VorbisComments,
    }
}

/// Zet of verwijdert elk gemodelleerd veld; de rest van de tag blijft staan.
fn set_tags(tag: &mut Tag, wanted: &Tags) {
    match &wanted.title {
        Some(value) => tag.set_title(value.clone()),
        None => tag.remove_title(),
    }
    match &wanted.artist {
        Some(value) => tag.set_artist(value.clone()),
        None => tag.remove_artist(),
    }
    match &wanted.album {
        Some(value) => tag.set_album(value.clone()),
        None => tag.remove_album(),
    }
    match &wanted.genre {
        Some(value) => tag.set_genre(value.clone()),
        None => tag.remove_genre(),
    }

    set_text(tag, ItemKey::AlbumArtist, wanted.album_artist.as_deref());
    set_text(tag, ItemKey::Composer, wanted.composer.as_deref());

    // Het jaar gaat naar `RecordingDate` (TDRC in ID3v2.4, DATE in Vorbis).
    // Het losse `Year`-veld wordt opgeruimd: twee plekken met een verschillend
    // jaartal is precies de verwarring die deze app moet wegnemen.
    set_text(tag, ItemKey::RecordingDate, wanted.year.as_deref());
    tag.remove_key(ItemKey::Year);

    // Commentaar staat in Vorbis soms als DESCRIPTION (zo schrijft ffmpeg het)
    // en soms als COMMENT (zo schrijft Picard het). Sleeve schrijft COMMENT en
    // ruimt DESCRIPTION op, zodat er niet twee tegenstrijdige waarden blijven.
    match &wanted.comment {
        Some(value) => tag.set_comment(value.clone()),
        None => tag.remove_comment(),
    }
    tag.remove_key(ItemKey::Description);

    // Nummer en totaal: lofty kent het formaatverschil en schrijft ze voor
    // ID3v2 samen als `TRCK` (`n/total`), voor Vorbis als losse velden.
    match wanted.track {
        Some(value) => tag.set_track(value),
        None => tag.remove_track(),
    }
    match wanted.track_total {
        Some(value) => tag.set_track_total(value),
        None => tag.remove_track_total(),
    }
    match wanted.disc {
        Some(value) => tag.set_disk(value),
        None => tag.remove_disk(),
    }
    match wanted.disc_total {
        Some(value) => tag.set_disk_total(value),
        None => tag.remove_disk_total(),
    }
}

/// Zet een tekstveld, of verwijdert het wanneer er geen waarde is.
fn set_text(tag: &mut Tag, key: ItemKey, value: Option<&str>) {
    match value {
        Some(value) => {
            tag.insert_text(key, value.to_string());
        }
        None => {
            tag.remove_key(key);
        }
    }
}

/// De namen van de velden die veranderen, voor in het logboek.
fn changed_fields(current: &Tags, wanted: &Tags) -> Vec<&'static str> {
    let mut changed = Vec::new();

    let mut note = |name: &'static str, differs: bool| {
        if differs {
            changed.push(name);
        }
    };

    note("titel", current.title != wanted.title);
    note("artiest", current.artist != wanted.artist);
    note("albumartiest", current.album_artist != wanted.album_artist);
    note("album", current.album != wanted.album);
    note("tracknummer", current.track != wanted.track);
    note("tracktotaal", current.track_total != wanted.track_total);
    note("discnummer", current.disc != wanted.disc);
    note("disctotaal", current.disc_total != wanted.disc_total);
    note("jaar", current.year != wanted.year);
    note("genre", current.genre != wanted.genre);
    note("componist", current.composer != wanted.composer);
    note("commentaar", current.comment != wanted.comment);

    changed
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

/// Beschrijft de hoes zonder de pixels uit te pakken.
///
/// De afmetingen komen uit [`crate::art`], want dat is de enige module die aan
/// afbeeldingen raakt; hier gaan alleen de ruwe bytes uit het bestand naartoe.
/// Er wordt daar alleen de header gelezen: de maplijst toont per bestand of er
/// art is en hoe groot, en mag daarvoor geen dertig afbeeldingen uitpakken.
fn describe_art(picture: &Picture) -> Option<ArtInfo> {
    let data = picture.data();
    let (width, height) = crate::art::dimensions(data).ok()?;

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
        let keys: Vec<&str> = mp3.items.iter().map(|tag| tag.key.as_str()).collect();

        // ID3v2 gebruikt frame-ID's, geen genormaliseerde veldnamen.
        assert!(keys.contains(&"TIT2"), "sleutels waren: {keys:?}");
        assert!(keys.contains(&"TPE1"), "sleutels waren: {keys:?}");

        let flac = read_raw_tags(&testfixtures::fixture_path(testfixtures::FLAC_WITH_TAGS))
            .expect("lezen moet lukken");
        let keys: Vec<&str> = flac.items.iter().map(|tag| tag.key.as_str()).collect();

        // Vorbis-comments gebruiken hun eigen namen.
        assert!(keys.contains(&"TITLE"), "sleutels waren: {keys:?}");
        assert!(keys.contains(&"ARTIST"), "sleutels waren: {keys:?}");
    }

    /// Kopieert een fixture naar een tempdir en geeft het pad naar de kopie.
    ///
    /// Schrijftests werken nooit tegen de fixture in de repo zelf.
    fn writable_copy(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        testfixtures::copy_to_tempdir(name)
    }

    /// Het model dat de schrijftests wegschrijven.
    fn wanted_tags() -> Tags {
        Tags {
            title: Some("Nieuwe titel".to_string()),
            artist: Some("Nieuwe artiest".to_string()),
            album_artist: Some("Nieuwe albumartiest".to_string()),
            album: Some("Nieuw album".to_string()),
            track: Some(7),
            track_total: Some(9),
            disc: Some(2),
            disc_total: Some(3),
            year: Some("2001".to_string()),
            genre: Some("Shoegaze".to_string()),
            composer: Some("Nieuwe componist".to_string()),
            comment: Some("Een nieuw commentaar".to_string()),
        }
    }

    /// De ruwe sleutels van een bestand, voor controles op frameniveau.
    fn raw_keys(path: &std::path::Path) -> Vec<String> {
        read_raw_tags(path)
            .expect("ruwe tags moeten leesbaar zijn")
            .items
            .into_iter()
            .map(|item| item.key)
            .collect()
    }

    /// De ruwe waarde van één sleutel; paniekt als de sleutel ontbreekt.
    fn raw_value(path: &std::path::Path, key: &str) -> String {
        read_raw_tags(path)
            .expect("ruwe tags moeten leesbaar zijn")
            .items
            .into_iter()
            .find(|item| item.key == key)
            .unwrap_or_else(|| panic!("sleutel '{key}' ontbreekt in {}", path.display()))
            .value
    }

    #[test]
    fn writes_the_whole_model_to_mp3_and_flac() {
        for name in [testfixtures::MP3_WITH_TAGS, testfixtures::FLAC_WITH_TAGS] {
            let (_tempdir, path) = writable_copy(name);

            write(&path, &wanted_tags(), atomic::Options::default())
                .unwrap_or_else(|error| panic!("{name} moet schrijfbaar zijn: {error}"));

            let after = read(&path).expect("teruglezen moet lukken");
            assert_eq!(after.tags, wanted_tags(), "fixture: {name}");
        }
    }

    #[test]
    fn writing_works_on_a_file_without_any_tags() {
        for name in [
            testfixtures::MP3_WITHOUT_TAGS,
            testfixtures::FLAC_WITHOUT_TAGS,
        ] {
            let (_tempdir, path) = writable_copy(name);

            write(&path, &wanted_tags(), atomic::Options::default())
                .unwrap_or_else(|error| panic!("{name} moet schrijfbaar zijn: {error}"));

            assert_eq!(
                read(&path).expect("teruglezen").tags,
                wanted_tags(),
                "fixture: {name}"
            );
        }
    }

    #[test]
    fn an_emptied_field_is_removed_from_the_file() {
        // Niet alleen uit het model: de sleutel hoort werkelijk uit het bestand
        // te verdwijnen in plaats van als lege waarde achter te blijven.
        let (_tempdir, path) = writable_copy(testfixtures::MP3_WITH_TAGS);
        assert!(raw_keys(&path).contains(&"TCOM".to_string()));

        let mut wanted = read(&path).expect("lezen").tags;
        wanted.composer = None;
        // Een waarde die alleen uit spaties bestaat telt ook als leeg.
        wanted.genre = Some("   ".to_string());

        write(&path, &wanted, atomic::Options::default()).expect("schrijven moet lukken");

        let keys = raw_keys(&path);
        assert!(!keys.contains(&"TCOM".to_string()), "sleutels: {keys:?}");
        assert!(!keys.contains(&"TCON".to_string()), "sleutels: {keys:?}");

        let after = read(&path).expect("teruglezen").tags;
        assert_eq!(after.composer, None);
        assert_eq!(after.genre, None);
    }

    /// Zet twee velden in een bestand die Sleeve niet modelleert.
    ///
    /// De ingecheckte fixtures dragen uitsluitend velden die het model kent, dus
    /// zonder deze stap valt er niets te bewijzen. Uitgever en ISRC zijn geen
    /// bedachte gevallen: Picard schrijft ze standaard, en een bibliotheek die
    /// daar doorheen is gegaan zit er vol mee.
    fn add_unmodelled_fields(path: &std::path::Path) {
        let file = open(path).expect("bestand moet leesbaar zijn");
        let target = tag_type_for(format_of(&file).expect("formaat moet bekend zijn"));

        let mut tag = file
            .tag(target)
            .cloned()
            .unwrap_or_else(|| Tag::new(target));

        tag.insert_text(ItemKey::Publisher, "Een platenlabel".to_string());
        tag.insert_text(ItemKey::Isrc, "NLA123456789".to_string());

        tag.save_to_path(path, WriteOptions::new())
            .expect("de voorbereiding moet schrijfbaar zijn");
    }

    #[test]
    fn fields_outside_the_model_survive_a_write() {
        // Per formaat de sleutels waaronder uitgever en ISRC terechtkomen.
        for (name, keys) in [
            (testfixtures::MP3_WITH_TAGS, ["TPUB", "TSRC"]),
            (testfixtures::FLAC_WITH_TAGS, ["PUBLISHER", "ISRC"]),
        ] {
            let (_tempdir, path) = writable_copy(name);
            add_unmodelled_fields(&path);

            let before = raw_keys(&path);
            for key in keys {
                assert!(
                    before.contains(&key.to_string()),
                    "{name}: de voorbereiding zette '{key}' niet: {before:?}"
                );
            }

            write(&path, &wanted_tags(), atomic::Options::default())
                .unwrap_or_else(|error| panic!("{name}: {error}"));

            let after = raw_keys(&path);
            for key in keys {
                assert!(
                    after.contains(&key.to_string()),
                    "{name}: '{key}' is verdwenen na het schrijven: {after:?}"
                );
            }
            assert_eq!(
                raw_value(&path, keys[1]),
                "NLA123456789",
                "{name}: de waarde is veranderd"
            );
        }
    }

    #[test]
    fn embedded_art_survives_a_write() {
        // Een tagwijziging mag de hoes niet slopen.
        for name in [testfixtures::MP3_WITH_ART, testfixtures::FLAC_WITH_ART] {
            let (_tempdir, path) = writable_copy(name);
            let before = read_front_cover(&path)
                .expect("lezen")
                .expect("de fixture heeft een hoes");

            write(&path, &wanted_tags(), atomic::Options::default())
                .unwrap_or_else(|error| panic!("{name}: {error}"));

            let after = read_front_cover(&path)
                .expect("lezen")
                .unwrap_or_else(|| panic!("{name}: de hoes is verdwenen"));

            assert_eq!(after.0, before.0, "{name}: het MIME-type is veranderd");
            assert_eq!(after.1, before.1, "{name}: de hoes is veranderd");
        }
    }

    /// Leest een syncsafe geheel getal van vier bytes (ID3v2.4).
    fn syncsafe(bytes: &[u8]) -> usize {
        bytes
            .iter()
            .fold(0usize, |total, byte| (total << 7) | (*byte as usize & 0x7F))
    }

    /// De lengte van het ID3v2-blok aan het begin van een bestand, met header.
    fn id3v2_length(bytes: &[u8]) -> Option<usize> {
        if bytes.len() < 10 || &bytes[..3] != b"ID3" {
            return None;
        }
        Some(10 + syncsafe(&bytes[6..10]))
    }

    /// De tekstinhoud van één ID3v2.4-frame, rechtstreeks uit de bytes gelezen.
    ///
    /// Nodig omdat `read_raw_tags` de gesplitste kijk van de tagbibliotheek
    /// geeft: een `TRCK` met `7/9` komt daar als twee regels langs. Om te
    /// bewijzen dat er één frame met `7/9` in het bestand staat, moet je naar de
    /// bytes zelf kijken.
    fn id3v2_frame(bytes: &[u8], wanted: &str) -> Option<String> {
        let end = id3v2_length(bytes)?;
        let mut at = 10;

        while at + 10 <= end {
            let id = &bytes[at..at + 4];
            if id == [0, 0, 0, 0] {
                // Vanaf hier is het opvulling.
                break;
            }

            // ID3v2.4 gebruikt ook voor framelengtes syncsafe getallen.
            let size = syncsafe(&bytes[at + 4..at + 8]);
            let content = &bytes[at + 10..(at + 10 + size).min(bytes.len())];

            if id == wanted.as_bytes() {
                // De eerste byte van een tekstframe is de codering.
                let text = String::from_utf8_lossy(&content[1..]);
                return Some(text.trim_end_matches('\0').to_string());
            }

            at += 10 + size;
        }

        None
    }

    /// De audio-inhoud van een bestand: alles behalve de tagblokken.
    ///
    /// Voor MP3 vervallen het ID3v2-blok vooraan en een eventuele ID3v1-staart
    /// van 128 bytes; voor FLAC alle metadatablokken tot en met het laatste.
    /// Wat overblijft zijn de audioframes, en die horen een tagwijziging
    /// byte-voor-byte te overleven.
    fn audio_bytes(path: &std::path::Path) -> Vec<u8> {
        let bytes = std::fs::read(path).expect("bestand moet leesbaar zijn");

        if bytes.starts_with(b"fLaC") {
            let mut at = 4;
            loop {
                let header = &bytes[at..at + 4];
                let last = header[0] & 0x80 != 0;
                let length =
                    ((header[1] as usize) << 16) | ((header[2] as usize) << 8) | header[3] as usize;
                at += 4 + length;
                if last {
                    break;
                }
            }
            return bytes[at..].to_vec();
        }

        let start = id3v2_length(&bytes).unwrap_or(0);
        let mut end = bytes.len();
        if end >= 128 && &bytes[end - 128..end - 125] == b"TAG" {
            end -= 128;
        }

        bytes[start..end].to_vec()
    }

    #[test]
    fn an_mp3_is_id3v2_4_after_writing() {
        // Ook wanneer het bestand daarvoor iets anders had: de fixture met
        // alleen een ID3v1-tag heeft helemaal geen ID3v2 om mee te beginnen.
        for name in [testfixtures::MP3_WITH_TAGS, testfixtures::MP3_ID3V1_ONLY] {
            let (_tempdir, path) = writable_copy(name);

            write(&path, &wanted_tags(), atomic::Options::default())
                .unwrap_or_else(|error| panic!("{name}: {error}"));

            let bytes = std::fs::read(&path).expect("lezen");
            assert_eq!(&bytes[..3], b"ID3", "{name}: geen ID3v2-blok");
            assert_eq!(
                bytes[3], 4,
                "{name}: geen ID3v2.4 maar versie 2.{}",
                bytes[3]
            );

            // UTF-8 hoort erbij: codering 3 is UTF-8, en de waarde moet er
            // ongeschonden uit komen.
            let title = id3v2_frame(&bytes, "TIT2").expect("TIT2 moet er staan");
            assert_eq!(title, "Nieuwe titel", "{name}");
        }
    }

    #[test]
    fn an_id3v1_tag_is_gone_after_writing() {
        // De fixture met een ID3v1 die afwijkt van ID3v2 is het lastige geval:
        // laten staan zou betekenen dat een speler die ID3v1 leest iets anders
        // toont dan Sleeve. Verwijderen maakt dat onmogelijk.
        for name in [
            testfixtures::MP3_ID3V1_INCONSISTENT,
            testfixtures::MP3_ID3V1_ONLY,
        ] {
            let (_tempdir, path) = writable_copy(name);

            let before = std::fs::read(&path).expect("lezen");
            assert_eq!(
                &before[before.len() - 128..before.len() - 125],
                b"TAG",
                "{name}: de fixture heeft geen ID3v1-tag om op te testen"
            );

            write(&path, &wanted_tags(), atomic::Options::default())
                .unwrap_or_else(|error| panic!("{name}: {error}"));

            let after = std::fs::read(&path).expect("lezen");
            assert_ne!(
                &after[after.len() - 128..after.len() - 125],
                b"TAG",
                "{name}: er staat nog een ID3v1-tag in het bestand"
            );
        }
    }

    #[test]
    fn combined_fields_use_the_notation_of_their_format() {
        // ID3v2 stopt nummer en totaal in één frame met een schuine streep.
        let (_tempdir, mp3) = writable_copy(testfixtures::MP3_WITH_TAGS);
        write(&mp3, &wanted_tags(), atomic::Options::default()).expect("schrijven");

        let bytes = std::fs::read(&mp3).expect("lezen");
        assert_eq!(id3v2_frame(&bytes, "TRCK").as_deref(), Some("7/9"));
        assert_eq!(id3v2_frame(&bytes, "TPOS").as_deref(), Some("2/3"));

        // Vorbis-comments houden ze uit elkaar.
        let (_tempdir, flac) = writable_copy(testfixtures::FLAC_WITH_TAGS);
        write(&flac, &wanted_tags(), atomic::Options::default()).expect("schrijven");

        assert_eq!(raw_value(&flac, "TRACKNUMBER"), "7");
        assert_eq!(raw_value(&flac, "TRACKTOTAL"), "9");
        assert_eq!(raw_value(&flac, "DISCNUMBER"), "2");
        assert_eq!(raw_value(&flac, "DISCTOTAL"), "3");
    }

    /// De losse coverafbeelding uit de fixtures.
    fn cover_bytes(name: &str) -> Vec<u8> {
        std::fs::read(testfixtures::fixture_path(name)).expect("fixture moet leesbaar zijn")
    }

    #[test]
    fn a_cover_can_be_embedded_in_both_formats() {
        for name in [testfixtures::MP3_WITH_TAGS, testfixtures::FLAC_WITH_TAGS] {
            let (_tempdir, path) = writable_copy(name);
            let cover = cover_bytes(testfixtures::COVER_PNG);

            assert!(read(&path).expect("lezen").art.is_none(), "{name}");

            let written = write_art(
                &path,
                Some(("image/png", &cover)),
                atomic::Options::default(),
            )
            .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(written, "{name}: er hoorde iets geschreven te worden");

            let (mime, data) = read_front_cover(&path)
                .expect("lezen")
                .unwrap_or_else(|| panic!("{name}: er hoort nu een hoes in te zitten"));

            assert_eq!(mime, "image/png", "{name}");
            assert_eq!(data, cover, "{name}: de bytes horen ongewijzigd te zijn");
        }
    }

    #[test]
    fn embedding_a_cover_leaves_the_other_tags_alone() {
        // De hoes vervangen is geen reden om aan de tekst te komen.
        let (_tempdir, path) = writable_copy(testfixtures::MP3_WITH_TAGS);
        let before = read(&path).expect("lezen").tags;

        write_art(
            &path,
            Some(("image/jpeg", &cover_bytes(testfixtures::COVER_JPEG))),
            atomic::Options::default(),
        )
        .expect("schrijven moet lukken");

        assert_eq!(read(&path).expect("lezen").tags, before);
    }

    #[test]
    fn a_cover_replaces_the_one_that_was_there() {
        // Niet ernaast: welke van de twee een speler dan kiest, is niet te
        // zeggen.
        let (_tempdir, path) = writable_copy(testfixtures::MP3_WITH_ART);
        let other = cover_bytes(testfixtures::OTHER_COVER_PNG);

        write_art(
            &path,
            Some(("image/png", &other)),
            atomic::Options::default(),
        )
        .expect("schrijven moet lukken");

        let art = read(&path).expect("lezen").art.expect("er is een hoes");
        assert_eq!(art.mime, "image/png");
        assert_eq!(art.bytes, other.len());
        assert_eq!((art.width, art.height), (500, 500));
    }

    #[test]
    fn a_cover_can_be_removed() {
        for name in [testfixtures::MP3_WITH_ART, testfixtures::FLAC_WITH_ART] {
            let (_tempdir, path) = writable_copy(name);
            let tags = read(&path).expect("lezen").tags;

            let written = write_art(&path, None, atomic::Options::default())
                .unwrap_or_else(|error| panic!("{name}: {error}"));

            assert!(written, "{name}: er hoorde iets geschreven te worden");
            assert!(read(&path).expect("lezen").art.is_none(), "{name}");
            assert_eq!(
                read(&path).expect("lezen").tags,
                tags,
                "{name}: de tekstuele tags horen ongemoeid te blijven"
            );
        }
    }

    #[test]
    fn writing_the_same_cover_leaves_the_file_untouched() {
        let (_tempdir, path) = writable_copy(testfixtures::MP3_WITH_ART);
        let before = std::fs::read(&path).expect("lezen");
        let (mime, data) = read_front_cover(&path)
            .expect("lezen")
            .expect("de fixture heeft een hoes");

        let written = write_art(&path, Some((&mime, &data)), atomic::Options::default())
            .expect("schrijven moet lukken");

        assert!(!written, "er viel niets te wijzigen");
        assert_eq!(std::fs::read(&path).expect("lezen"), before);
    }

    #[test]
    fn removing_a_cover_that_is_not_there_leaves_the_file_untouched() {
        let (_tempdir, path) = writable_copy(testfixtures::MP3_WITH_TAGS);
        let before = std::fs::read(&path).expect("lezen");

        let written =
            write_art(&path, None, atomic::Options::default()).expect("dit hoort te lukken");

        assert!(!written);
        assert_eq!(std::fs::read(&path).expect("lezen"), before);
    }

    #[test]
    fn the_audio_survives_a_cover_change_bit_for_bit() {
        for name in [testfixtures::MP3_WITH_ART, testfixtures::FLAC_WITH_ART] {
            let (_tempdir, path) = writable_copy(name);
            let before = audio_bytes(&path);

            write_art(
                &path,
                Some(("image/png", &cover_bytes(testfixtures::OTHER_COVER_PNG))),
                atomic::Options::default(),
            )
            .unwrap_or_else(|error| panic!("{name}: {error}"));

            assert_eq!(
                audio_bytes(&path),
                before,
                "{name}: de audio is veranderd door een hoeswijziging"
            );
        }
    }

    #[test]
    fn the_audio_survives_a_tag_change_bit_for_bit() {
        for name in [
            testfixtures::MP3_WITH_TAGS,
            testfixtures::FLAC_WITH_TAGS,
            testfixtures::MP3_WITH_ART,
            testfixtures::FLAC_WITH_ART,
        ] {
            let (_tempdir, path) = writable_copy(name);
            let before = audio_bytes(&path);
            assert!(!before.is_empty(), "{name}: geen audio gevonden");

            write(&path, &wanted_tags(), atomic::Options::default())
                .unwrap_or_else(|error| panic!("{name}: {error}"));

            assert_eq!(
                audio_bytes(&path),
                before,
                "{name}: de audio is veranderd door een tagwijziging"
            );
        }
    }

    #[test]
    fn writing_the_same_tags_leaves_the_file_untouched() {
        // Een bestand herschrijven dat gelijk blijft is een ongevraagde
        // wijziging: de wijzigingsdatum verspringt en Navidrome gaat er
        // opnieuw naar kijken zonder dat er iets te zien valt.
        let (_tempdir, path) = writable_copy(testfixtures::MP3_WITH_TAGS);
        let before = std::fs::read(&path).expect("lezen");
        let unchanged = read(&path).expect("lezen").tags;

        write(&path, &unchanged, atomic::Options::default()).expect("schrijven moet lukken");

        assert_eq!(
            std::fs::read(&path).expect("lezen"),
            before,
            "het bestand is aangeraakt terwijl er niets veranderde"
        );
    }

    #[test]
    fn a_failed_write_leaves_the_audio_file_intact() {
        use std::os::unix::fs::PermissionsExt;

        let (tempdir, path) = writable_copy(testfixtures::MP3_WITH_TAGS);
        let before = std::fs::read(&path).expect("lezen");

        // Een map waarin niets bijgemaakt mag worden: het tijdelijke bestand
        // kan niet eens ontstaan.
        std::fs::set_permissions(tempdir.path(), std::fs::Permissions::from_mode(0o555))
            .expect("rechten moeten in te stellen zijn");

        let result = write(&path, &wanted_tags(), atomic::Options::default());

        // Rechten terugzetten, anders kan de tempdir niet opgeruimd worden.
        std::fs::set_permissions(tempdir.path(), std::fs::Permissions::from_mode(0o755))
            .expect("rechten moeten terug te zetten zijn");

        assert!(result.is_err(), "de schrijfactie had moeten mislukken");
        assert_eq!(
            std::fs::read(&path).expect("lezen"),
            before,
            "het bestand is aangetast door een mislukte schrijfactie"
        );
    }

    #[test]
    fn an_independent_tool_reads_back_what_was_written() {
        // De onafhankelijke controle uit het acceptatiecriterium. ffprobe hoort
        // niet bij de toolchain, dus zonder ffprobe slaat deze test zichzelf
        // over: de kwaliteitspoort mag niet van een systeemtool afhangen.
        let (_tempdir, path) = writable_copy(testfixtures::MP3_WITH_TAGS);
        write(&path, &wanted_tags(), atomic::Options::default()).expect("schrijven");

        let output = match std::process::Command::new("ffprobe")
            .args([
                "-v",
                "quiet",
                "-show_entries",
                "format_tags",
                "-of",
                "default=noprint_wrappers=1",
            ])
            .arg(&path)
            .output()
        {
            Ok(output) => output,
            Err(_) => {
                eprintln!("ffprobe ontbreekt; de onafhankelijke controle is overgeslagen");
                return;
            }
        };

        let tags = String::from_utf8_lossy(&output.stdout);
        for expected in [
            "TAG:title=Nieuwe titel",
            "TAG:artist=Nieuwe artiest",
            "TAG:album=Nieuw album",
            "TAG:track=7/9",
            "TAG:disc=2/3",
        ] {
            assert!(
                tags.contains(expected),
                "ffprobe zag '{expected}' niet. Wat het wel zag:\n{tags}"
            );
        }
    }

    #[test]
    fn the_lofty_workarounds_are_still_needed() {
        // `tags::write` gaat om twee gebreken in lofty 0.25.1 heen. Deze test
        // legt vast dát ze er zijn: gaat hij bij een nieuwere versie stuk, dan
        // is dat het sein om de omweg weg te halen in plaats van hem mee te
        // slepen omdat niemand meer weet waarom hij er staat.
        let (_tempdir, path) = writable_copy(testfixtures::MP3_ID3V1_ONLY);

        // 1. `remove_from_path` opent het bestand alleen-lezen en probeert er
        //    dan in te schrijven.
        assert!(
            TagType::Id3v1
                .remove_from_path(&path, WriteOptions::new())
                .is_err(),
            "remove_from_path werkt weer; de omweg in remove_stale_tags kan weg"
        );

        // 2. `remove_others` laat de ID3v1-tag gewoon staan.
        let (_tempdir, path) = writable_copy(testfixtures::MP3_ID3V1_INCONSISTENT);
        let tag = open(&path)
            .expect("lezen")
            .tag(TagType::Id3v2)
            .cloned()
            .expect("de fixture heeft een ID3v2-tag");

        tag.save_to_path(&path, WriteOptions::new().remove_others(true))
            .expect("schrijven moet lukken");

        let bytes = std::fs::read(&path).expect("lezen");
        assert_eq!(
            &bytes[bytes.len() - 128..bytes.len() - 125],
            b"TAG",
            "remove_others werkt weer; de handmatige opruiming kan weg"
        );
    }

    #[test]
    fn raw_tags_name_the_kind_of_tag() {
        // Dezelfde titel heet in MP3 `TIT2` en in FLAC `TITLE`; de weergave
        // hoort te zeggen waar je naar kijkt.
        let mp3 = read_raw_tags(&testfixtures::fixture_path(testfixtures::MP3_WITH_TAGS))
            .expect("lezen moet lukken");
        assert_eq!(mp3.format, Format::Mp3);
        assert_eq!(mp3.kind.as_deref(), Some("ID3v2"));

        let flac = read_raw_tags(&testfixtures::fixture_path(testfixtures::FLAC_WITH_TAGS))
            .expect("lezen moet lukken");
        assert_eq!(flac.format, Format::Flac);
        assert_eq!(flac.kind.as_deref(), Some("Vorbis-comments"));

        // Een MP3 zonder ID3v2 valt terug op de tag die er wél is.
        let old = read_raw_tags(&testfixtures::fixture_path(testfixtures::MP3_ID3V1_ONLY))
            .expect("lezen moet lukken");
        assert_eq!(old.kind.as_deref(), Some("ID3v1"));
    }

    #[test]
    fn a_file_without_tags_has_nothing_to_show() {
        // Een MP3 zonder tags heeft werkelijk geen tagblok.
        let mp3 = read_raw_tags(&testfixtures::fixture_path(testfixtures::MP3_WITHOUT_TAGS))
            .expect("lezen moet lukken");
        assert_eq!(mp3.kind, None);
        assert!(mp3.items.is_empty(), "{:?}", mp3.items);

        // De "ongetagde" FLAC draagt wél een Vorbis-comment-blok, met daarin
        // `ENCODER=ffmpeg`: dat schrijft ffmpeg ook met `-map_metadata -1`.
        // Het genormaliseerde model laat er niets van zien, want ENCODER hoort
        // niet bij de gemodelleerde velden — en juist dat verschil is waar
        // deze weergave voor bestaat.
        let flac = read_raw_tags(&testfixtures::fixture_path(testfixtures::FLAC_WITHOUT_TAGS))
            .expect("lezen moet lukken");
        assert_eq!(flac.kind.as_deref(), Some("Vorbis-comments"));
        assert_eq!(
            flac.items,
            vec![RawTag {
                key: "ENCODER".to_string(),
                value: "ffmpeg".to_string(),
            }]
        );

        let model = read(&testfixtures::fixture_path(testfixtures::FLAC_WITHOUT_TAGS))
            .expect("lezen moet lukken");
        assert!(
            model.tags.is_empty(),
            "het gemodelleerde beeld hoort leeg te blijven: {:?}",
            model.tags
        );
    }

    #[test]
    fn raw_tags_summarise_binary_values() {
        let raw = read_raw_tags(&testfixtures::fixture_path(testfixtures::MP3_WITH_ART))
            .expect("lezen moet lukken");

        let art = raw
            .items
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
