//! Het weergavemodel van één map: wat de mapbrowser op het scherm zet.
//!
//! Deze module brengt de padlaag ([`crate::fs`]) en het tagmodel
//! ([`crate::tags`]) bij elkaar en levert een structuur die de templates
//! rechtstreeks kunnen renderen. Ze opent zelf geen bestanden en bouwt zelf geen
//! paden op: tag-I/O gaat uitsluitend via [`crate::tags`], zoals de
//! architectuurtest afdwingt.
//!
//! Alles wat naar de browser gaat is relatief aan `MUSIC_ROOT`. Het absolute
//! pad van de NAS blijft binnen [`crate::fs`].

use std::path::Path;
use std::time::Duration;

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use crate::checks::{self, FolderIssue, TrackIssue};
use crate::fs::{DirEntry, Library, PathError};
use crate::tags::{self, Tags};

/// Naam van de bibliotheekwortel in het broodkruimelpad.
///
/// De gebruiker hoeft niet te weten dat de map in de container `/music` heet.
const ROOT_NAME: &str = "Bibliotheek";

/// Wat er staat waar een tag ontbreekt.
const MISSING: &str = "—";

/// Wat er op de kaart van een map staat waarin niets te bewerken valt.
const NOTHING_EDITABLE: &str = "Geen bewerkbare bestanden";

/// Het opschrift van de groep bestanden zonder discnummer.
const NO_DISC: &str = "Zonder discnummer";

/// Waarde van de `size`-parameter waarmee om de verkleinde hoes wordt gevraagd.
///
/// Staat hier omdat de URL's hier worden opgebouwd; het endpoint in
/// [`crate::web`] leest dezelfde constante, zodat de twee niet uit elkaar
/// kunnen lopen.
pub const THUMBNAIL_SIZE_PARAM: &str = "thumb";

/// Tekens die in een padsegment van een URL gecodeerd moeten worden.
///
/// `/` blijft er bewust buiten: het scheidt de segmenten en hoort niet gecodeerd
/// te worden. `?` en `#` juist wel, anders begint de browser halverwege een
/// mapnaam aan een query of een fragment.
const PATH_ESCAPES: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'%');

/// Tekens die in een waarde ván een queryparameter gecodeerd moeten worden.
///
/// Ruimer dan [`PATH_ESCAPES`]: `&` en `=` scheiden hier de parameters, dus een
/// zoekterm waar zo'n teken in staat zou de rest van de URL kapotmaken.
const QUERY_ESCAPES: &AsciiSet = &PATH_ESCAPES.add(b'&').add(b'=').add(b'+').add(b'/');

/// Naam van de queryparameter waarmee het aandachtsfilter aan staat.
///
/// De stand hoort in de URL: een gefilterde lijst moet te delen en te
/// bookmarken zijn, en zonder JavaScript is een gewone link de enige manier om
/// hem aan of uit te zetten.
pub const ATTENTION_PARAM: &str = "aandacht";

/// De waarde waarmee dat filter aan staat.
pub const ATTENTION_ON: &str = "1";

/// Waarop de lijst van één map versmald wordt.
///
/// De twee filters werken samen en vervangen elkaar niet: staat er een
/// zoekterm én het aandachtsfilter, dan blijft over wat aan allebei voldoet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// De zoekterm zoals de gebruiker hem heeft ingevuld (FR-3).
    pub query: String,

    /// Alleen de bestanden met ten minste één signalering (FR-4).
    pub only_flagged: bool,
}

impl Filter {
    /// Het filter zoals het uit de querystring komt.
    ///
    /// De parameterwaarde wordt hier op één plek geduid, zodat de handler geen
    /// eigen idee kan krijgen over wat "aan" betekent.
    pub fn from_query(query: &str, attention: &str) -> Filter {
        Filter {
            query: query.to_string(),
            only_flagged: attention == ATTENTION_ON,
        }
    }
}

/// Eén stap in het broodkruimelpad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub name: String,
    pub url: String,
}

/// Een submap zoals de browser hem toont: één kaart in het raster.
///
/// De tellingen komen uit [`Library::summarize`] en dus uit de mapinhoud zelf —
/// namen, extensies, aantallen. Er wordt geen bestand geopend om de bibliotheek
/// te kunnen tonen, en er staat daarom ook geen signalering op een kaart: die
/// zou elk bestand in elke submap moeten lezen, en op een NAS met een grote
/// bibliotheek is dat het verschil tussen een pagina die er staat en een pagina
/// waar je op wacht. De signalering blijft in de map die je opent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    pub url: String,

    /// Aantal bewerkbare bestanden dat direct in deze map staat.
    pub files: usize,

    /// Aantal submappen dat direct in deze map staat.
    pub subfolders: usize,

    /// De formaten die tussen die bestanden voorkomen, bijvoorbeeld `MP3`.
    ///
    /// Leeg wanneer er geen bewerkbare bestanden zijn; dan valt er ook geen
    /// formaat te noemen.
    pub formats: Vec<String>,
}

impl Folder {
    /// Wat er in deze map staat, in één regel.
    ///
    /// Bevat de map geen bewerkbare bestanden, dan staat er geen bestandstelling
    /// in: "0 bestanden" naast een map vol albums leest als een lege map. Er
    /// staat dan wat er wél is — het aantal submappen — of, als ook dat er niet
    /// is, dat er niets te bewerken valt.
    pub fn contents_label(&self) -> String {
        let mut parts = Vec::new();

        if self.files > 0 {
            parts.push(count_label(self.files, "bestand", "bestanden"));
        }

        if self.subfolders > 0 {
            parts.push(count_label(self.subfolders, "submap", "submappen"));
        }

        if parts.is_empty() {
            return NOTHING_EDITABLE.to_string();
        }

        parts.join(" · ")
    }

    /// Of er formaten te tonen zijn; zonder bestanden zijn die er niet.
    pub fn has_files(&self) -> bool {
        self.files > 0
    }
}

/// "1 bestand", "12 bestanden".
fn count_label(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// Eén regel in de bestandslijst.
///
/// De `Option`-velden komen rechtstreeks uit het tagmodel; de `*_label`-methoden
/// maken er tekst van die een template zonder verdere logica kan tonen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackSummary {
    /// Bestandsnaam, inclusief extensie.
    pub name: String,

    /// Pad relatief aan `MUSIC_ROOT`; het handvat voor latere bewerkacties.
    pub path: String,

    /// Het volledige genormaliseerde tagmodel van dit bestand.
    ///
    /// De lijst toont er maar een handvol velden van, maar de signalering
    /// kijkt ook naar albumartiest en jaar, en het bewerkformulier heeft
    /// straks alles nodig.
    pub tags: Tags,

    /// Wat er aan dit bestand mankeert; leeg wanneer er niets te melden is.
    pub issues: Vec<TrackIssue>,

    /// Tagblokken in dit bestand die niet bij het formaat horen, bij naam.
    ///
    /// Komt uit `tags::` en gaat naar de signalering; de lijst zelf toont het
    /// niet, want daar staat de melding die er uit volgt al.
    pub foreign_tags: Vec<String>,

    /// Speelduur als `m:ss`, of `u:mm:ss` vanaf een uur.
    pub duration: String,

    /// `MP3` of `FLAC`.
    pub format: String,

    /// Wat er over de embedded hoes bekend is; `None` wanneer het bestand er
    /// geen heeft.
    ///
    /// De maplijst gebruikt alleen het bestaan ervan — dat bepaalt of ze een
    /// afbeelding of een placeholder toont, zodat de browser geen verzoek doet
    /// dat toch een 404 oplevert. De signalering kijkt naar de inhoud: twee
    /// tracks van hetzelfde album horen dezelfde hoes te hebben.
    pub art: Option<crate::tags::ArtInfo>,

    /// URL van de verkleinde hoes. Alleen zinvol wanneer er een hoes is.
    pub art_url: String,

    /// URL van het bewerkformulier van dit bestand.
    ///
    /// De ingang naar de geavanceerde weergave zit dáár en niet in deze lijst:
    /// FR-7 beschrijft die als onderdeel van de bestandspagina, en een tweede
    /// link per regel maakt de lijst op een telefoon alleen maar drukker.
    pub edit_url: String,
}

impl TrackSummary {
    /// Of er een embedded hoes in dit bestand zit.
    pub fn has_art(&self) -> bool {
        self.art.is_some()
    }

    /// Of dit bestand ten minste één signalering heeft.
    ///
    /// Dit is wat het aandachtsfilter en de telling in de kopbalk tellen, en
    /// wat de kop boven een schijf per groep optelt. Het oordeel zelf komt van
    /// [`crate::checks`] en blijft constateren: hier wordt niets voorgesteld
    /// en niets gerepareerd, er wordt alleen geteld.
    pub fn needs_attention(&self) -> bool {
        !self.issues.is_empty()
    }

    /// Het tracknummer, of een lege tekst wanneer het ontbreekt.
    ///
    /// Bewust leeg en niet `—`: in een smalle kolom vóór de titel is een streepje
    /// per regel meer ruis dan informatie.
    pub fn track_label(&self) -> String {
        self.tags
            .track
            .map(|number| number.to_string())
            .unwrap_or_default()
    }

    /// De titel, of een streepje wanneer die ontbreekt.
    ///
    /// De bestandsnaam invullen zou vriendelijk lijken, maar verbergt precies
    /// wat de gebruiker moet zien: hier staat geen titel in het bestand.
    pub fn title_label(&self) -> &str {
        self.tags.title.as_deref().unwrap_or(MISSING)
    }

    pub fn artist_label(&self) -> &str {
        self.tags.artist.as_deref().unwrap_or(MISSING)
    }

    pub fn album_label(&self) -> &str {
        self.tags.album.as_deref().unwrap_or(MISSING)
    }

    /// Het discnummer, of een lege tekst wanneer het ontbreekt.
    ///
    /// Leeg en niet `—`, om dezelfde reden als bij het tracknummer: in een
    /// smalle getalkolom is een streepje per regel meer ruis dan informatie.
    /// Een map zonder discnummers houdt zo een lege kolom in plaats van een
    /// kolom vol streepjes.
    pub fn disc_label(&self) -> String {
        self.tags
            .disc
            .map(|number| number.to_string())
            .unwrap_or_default()
    }

    /// Het jaar zoals het in het bestand staat, of een streepje.
    ///
    /// Niet als getal: in het tagmodel is dit tekst, omdat ID3v2.4 en Vorbis er
    /// een volledige datum in kunnen zetten. Wat er staat, staat er.
    pub fn year_label(&self) -> &str {
        self.tags.year.as_deref().unwrap_or(MISSING)
    }

    pub fn genre_label(&self) -> &str {
        self.tags.genre.as_deref().unwrap_or(MISSING)
    }
}

/// Eén schijf uit de bestandslijst.
///
/// De lijst staat op discnummer gegroepeerd; deze structuur beschrijft wat er
/// in de kop boven zo'n groep komt te staan. Ze verwijst naar de bestanden met
/// een positie en een aantal en houdt er geen kopie van: de bestanden zelf
/// staan in [`Listing::tracks`], in precies deze volgorde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscGroup {
    /// Het discnummer, of `None` voor de bestanden die er geen hebben.
    pub disc: Option<u32>,

    /// De positie van het eerste bestand van deze groep in de lijst.
    pub start: usize,

    /// Hoeveel bestanden er in deze groep zitten.
    pub count: usize,

    /// Hoeveel van die bestanden aandacht vragen (FR-4).
    pub attention: usize,
}

impl DiscGroup {
    /// Het opschrift van deze groep.
    ///
    /// Bestanden zonder discnummer krijgen geen verzonnen nummer maar worden
    /// benoemd om wat ze zijn: er valt niet te zeggen bij welke schijf ze
    /// horen.
    pub fn label(&self) -> String {
        match self.disc {
            Some(number) => format!("Schijf {number}"),
            None => NO_DISC.to_string(),
        }
    }

    /// De sleutel van deze groep in een formulier; leeg voor "zonder
    /// discnummer".
    pub fn key(&self) -> String {
        self.disc
            .map(|number| number.to_string())
            .unwrap_or_default()
    }

    /// Hoeveel bestanden er in deze groep zitten, als tekst.
    pub fn count_label(&self) -> String {
        match self.count {
            1 => "1 bestand".to_string(),
            count => format!("{count} bestanden"),
        }
    }

    /// Hoeveel bestanden er aandacht vragen; `None` wanneer dat er geen zijn.
    ///
    /// Bewust niets in plaats van "0 vragen aandacht": een kop die bij elke
    /// groep meldt dat er niets aan de hand is, is ruis (AC #3).
    pub fn attention_label(&self) -> Option<String> {
        match self.attention {
            0 => None,
            1 => Some("1 vraagt aandacht".to_string()),
            count => Some(format!("{count} vragen aandacht")),
        }
    }

    /// De hele kop in één zin: de telling en wat er aandacht vraagt.
    pub fn describe(&self) -> String {
        match self.attention_label() {
            Some(attention) => format!("{}, {attention}", self.count_label()),
            None => self.count_label(),
        }
    }
}

/// Groepeert een gesorteerde bestandslijst op discnummer.
///
/// De lijst komt uit [`sort_tracks`] en staat dus al per schijf bij elkaar,
/// met de bestanden zonder discnummer achteraan; deze functie hoeft er alleen
/// nog de grenzen in te vinden. Ze telt ook per groep hoeveel bestanden er
/// aandacht vragen, zodat de kop dat kan melden.
pub fn disc_groups(tracks: &[TrackSummary]) -> Vec<DiscGroup> {
    let mut groups: Vec<DiscGroup> = Vec::new();

    for (index, track) in tracks.iter().enumerate() {
        let attention = usize::from(track.needs_attention());

        match groups.last_mut() {
            Some(group) if group.disc == track.tags.disc => {
                group.count += 1;
                group.attention += attention;
            }
            _ => groups.push(DiscGroup {
                disc: track.tags.disc,
                start: index,
                count: 1,
                attention,
            }),
        }
    }

    groups
}

/// Alles wat één mappagina nodig heeft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// Naam van de map zelf; voor de root de naam van de bibliotheek.
    pub name: String,

    /// Pad van de map relatief aan `MUSIC_ROOT`; leeg voor de root.
    pub path: String,

    /// URL van deze pagina, zonder query. Het formulier en de HTMX-aanroep
    /// wijzen hiernaartoe.
    pub url: String,

    /// URL van de albumweergave van deze map: dezelfde bestanden, maar dan om
    /// er met een selectie tegelijk aan te werken (FR-8).
    pub album_url: String,

    /// Van de bibliotheekwortel tot en met deze map.
    pub crumbs: Vec<Crumb>,

    pub folders: Vec<Folder>,
    pub tracks: Vec<TrackSummary>,

    /// De schijven waarin `tracks` uiteenvalt, in dezelfde volgorde.
    ///
    /// Wordt bepaald over de lijst zoals hij op het scherm komt — dus ná het
    /// filteren: een kop die twaalf bestanden telt terwijl er twee te zien
    /// zijn, telt iets anders dan wat eronder staat. De signalering per map
    /// gaat wél over de hele map; die zegt iets over de map en niet over de
    /// lijst.
    pub groups: Vec<DiscGroup>,

    /// Wat er tussen de bestanden van deze map onderling niet klopt.
    ///
    /// Wordt over de héle map bepaald, ook wanneer er gefilterd wordt: aan de
    /// map verandert niets doordat de gebruiker zoekt.
    pub folder_issues: Vec<FolderIssue>,

    /// De zoekterm zoals de gebruiker hem heeft ingevuld.
    pub query: String,

    /// Hoeveel bestanden in deze map ten minste één signalering hebben.
    ///
    /// Over de héle map geteld, net als [`Listing::folder_issues`]: de telling
    /// hoort bij de map die je bekijkt en niet bij wat er na het filteren
    /// toevallig overblijft. Anders zou hij bij elke aanslag in het zoekveld
    /// iets anders beweren.
    pub flagged_count: usize,

    /// Of de lijst tot die bestanden beperkt is.
    pub only_flagged: bool,
}

impl Listing {
    /// Of er in deze map iets aandacht vraagt.
    pub fn has_flagged(&self) -> bool {
        self.flagged_count > 0
    }

    /// De URL die het aandachtsfilter omzet en de zoekterm laat staan.
    ///
    /// Staat het filter aan, dan laat deze link de parameter weg en komt de
    /// hele lijst terug; staat het uit, dan zet hij hem aan. Eén gewone link
    /// dus, die het ook zonder JavaScript doet.
    pub fn attention_url(&self) -> String {
        let mut params: Vec<String> = Vec::new();

        if !self.query.is_empty() {
            params.push(format!(
                "q={}",
                utf8_percent_encode(&self.query, QUERY_ESCAPES)
            ));
        }

        if !self.only_flagged {
            params.push(format!("{ATTENTION_PARAM}={ATTENTION_ON}"));
        }

        if params.is_empty() {
            self.url.clone()
        } else {
            format!("{}?{}", self.url, params.join("&"))
        }
    }

    /// Wat er in deze lijst staat, in één regel naast de mapnaam.
    ///
    /// Hoeveel bestanden er te zien zijn, en — alleen wanneer er discnummers
    /// in de map staan — hoeveel schijven dat zijn. Het gaat over de lijst
    /// zoals hij op het scherm komt, dus over wat er na het filteren overblijft;
    /// dat is wat je eronder telt. De telling van wat aandacht vraagt is iets
    /// anders: die hoort bij de map en staat in de kopbalk.
    ///
    /// `None` wanneer er geen bestanden zijn: dan staat er al een uitleg
    /// waarom de lijst leeg is, en "0 bestanden" naast de mapnaam voegt daar
    /// niets aan toe.
    pub fn summary_label(&self) -> Option<String> {
        if self.tracks.is_empty() {
            return None;
        }

        let files = match self.tracks.len() {
            1 => "1 bestand".to_string(),
            count => format!("{count} bestanden"),
        };

        if !self.is_grouped() {
            return Some(files);
        }

        // De groep bestanden zónder discnummer telt niet als schijf: van die
        // bestanden is juist niet te zeggen bij welke schijf ze horen.
        let numbered = self
            .groups
            .iter()
            .filter(|group| group.disc.is_some())
            .count();

        let discs = match numbered {
            1 => "1 schijf".to_string(),
            count => format!("{count} schijven"),
        };

        Some(format!("{files} · {discs}"))
    }
}

impl Listing {
    /// Of de lijst koppen per schijf krijgt.
    ///
    /// Alleen wanneer er ergens een discnummer staat. Een map waarin geen
    /// enkel bestand er een heeft, is één doorlopende lijst en heeft niets aan
    /// een kop die dat nog eens zegt (AC #5).
    pub fn is_grouped(&self) -> bool {
        self.groups.iter().any(|group| group.disc.is_some())
    }

    /// De groep die bij dit bestand begint, als er hier een kop hoort.
    ///
    /// De koppen staan tússen de bestanden en niet eromheen: zo blijft de
    /// lijst één opsomming, en blijft de rij ernaast wat ze was.
    pub fn group_starting_at(&self, index: usize) -> Option<&DiscGroup> {
        if !self.is_grouped() {
            return None;
        }

        self.groups.iter().find(|group| group.start == index)
    }
}

/// Bouwt het weergavemodel van één map.
///
/// `relative` is het door de gebruiker aangeleverde pad en gaat ongewijzigd naar
/// [`Library::list_directory`], die het controleert. `filter` versmalt binnen
/// deze map: op bestandsnaam of titel, op wat aandacht vraagt, of op allebei.
///
/// Dit is blokkerende I/O: elk bestand wordt geopend om zijn tags te lezen. De
/// aanroeper hoort dat buiten de async-runtime te doen.
pub fn listing(library: &Library, relative: &str, filter: &Filter) -> Result<Listing, PathError> {
    let contents = library.list_directory(relative)?;

    let path = library
        .relative_path(&contents.path)
        .map(to_url_path)
        .unwrap_or_default();

    let needle = filter.query.trim().to_lowercase();

    let folders: Vec<Folder> = contents
        .directories
        .iter()
        .filter(|entry| needle.is_empty() || entry.name.to_lowercase().contains(&needle))
        .map(|entry| {
            // Eén `read_dir` per getoonde map, en alleen voor de mappen die het
            // filter overleven. Er gaat geen bestand open: wat op de kaart komt
            // volgt uit de namen in de map.
            let summary = library.summarize(&entry.path);

            Folder {
                url: url_for(&join(&path, &entry.name)),
                name: entry.name.clone(),
                files: summary.files,
                subfolders: summary.directories,
                formats: summary.formats,
            }
        })
        .collect();

    // Eerst de hele map inlezen en beoordelen, dan pas filteren: een melding
    // als "twee verschillende albumtitels" hoort niet te verdwijnen zodra de
    // gebruiker zoekt.
    let mut tracks: Vec<TrackSummary> = contents
        .files
        .iter()
        .filter_map(|entry| summarize(entry, &path))
        .collect();

    let folder_issues = review(&mut tracks);

    // Tellen vóór het filteren, en over alle bestanden: wat de kopbalk meldt
    // gaat over de map, niet over wat er na een zoekterm van over is.
    let flagged_count = tracks
        .iter()
        .filter(|track| track.needs_attention())
        .count();

    if filter.only_flagged {
        tracks.retain(TrackSummary::needs_attention);
    }

    tracks.retain(|track| matches_query(track, &needle));
    sort_tracks(&mut tracks);

    let groups = disc_groups(&tracks);

    Ok(Listing {
        groups,
        folder_issues,
        flagged_count,
        only_flagged: filter.only_flagged,
        name: name_of(&path),
        crumbs: crumbs_for(&path),
        url: url_for(&path),
        album_url: album_url(&path),
        path,
        folders,
        tracks,
        query: filter.query.trim().to_string(),
    })
}

/// Leest één bestand en maakt er een lijstregel van.
///
/// Levert `None` wanneer de tags niet te lezen zijn. Dat is geen fout die de
/// pagina hoort te breken: het betekent dat het bestand ondanks zijn extensie
/// geen MP3 of FLAC is, en dus niet bewerkbaar. Dat is precies het oordeel van
/// `fs::is_editable`, maar dan zonder het bestand een tweede keer te openen.
fn summarize(entry: &DirEntry, directory: &str) -> Option<TrackSummary> {
    let track = match tags::read(&entry.path) {
        Ok(track) => track,
        Err(error) => {
            tracing::debug!(
                path = %entry.path.display(),
                %error,
                "bestand overgeslagen: niet als audio te lezen"
            );
            return None;
        }
    };

    let path = join(directory, &entry.name);

    Some(TrackSummary {
        art_url: thumbnail_url(&path),
        edit_url: edit_url(&path),
        path,
        name: entry.name.clone(),
        duration: format_duration(track.duration),
        format: track.format.to_string(),
        art: track.art,
        tags: track.tags,
        foreign_tags: track.foreign_tags,
        // Wordt hierna ingevuld: wat er aan één bestand mankeert hangt mede af
        // van de rest van de map.
        issues: Vec::new(),
    })
}

/// Laat de signalering over de hele map lopen en hangt de bevindingen op.
///
/// Geeft de meldingen op mapniveau terug; die per bestand komen op de rij zelf
/// terecht.
fn review(tracks: &mut [TrackSummary]) -> Vec<FolderIssue> {
    let entries: Vec<checks::Entry<'_>> = tracks
        .iter()
        .map(|track| checks::Entry {
            tags: &track.tags,
            art: track.art.as_ref(),
            foreign_tags: &track.foreign_tags,
        })
        .collect();

    let review = checks::review(&entries);

    for (track, issues) in tracks.iter_mut().zip(review.tracks) {
        track.issues = issues;
    }

    review.folder
}

/// Sorteert op schijf, dan op tracknummer, met de bestandsnaam als terugval.
///
/// Antwoord op het open punt in PRD §12: het tracknummer uit de tags bepaalt de
/// volgorde, want dat is de volgorde waarin het album bedoeld is. Bestanden
/// zonder tracknummer kunnen daar niet tussen worden geplaatst en komen
/// erachter, onderling op naam.
///
/// Het discnummer gaat daarvóór, want anders staat track 1 van de tweede schijf
/// tussen de eerste tracks van de eerste: bij een set is dat precies de
/// verwarring waar het misgaat. Bestanden zonder discnummer komen achteraan, om
/// dezelfde reden als bestanden zonder tracknummer — waar ze horen is niet te
/// zeggen. Binnen een schijf verandert er niets aan de volgorde die er al was,
/// en in een map zonder discnummers verandert er dus helemaal niets.
fn sort_tracks(tracks: &mut [TrackSummary]) {
    tracks.sort_by(|a, b| {
        a.tags
            .disc
            .unwrap_or(u32::MAX)
            .cmp(&b.tags.disc.unwrap_or(u32::MAX))
            .then_with(|| {
                a.tags
                    .track
                    .unwrap_or(u32::MAX)
                    .cmp(&b.tags.track.unwrap_or(u32::MAX))
            })
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Of een regel bij de zoekterm past (FR-3): bestandsnaam of titel.
fn matches_query(track: &TrackSummary, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    track.name.to_lowercase().contains(needle)
        || track
            .tags
            .title
            .as_deref()
            .is_some_and(|title| title.to_lowercase().contains(needle))
}

/// Zet een speelduur om naar `m:ss`, of `u:mm:ss` vanaf een uur.
pub fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let (hours, minutes, seconds) = (total / 3600, (total % 3600) / 60, total % 60);

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// De naam van de map zelf, of die van de bibliotheek voor de root.
fn name_of(path: &str) -> String {
    path.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(ROOT_NAME)
        .to_string()
}

/// Het broodkruimelpad van de wortel tot en met deze map (AC #2).
fn crumbs_for(path: &str) -> Vec<Crumb> {
    let mut crumbs = vec![Crumb {
        name: ROOT_NAME.to_string(),
        url: url_for(""),
    }];

    let mut walked = String::new();
    for part in path.split('/').filter(|part| !part.is_empty()) {
        walked = join(&walked, part);
        crumbs.push(Crumb {
            name: part.to_string(),
            url: url_for(&walked),
        });
    }

    crumbs
}

/// De URL van een mappagina; de root is de startpagina.
pub fn url_for(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/map/{}", encode(path))
    }
}

/// De URL van de albumweergave van een map (FR-8).
///
/// De wortel heeft geen padsegment; die krijgt de kale route, net zoals de
/// mapweergave daar `/` gebruikt.
pub fn album_url(path: &str) -> String {
    if path.is_empty() {
        "/album".to_string()
    } else {
        format!("/album/{}", encode(path))
    }
}

/// De URL van de verkleinde hoes van één bestand.
fn thumbnail_url(path: &str) -> String {
    format!("/art/{}?size={THUMBNAIL_SIZE_PARAM}", encode(path))
}

/// De URL van de geavanceerde weergave van één bestand.
pub fn raw_tags_url(path: &str) -> String {
    format!("/tags/{}", encode(path))
}

/// De URL van het bewerkformulier van één bestand.
pub fn edit_url(path: &str) -> String {
    format!("/bewerk/{}", encode(path))
}

/// Wie het bewerkformulier heeft geopend, zodat de weg terug erheen leidt.
///
/// Zonder dit komt iedereen op de maplijst uit, ook wie uit de albumweergave
/// kwam — en juist daar heeft de gebruiker net een selectie gemaakt waar hij
/// naar terug wil.
pub const FROM_ALBUM: &str = "album";

/// De URL van het bewerkformulier, met de albumweergave als herkomst.
pub fn edit_url_from_album(path: &str) -> String {
    format!("{}?terug={FROM_ALBUM}", edit_url(path))
}

/// De URL van de hoes op ware grootte.
pub fn art_url(path: &str) -> String {
    format!("/art/{}", encode(path))
}

/// De URL van de hoesweergave van één bestand (FR-12).
///
/// Niet te verwarren met [`art_url`]: dat is de afbeelding zelf, dit is de
/// pagina eromheen.
pub fn cover_url(path: &str) -> String {
    format!("/hoes/{}", encode(path))
}

/// Broodkruimels tot en met de map waarin dit bestand staat.
///
/// De bestandsnaam zelf hoort er niet bij: die is de kop van de pagina, en een
/// bestand is geen map om naartoe te navigeren.
pub fn crumbs_to_parent(path: &str) -> Vec<Crumb> {
    crumbs_for(parent_of(path))
}

/// De naam van een bestand, los van het pad ernaartoe.
pub fn name_of_file(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Het pad van de map waarin dit bestand staat; leeg voor de wortel.
pub fn parent_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(index) => &path[..index],
        None => "",
    }
}

/// Codeert een relatief pad voor gebruik in een URL.
fn encode(path: &str) -> impl std::fmt::Display {
    utf8_percent_encode(path, PATH_ESCAPES)
}

/// Plakt een naam achter een relatief pad.
fn join(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.to_string()
    } else {
        format!("{directory}/{name}")
    }
}

/// Zet een relatief pad om naar de `/`-notatie die in een URL past.
fn to_url_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testfixtures;

    /// Bouwt een bibliotheek met één album erin.
    ///
    /// De root is een tempdir en wordt gecanonicaliseerd, omdat macOS `/var`
    /// naar `/private/var` laat wijzen.
    fn library_with_album() -> (tempfile::TempDir, Library) {
        let tempdir = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        std::fs::create_dir_all(tempdir.path().join("Artiest").join("Album"))
            .expect("albummap moet aan te maken zijn");

        let root =
            std::fs::canonicalize(tempdir.path()).expect("root moet te canonicaliseren zijn");
        (tempdir, Library::new(root))
    }

    /// Kopieert een fixture naar de albummap onder een eigen naam.
    ///
    /// De naam is los van de fixture, zodat een test meerdere tracks met
    /// verschillende bestandsnamen in dezelfde map kan zetten.
    fn place(library: &Library, name: &str, fixture: &str) {
        let album = library.root().join("Artiest").join("Album");
        std::fs::copy(testfixtures::fixture_path(fixture), album.join(name))
            .expect("fixture moet te kopiëren zijn");
    }

    fn album_listing(library: &Library, query: &str) -> Listing {
        listing(library, "Artiest/Album", &Filter::from_query(query, ""))
            .expect("de albummap moet te tonen zijn")
    }

    #[test]
    fn shows_the_fields_from_the_tag_model() {
        let (_tempdir, library) = library_with_album();
        place(&library, "tagged.mp3", testfixtures::MP3_WITH_TAGS);

        let listing = album_listing(&library, "");
        let track = listing.tracks.first().expect("er moet één track staan");

        assert_eq!(track.name, "tagged.mp3");
        assert_eq!(track.path, "Artiest/Album/tagged.mp3");
        assert_eq!(track.format, "MP3");
        assert!(track.tags.track.is_some(), "tracknummer ontbreekt");
        assert!(track.tags.title.is_some(), "titel ontbreekt");
        assert!(track.tags.artist.is_some(), "artiest ontbreekt");
        assert!(track.tags.album.is_some(), "album ontbreekt");
        assert_eq!(track.duration, "0:01", "de fixture is één seconde stilte");
    }

    #[test]
    fn shows_subdirectories_and_starts_at_the_root() {
        let (_tempdir, library) = library_with_album();

        let root = listing(&library, "", &Filter::default()).expect("de root moet te tonen zijn");

        assert_eq!(root.name, ROOT_NAME);
        assert_eq!(root.path, "");
        assert_eq!(root.url, "/");
        assert_eq!(
            root.folders,
            vec![Folder {
                name: "Artiest".to_string(),
                url: "/map/Artiest".to_string(),
                files: 0,
                subfolders: 1,
                formats: Vec::new(),
            }]
        );
        assert!(root.tracks.is_empty());
    }

    #[test]
    fn breadcrumbs_lead_back_to_the_root() {
        let (_tempdir, library) = library_with_album();

        let listing = album_listing(&library, "");

        assert_eq!(
            listing.crumbs,
            vec![
                Crumb {
                    name: ROOT_NAME.to_string(),
                    url: "/".to_string()
                },
                Crumb {
                    name: "Artiest".to_string(),
                    url: "/map/Artiest".to_string()
                },
                Crumb {
                    name: "Album".to_string(),
                    url: "/map/Artiest/Album".to_string()
                },
            ]
        );
    }

    #[test]
    fn sorts_by_track_number_with_the_filename_as_fallback() {
        let (_tempdir, library) = library_with_album();

        // De getagde fixture heeft tracknummer 3; de ongetagde heeft er geen.
        place(&library, "b-met-nummer.mp3", testfixtures::MP3_WITH_TAGS);
        place(
            &library,
            "c-zonder-nummer.mp3",
            testfixtures::MP3_WITHOUT_TAGS,
        );
        place(
            &library,
            "a-zonder-nummer.flac",
            testfixtures::FLAC_WITHOUT_TAGS,
        );

        let listing = album_listing(&library, "");
        let order: Vec<&str> = listing
            .tracks
            .iter()
            .map(|track| track.name.as_str())
            .collect();

        assert_eq!(
            order,
            vec![
                "b-met-nummer.mp3",
                "a-zonder-nummer.flac",
                "c-zonder-nummer.mp3"
            ],
            "een tracknummer gaat voor; de rest volgt op bestandsnaam"
        );
    }

    #[test]
    fn filters_on_the_filename() {
        let (_tempdir, library) = library_with_album();
        place(&library, "eerste.mp3", testfixtures::MP3_WITHOUT_TAGS);
        place(&library, "tweede.mp3", testfixtures::MP3_WITHOUT_TAGS);

        let listing = album_listing(&library, "TWEE");

        assert_eq!(listing.tracks.len(), 1, "filter is hoofdletterongevoelig");
        assert_eq!(listing.tracks[0].name, "tweede.mp3");
        assert_eq!(listing.query, "TWEE");
    }

    #[test]
    fn filters_on_the_title() {
        let (_tempdir, library) = library_with_album();
        place(&library, "aaa.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "bbb.mp3", testfixtures::MP3_WITHOUT_TAGS);

        let title = {
            let all = album_listing(&library, "");
            all.tracks
                .iter()
                .find_map(|track| track.tags.title.clone())
                .expect("de getagde fixture heeft een titel")
        };

        // Een deel van de titel dat niet in de bestandsnaam voorkomt.
        let listing = album_listing(&library, &title);

        assert_eq!(listing.tracks.len(), 1);
        assert_eq!(listing.tracks[0].name, "aaa.mp3");
    }

    #[test]
    fn filter_also_applies_to_subdirectories() {
        let (_tempdir, library) = library_with_album();
        std::fs::create_dir(library.root().join("Andere artiest"))
            .expect("map moet aan te maken zijn");

        let listing = listing(&library, "", &Filter::from_query("artiest", ""))
            .expect("de root moet te tonen zijn");

        assert_eq!(
            listing.folders.iter().map(|f| &f.name).collect::<Vec<_>>(),
            vec!["Andere artiest", "Artiest"]
        );

        let none = listing_of_root(&library, "bestaat niet");
        assert!(none.folders.is_empty());
    }

    fn listing_of_root(library: &Library, query: &str) -> Listing {
        listing(library, "", &Filter::from_query(query, "")).expect("de root moet te tonen zijn")
    }

    #[test]
    fn skips_a_file_that_only_looks_like_audio() {
        let (_tempdir, library) = library_with_album();
        place(&library, "echt.mp3", testfixtures::MP3_WITH_TAGS);

        // Juiste extensie, verkeerde inhoud: die hoort niet als bewerkbaar
        // bestand in de lijst te staan.
        place(&library, "nep.mp3", testfixtures::COVER_JPEG);

        let listing = album_listing(&library, "");

        assert_eq!(
            listing.tracks.iter().map(|t| &t.name).collect::<Vec<_>>(),
            vec!["echt.mp3"]
        );
    }

    #[test]
    fn labels_fill_in_missing_tags() {
        let (_tempdir, library) = library_with_album();
        place(&library, "kaal.flac", testfixtures::FLAC_WITHOUT_TAGS);

        let listing = album_listing(&library, "");
        let track = listing.tracks.first().expect("er moet één track staan");

        assert_eq!(track.track_label(), "");
        assert_eq!(track.title_label(), MISSING);
        assert_eq!(track.artist_label(), MISSING);
        assert_eq!(track.album_label(), MISSING);
        assert_eq!(track.format, "FLAC");
        assert!(!track.has_art());
    }

    #[test]
    fn reports_embedded_art_with_a_thumbnail_url() {
        let (_tempdir, library) = library_with_album();
        place(&library, "hoes.mp3", testfixtures::MP3_WITH_ART);

        let listing = album_listing(&library, "");
        let track = &listing.tracks[0];

        assert!(track.has_art(), "de fixture heeft een hoes");
        assert_eq!(
            track.art_url, "/art/Artiest/Album/hoes.mp3?size=thumb",
            "de lijst hoort de verkleinde variant op te vragen"
        );
    }

    #[test]
    fn a_thumbnail_url_escapes_the_path() {
        assert_eq!(
            thumbnail_url("Sigur Rós/( )/01 intro.flac"),
            "/art/Sigur%20R%C3%B3s/(%20)/01%20intro.flac?size=thumb"
        );
    }

    #[test]
    fn issues_land_on_the_file_they_belong_to() {
        let (_tempdir, library) = library_with_album();
        place(&library, "compleet.mp3", testfixtures::MP3_WITH_ART);
        place(&library, "kaal.mp3", testfixtures::MP3_WITHOUT_TAGS);

        let listing = album_listing(&library, "");

        let complete = listing
            .tracks
            .iter()
            .find(|track| track.name == "compleet.mp3")
            .expect("het volledig getagde bestand moet er staan");
        assert!(
            complete.issues.is_empty(),
            "onterechte meldingen: {:?}",
            complete.issues
        );

        let bare = listing
            .tracks
            .iter()
            .find(|track| track.name == "kaal.mp3")
            .expect("het ongetagde bestand moet er staan");
        assert!(bare.issues.contains(&TrackIssue::MissingTitle));
        assert!(bare.issues.contains(&TrackIssue::MissingArt));
        assert!(bare.issues.contains(&TrackIssue::MissingTrackNumber));
    }

    #[test]
    fn folder_issues_describe_the_whole_directory() {
        let (_tempdir, library) = library_with_album();
        // Beide fixtures hebben tracknummer 3 en hetzelfde album, dus dit
        // levert een dubbel tracknummer op maar geen afwijkende albumtitel.
        place(&library, "een.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "twee.flac", testfixtures::FLAC_WITH_TAGS);

        let listing = album_listing(&library, "");

        assert!(
            listing
                .folder_issues
                .contains(&FolderIssue::DuplicateTrackNumbers(vec![3])),
            "gevonden: {:?}",
            listing.folder_issues
        );
    }

    #[test]
    fn folder_issues_survive_a_filter() {
        let (_tempdir, library) = library_with_album();
        place(&library, "een.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "twee.flac", testfixtures::FLAC_WITH_TAGS);

        let everything = album_listing(&library, "");
        let filtered = album_listing(&library, "een");

        assert_eq!(filtered.tracks.len(), 1, "het filter hoort te werken");
        assert_eq!(
            filtered.folder_issues, everything.folder_issues,
            "aan de map verandert niets doordat de gebruiker zoekt"
        );
    }

    #[test]
    fn a_tidy_directory_reports_nothing() {
        let (_tempdir, library) = library_with_album();
        place(&library, "hoes.mp3", testfixtures::MP3_WITH_ART);

        let listing = album_listing(&library, "");

        assert!(
            listing.folder_issues.is_empty(),
            "gevonden: {:?}",
            listing.folder_issues
        );
        assert!(
            listing.tracks[0].issues.is_empty(),
            "gevonden: {:?}",
            listing.tracks[0].issues
        );
    }

    /// De albummap met een uitgesproken filter.
    fn album_listing_with(library: &Library, filter: &Filter) -> Listing {
        listing(library, "Artiest/Album", filter).expect("de albummap moet te tonen zijn")
    }

    /// Een filter met zoekterm én aandachtsfilter.
    fn attention_and(query: &str) -> Filter {
        Filter {
            query: query.to_string(),
            only_flagged: true,
        }
    }

    #[test]
    fn counts_the_files_that_need_attention() {
        let (_tempdir, library) = library_with_album();
        // Compleet getagd, mét hoes: hier valt niets over te melden.
        place(&library, "compleet.mp3", testfixtures::MP3_WITH_ART);
        place(&library, "kaal.mp3", testfixtures::MP3_WITHOUT_TAGS);
        place(&library, "ook-kaal.flac", testfixtures::FLAC_WITHOUT_TAGS);

        let listing = album_listing(&library, "");

        assert_eq!(
            listing.flagged_count, 2,
            "twee van de drie missen van alles"
        );
        assert!(listing.has_flagged());
        assert!(!listing.only_flagged, "zonder filter staat de lijst open");
        assert_eq!(listing.tracks.len(), 3, "ongefilterd staat alles er nog");
    }

    #[test]
    fn a_tidy_directory_counts_nothing() {
        let (_tempdir, library) = library_with_album();
        place(&library, "hoes.mp3", testfixtures::MP3_WITH_ART);

        let listing = album_listing(&library, "");

        assert_eq!(listing.flagged_count, 0);
        assert!(!listing.has_flagged());
    }

    #[test]
    fn the_filter_keeps_only_what_needs_attention() {
        let (_tempdir, library) = library_with_album();
        place(&library, "compleet.mp3", testfixtures::MP3_WITH_ART);
        place(&library, "kaal.mp3", testfixtures::MP3_WITHOUT_TAGS);

        let listing = album_listing_with(&library, &attention_and(""));

        assert!(listing.only_flagged);
        assert_eq!(
            listing.tracks.iter().map(|t| &t.name).collect::<Vec<_>>(),
            vec!["kaal.mp3"]
        );
        assert_eq!(
            listing.flagged_count, 1,
            "de telling blijft over de hele map gaan"
        );
    }

    #[test]
    fn the_filter_and_the_search_term_narrow_together() {
        let (_tempdir, library) = library_with_album();
        place(&library, "compleet.mp3", testfixtures::MP3_WITH_ART);
        place(&library, "kaal-een.mp3", testfixtures::MP3_WITHOUT_TAGS);
        place(&library, "kaal-twee.flac", testfixtures::FLAC_WITHOUT_TAGS);

        // Beide filters samen: alleen wat aan allebei voldoet blijft over.
        let both = album_listing_with(&library, &attention_and("twee"));
        assert_eq!(
            both.tracks.iter().map(|t| &t.name).collect::<Vec<_>>(),
            vec!["kaal-twee.flac"]
        );

        // Een zoekterm die alleen het nette bestand vindt, houdt met het
        // aandachtsfilter erbij niets over — geen OR dus.
        let none = album_listing_with(&library, &attention_and("compleet"));
        assert!(none.tracks.is_empty(), "gevonden: {:?}", none.tracks);
        assert_eq!(none.flagged_count, 2);

        // Zonder het aandachtsfilter komt datzelfde bestand gewoon terug.
        let search_only = album_listing(&library, "compleet");
        assert_eq!(search_only.tracks.len(), 1);
    }

    #[test]
    fn a_tidy_directory_has_nothing_left_under_the_filter() {
        let (_tempdir, library) = library_with_album();
        place(&library, "hoes.mp3", testfixtures::MP3_WITH_ART);

        let listing = album_listing_with(&library, &attention_and(""));

        assert!(listing.tracks.is_empty());
        assert_eq!(listing.flagged_count, 0);
        assert!(
            listing.only_flagged,
            "de weergave moet kunnen uitleggen waarom"
        );
    }

    #[test]
    fn folder_issues_survive_the_attention_filter() {
        let (_tempdir, library) = library_with_album();
        place(&library, "een.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "twee.flac", testfixtures::FLAC_WITH_TAGS);

        let everything = album_listing(&library, "");
        let filtered = album_listing_with(&library, &attention_and(""));

        assert_eq!(
            filtered.folder_issues, everything.folder_issues,
            "aan de map verandert niets doordat de gebruiker filtert"
        );
    }

    #[test]
    fn the_attention_link_toggles_and_keeps_the_search_term() {
        let (_tempdir, library) = library_with_album();
        place(&library, "kaal.mp3", testfixtures::MP3_WITHOUT_TAGS);

        // Uit: de link zet hem aan.
        let off = album_listing(&library, "");
        assert_eq!(off.attention_url(), "/map/Artiest/Album?aandacht=1");

        // Aan: de link laat de parameter weg en toont weer alles.
        let on = album_listing_with(&library, &attention_and(""));
        assert_eq!(on.attention_url(), "/map/Artiest/Album");

        // De zoekterm reist mee, in beide richtingen.
        let searching = album_listing(&library, "kaal");
        assert_eq!(
            searching.attention_url(),
            "/map/Artiest/Album?q=kaal&aandacht=1"
        );
        let both = album_listing_with(&library, &attention_and("kaal"));
        assert_eq!(both.attention_url(), "/map/Artiest/Album?q=kaal");
    }

    #[test]
    fn the_attention_link_escapes_the_search_term() {
        let (_tempdir, library) = library_with_album();

        let listing = album_listing(&library, "a&b=c d");

        assert_eq!(
            listing.attention_url(),
            "/map/Artiest/Album?q=a%26b%3Dc%20d&aandacht=1",
            "een zoekterm mag de rest van de URL niet kunnen kapotmaken"
        );
    }

    #[test]
    fn the_query_parameter_decides_what_is_on() {
        assert_eq!(
            Filter::from_query(" iets ", ATTENTION_ON),
            Filter {
                query: " iets ".to_string(),
                only_flagged: true,
            }
        );
        assert!(!Filter::from_query("", "").only_flagged);
        assert!(!Filter::from_query("", "0").only_flagged);
        assert_eq!(Filter::default(), Filter::from_query("", ""));
    }

    #[test]
    fn refuses_a_path_outside_the_library() {
        let (_tempdir, library) = library_with_album();

        assert_eq!(
            listing(&library, "../..", &Filter::default()).unwrap_err(),
            PathError::OutsideLibrary
        );
    }

    #[test]
    fn durations_are_readable() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0:00");
        assert_eq!(format_duration(Duration::from_secs(9)), "0:09");
        assert_eq!(format_duration(Duration::from_secs(204)), "3:24");
        assert_eq!(format_duration(Duration::from_secs(3600)), "1:00:00");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1:02:05");
    }

    #[test]
    fn urls_escape_what_a_path_may_contain() {
        assert_eq!(url_for(""), "/");
        assert_eq!(url_for("Artiest/Album"), "/map/Artiest/Album");
        assert_eq!(
            url_for("Sigur Rós/( )"),
            "/map/Sigur%20R%C3%B3s/(%20)",
            "spaties en accenten horen gecodeerd te worden, de scheidende / niet"
        );
        assert_eq!(url_for("vraag? #1"), "/map/vraag%3F%20%231");
    }

    #[test]
    fn a_directory_with_thirty_tracks_stays_quick() {
        // De eis uit PRD §8.5 geldt op de NAS (task-27). Deze test bewaakt dat
        // de mapweergave lineair blijft in het aantal bestanden en niet stiekem
        // per bestand meerdere keren opent.
        let (_tempdir, library) = library_with_album();
        for number in 1..=30 {
            place(
                &library,
                &format!("track-{number:02}.mp3"),
                testfixtures::MP3_WITH_TAGS,
            );
        }

        let start = std::time::Instant::now();
        let listing = album_listing(&library, "");
        let elapsed = start.elapsed();

        assert_eq!(listing.tracks.len(), 30);
        assert!(
            elapsed < Duration::from_secs(1),
            "dertig tracks kostten {elapsed:?}"
        );
    }

    #[test]
    fn a_folder_card_counts_what_is_in_the_directory() {
        let (_tempdir, library) = library_with_album();
        place(&library, "een.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "twee.mp3", testfixtures::MP3_WITHOUT_TAGS);
        place(&library, "drie.flac", testfixtures::FLAC_WITH_TAGS);

        let artist = listing(&library, "Artiest", &Filter::default())
            .expect("de artiestmap moet te tonen zijn");
        let album = artist.folders.first().expect("er moet één kaart staan");

        assert_eq!(album.name, "Album");
        assert_eq!(album.files, 3);
        assert_eq!(album.subfolders, 0);
        assert_eq!(album.formats, vec!["MP3".to_string(), "FLAC".to_string()]);
        assert!(album.has_files());
        assert_eq!(album.contents_label(), "3 bestanden");
    }

    #[test]
    fn a_card_of_a_directory_with_only_subdirectories_counts_those() {
        // Een artiestmap bevat albums en geen bestanden. "0 bestanden" zou daar
        // lezen als een lege map; er hoort te staan wat er wél is.
        let (_tempdir, library) = library_with_album();
        std::fs::create_dir(library.root().join("Artiest").join("Tweede album"))
            .expect("map moet aan te maken zijn");

        let root = listing(&library, "", &Filter::default()).expect("de root moet te tonen zijn");
        let artist = root.folders.first().expect("er moet één kaart staan");

        assert_eq!(artist.files, 0);
        assert_eq!(artist.subfolders, 2);
        assert!(!artist.has_files(), "er valt geen formaat te noemen");
        assert!(artist.formats.is_empty());
        assert_eq!(artist.contents_label(), "2 submappen");
    }

    #[test]
    fn a_card_of_an_empty_directory_says_so() {
        let (_tempdir, library) = library_with_album();
        std::fs::create_dir(library.root().join("Nog niets")).expect("map moet aan te maken zijn");

        let root = listing(&library, "", &Filter::default()).expect("de root moet te tonen zijn");
        let empty = root
            .folders
            .iter()
            .find(|folder| folder.name == "Nog niets")
            .expect("de lege map moet een kaart hebben");

        assert_eq!(empty.contents_label(), NOTHING_EDITABLE);
        assert!(!empty.has_files());
    }

    #[test]
    fn a_card_counts_one_file_in_the_singular() {
        let (_tempdir, library) = library_with_album();
        place(&library, "enige.flac", testfixtures::FLAC_WITH_TAGS);
        std::fs::create_dir(library.root().join("Artiest").join("Bonus"))
            .expect("map moet aan te maken zijn");

        let artist = listing(&library, "Artiest", &Filter::default())
            .expect("de artiestmap moet te tonen zijn");

        let album = artist
            .folders
            .iter()
            .find(|folder| folder.name == "Album")
            .expect("de albummap moet een kaart hebben");
        assert_eq!(album.contents_label(), "1 bestand");
        assert_eq!(album.formats, vec!["FLAC".to_string()]);

        let bonus = artist
            .folders
            .iter()
            .find(|folder| folder.name == "Bonus")
            .expect("de bonusmap moet een kaart hebben");
        assert_eq!(bonus.contents_label(), NOTHING_EDITABLE);
    }

    #[test]
    fn a_card_names_both_files_and_subdirectories() {
        let (_tempdir, library) = library_with_album();
        place(&library, "los.mp3", testfixtures::MP3_WITH_TAGS);
        std::fs::create_dir(
            library
                .root()
                .join("Artiest")
                .join("Album")
                .join("Schijf 2"),
        )
        .expect("map moet aan te maken zijn");

        let artist = listing(&library, "Artiest", &Filter::default())
            .expect("de artiestmap moet te tonen zijn");
        let album = artist.folders.first().expect("er moet één kaart staan");

        assert_eq!(album.contents_label(), "1 bestand · 1 submap");
    }

    #[test]
    fn a_card_only_counts_what_the_app_may_open() {
        let (_tempdir, library) = library_with_album();
        place(&library, "echt.mp3", testfixtures::MP3_WITH_TAGS);

        let album = library.root().join("Artiest").join("Album");
        std::fs::write(album.join("hoes.jpg"), b"geen audio")
            .expect("bestand moet te schrijven zijn");
        std::fs::write(album.join(".verborgen.mp3"), b"rommel")
            .expect("bestand moet te schrijven zijn");

        let outside = tempfile::tempdir().expect("tweede tempdir moet aan te maken zijn");
        std::fs::write(outside.path().join("geheim.mp3"), b"niet voor de app")
            .expect("bestand moet te schrijven zijn");
        std::os::unix::fs::symlink(outside.path().join("geheim.mp3"), album.join("geleend.mp3"))
            .expect("symlink moet aan te maken zijn");

        let artist = listing(&library, "Artiest", &Filter::default())
            .expect("de artiestmap moet te tonen zijn");
        let card = artist.folders.first().expect("er moet één kaart staan");

        assert_eq!(card.files, 1, "alleen het echte bestand telt mee");
    }

    #[test]
    fn a_library_with_many_folders_stays_quick() {
        // AC #7: de kaarten kosten één `read_dir` per map en openen geen enkel
        // bestand. Wordt dat ooit een recursieve telling of een tagleesactie,
        // dan loopt deze test als eerste vast.
        let (_tempdir, library) = library_with_album();
        for number in 1..=200 {
            let folder = library.root().join(format!("Artiest {number:03}"));
            std::fs::create_dir(&folder).expect("map moet aan te maken zijn");
            std::fs::write(folder.join("track.mp3"), b"placeholder")
                .expect("bestand moet te schrijven zijn");
        }

        let start = std::time::Instant::now();
        let root = listing(&library, "", &Filter::default()).expect("de root moet te tonen zijn");
        let elapsed = start.elapsed();

        assert_eq!(root.folders.len(), 201);
        assert!(
            elapsed < Duration::from_secs(1),
            "tweehonderd kaarten kostten {elapsed:?}"
        );
    }

    /// Bouwt een lijstregel met alleen wat de groepering ervan gebruikt.
    ///
    /// Een schijf 2 zit in geen enkele fixture, en een tweede schijf is nu
    /// juist het geval waar deze weergave voor bestaat. De sortering en de
    /// groepering werken op het tagmodel en niet op een bestand, dus kan dat
    /// hier zonder de bibliotheek aan te raken.
    fn summary(
        name: &str,
        disc: Option<u32>,
        track: Option<u32>,
        issues: Vec<TrackIssue>,
    ) -> TrackSummary {
        TrackSummary {
            name: name.to_string(),
            path: format!("Album/{name}"),
            tags: Tags {
                disc,
                track,
                ..Tags::default()
            },
            issues,
            foreign_tags: Vec::new(),
            duration: "0:00".to_string(),
            format: "MP3".to_string(),
            art: None,
            art_url: String::new(),
            edit_url: String::new(),
        }
    }

    #[test]
    fn a_set_of_two_discs_falls_apart_into_two_groups() {
        // AC #1: elke schijf zijn eigen groep, met de telling erbij.
        let mut tracks = vec![
            summary("d2-t1.mp3", Some(2), Some(1), Vec::new()),
            summary("d1-t1.mp3", Some(1), Some(1), Vec::new()),
            summary("d1-t2.mp3", Some(1), Some(2), Vec::new()),
        ];
        sort_tracks(&mut tracks);

        let groups = disc_groups(&tracks);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].disc, Some(1));
        assert_eq!(groups[0].start, 0);
        assert_eq!(groups[0].count_label(), "2 bestanden");
        assert_eq!(groups[0].label(), "Schijf 1");
        assert_eq!(groups[1].disc, Some(2));
        assert_eq!(groups[1].start, 2);
        assert_eq!(groups[1].count_label(), "1 bestand");
    }

    #[test]
    fn a_disc_number_sorts_before_the_track_number() {
        // AC #6: binnen een schijf blijft de sortering die er al was; de schijf
        // gaat ervóór, anders staat track 1 van de tweede cd tussen de eerste.
        let mut tracks = vec![
            summary("b.mp3", Some(2), Some(1), Vec::new()),
            summary("c.mp3", None, Some(1), Vec::new()),
            summary("a.mp3", Some(1), Some(2), Vec::new()),
            summary("d.mp3", Some(1), Some(1), Vec::new()),
        ];

        sort_tracks(&mut tracks);

        let order: Vec<&str> = tracks.iter().map(|track| track.name.as_str()).collect();
        assert_eq!(order, vec!["d.mp3", "a.mp3", "b.mp3", "c.mp3"]);
    }

    #[test]
    fn files_without_a_disc_number_form_the_last_group() {
        // AC #2: ze krijgen geen verzonnen nummer en staan achteraan.
        let mut tracks = vec![
            summary("los.mp3", None, Some(1), Vec::new()),
            summary("schijf.mp3", Some(1), Some(1), Vec::new()),
        ];
        sort_tracks(&mut tracks);

        let groups = disc_groups(&tracks);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].disc, Some(1));
        assert_eq!(groups[1].disc, None);
        assert_eq!(groups[1].label(), NO_DISC);
        assert_eq!(
            groups[1].key(),
            "",
            "de groep zonder schijf heeft geen nummer"
        );
    }

    #[test]
    fn a_heading_says_how_many_files_need_attention() {
        // AC #3: de telling, en wat er aandacht vraagt — of niets wanneer er
        // niets aan de hand is.
        let tracks = vec![
            summary("een.mp3", Some(1), Some(1), vec![TrackIssue::MissingTitle]),
            summary("twee.mp3", Some(1), Some(2), Vec::new()),
            summary("drie.mp3", Some(2), Some(1), Vec::new()),
        ];

        let groups = disc_groups(&tracks);

        assert_eq!(groups[0].attention, 1);
        assert_eq!(
            groups[0].attention_label().as_deref(),
            Some("1 vraagt aandacht")
        );
        assert_eq!(groups[0].describe(), "2 bestanden, 1 vraagt aandacht");

        assert_eq!(groups[1].attention, 0);
        assert_eq!(
            groups[1].attention_label(),
            None,
            "een kop hoort niet te melden dat er niets aan de hand is"
        );
        assert_eq!(groups[1].describe(), "1 bestand");
    }

    #[test]
    fn a_folder_without_disc_numbers_stays_one_list() {
        // AC #5: geen enkel discnummer, dus geen kop — de lijst ziet eruit als
        // altijd.
        let (_tempdir, library) = library_with_album();
        place(&library, "een.mp3", testfixtures::MP3_WITHOUT_TAGS);
        place(&library, "twee.flac", testfixtures::FLAC_WITHOUT_TAGS);

        let listing = album_listing(&library, "");

        assert_eq!(listing.groups.len(), 1);
        assert_eq!(listing.groups[0].disc, None);
        assert!(!listing.is_grouped());
        assert!(
            listing.group_starting_at(0).is_none(),
            "een lijst zonder schijven hoort geen kop te krijgen"
        );
    }

    #[test]
    fn a_folder_where_only_some_files_have_a_disc_number_gets_two_groups() {
        // De getagde fixture staat op schijf 1; de kale heeft geen discnummer.
        let (_tempdir, library) = library_with_album();
        place(&library, "getagd.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "kaal.mp3", testfixtures::MP3_WITHOUT_TAGS);

        let listing = album_listing(&library, "");

        assert!(listing.is_grouped());
        assert_eq!(listing.groups.len(), 2);
        assert_eq!(listing.groups[0].disc, Some(1));
        assert_eq!(listing.groups[1].disc, None);

        // De koppen staan vóór het eerste bestand van hun groep.
        assert_eq!(
            listing.group_starting_at(0).map(|group| group.label()),
            Some("Schijf 1".to_string())
        );
        assert_eq!(
            listing.group_starting_at(1).map(|group| group.label()),
            Some(NO_DISC.to_string())
        );
        assert!(listing.group_starting_at(2).is_none());
    }

    #[test]
    fn the_table_labels_show_what_is_there_and_mark_what_is_not() {
        // De tabel toont jaar, genre en disc in eigen kolommen. Wat ontbreekt
        // wordt gemarkeerd — behalve in de getalkolom, want een streepje per
        // regel is daar meer ruis dan informatie.
        let leeg = summary("kaal.mp3", None, None, Vec::new());

        assert_eq!(leeg.disc_label(), "");
        assert_eq!(leeg.year_label(), MISSING);
        assert_eq!(leeg.genre_label(), MISSING);

        let mut gevuld = summary("vol.mp3", Some(2), Some(1), Vec::new());
        gevuld.tags.year = Some("1998-04-13".to_string());
        gevuld.tags.genre = Some("Jazz".to_string());

        assert_eq!(gevuld.disc_label(), "2");
        assert_eq!(
            gevuld.year_label(),
            "1998-04-13",
            "het jaar is tekst en geen getal: een volledige datum blijft staan"
        );
        assert_eq!(gevuld.genre_label(), "Jazz");
    }

    #[test]
    fn the_summary_beside_the_name_counts_files_and_discs() {
        // Zonder discnummers staat er alleen een bestandstelling: het aantal
        // schijven melden in een map die er geen kent, is ruis.
        let (_tempdir, library) = library_with_album();
        place(&library, "een.mp3", testfixtures::MP3_WITHOUT_TAGS);
        place(&library, "twee.flac", testfixtures::FLAC_WITHOUT_TAGS);

        let listing = album_listing(&library, "");
        assert!(!listing.is_grouped());
        assert_eq!(listing.summary_label().as_deref(), Some("2 bestanden"));

        // Met een discnummer erbij komt het aantal schijven ernaast te staan.
        // De groep zonder discnummer telt daar niet in mee: van die bestanden
        // is juist niet te zeggen bij welke schijf ze horen.
        place(&library, "getagd.mp3", testfixtures::MP3_WITH_TAGS);

        let listing = album_listing(&library, "");
        assert!(listing.is_grouped());
        assert_eq!(
            listing.groups.len(),
            2,
            "de kale bestanden vormen een groep"
        );
        assert_eq!(
            listing.summary_label().as_deref(),
            Some("3 bestanden · 1 schijf")
        );

        // Een lege lijst zegt niets: daar staat al een uitleg waarom hij leeg
        // is, en "0 bestanden" voegt daar niets aan toe.
        let leeg = album_listing(&library, "bestaatniet");
        assert!(leeg.tracks.is_empty());
        assert_eq!(leeg.summary_label(), None);
    }

    #[test]
    fn the_groups_describe_the_list_that_is_shown() {
        // Filteren haalt bestanden uit de lijst; een kop die er twaalf telt
        // terwijl er één te zien is, telt iets anders dan wat eronder staat.
        let (_tempdir, library) = library_with_album();
        place(&library, "getagd.mp3", testfixtures::MP3_WITH_TAGS);
        place(&library, "kaal.mp3", testfixtures::MP3_WITHOUT_TAGS);

        let listing = album_listing(&library, "getagd");

        assert_eq!(listing.tracks.len(), 1);
        assert_eq!(listing.groups.len(), 1);
        assert_eq!(listing.groups[0].disc, Some(1));
        assert_eq!(listing.groups[0].count, 1);
    }
}
