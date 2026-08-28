//! De albumweergave: een selectie bestanden, de velden die ze delen (FR-8) en
//! wat er per bestand van afwijkt (FR-9).
//!
//! Waar [`crate::edit`] één bestand bedient, gaat het hier om een map vol
//! bestanden tegelijk. Deze module vertaalt tussen het verstuurde formulier en
//! een weergavemodel dat de templates rechtstreeks kunnen renderen, en bepaalt
//! per veld wat er met de selectie zou gebeuren. Titel en tracknummer horen bij
//! één bestand en zijn daarom in de tabel zelf in te tikken; zo'n override wint
//! van een gedeelde waarde voor datzelfde bestand.
//!
//! De hulpacties uit FR-10 horen hier ook: hernummeren, artiest → albumartiest
//! en hoofdletters normaliseren vullen invoervelden van datzelfde formulier en
//! doen verder niets.
//!
//! Er wordt hier niets geschreven en er gaat geen bestand open: in en uit gaan
//! een [`Listing`] en een [`Form`]. Het daadwerkelijk wegschrijven hoort bij de
//! diff-preview, zodat een batch-wijziging nooit zonder voorbeeld plaatsvindt.

use std::collections::{BTreeMap, BTreeSet};

use percent_encoding::percent_decode_str;

use crate::browse::{self, Crumb, Listing, TrackSummary};
use crate::casing;
use crate::edit;
use crate::tags::Tags;

/// Wat er in een invoerveld staat waar de selectie niets te melden heeft.
const EMPTY: &str = "—";

/// Een veld dat een heel album deelt (PRD FR-8).
///
/// Titel en tracknummer horen hier bewust níét bij: die verschillen per
/// bestand en krijgen hun eigen kolom in de tabel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedField {
    AlbumArtist,
    Album,
    Year,
    Genre,
    Disc,
}

impl SharedField {
    /// Alle gedeelde velden, in de volgorde waarin ze op het scherm staan.
    pub const ALL: [SharedField; 5] = [
        SharedField::AlbumArtist,
        SharedField::Album,
        SharedField::Year,
        SharedField::Genre,
        SharedField::Disc,
    ];

    /// De naam waaronder het veld in het formulier staat.
    pub fn name(self) -> &'static str {
        match self {
            SharedField::AlbumArtist => "album_artist",
            SharedField::Album => "album",
            SharedField::Year => "year",
            SharedField::Genre => "genre",
            SharedField::Disc => "disc",
        }
    }

    /// De naam van het bijbehorende wissen-vinkje.
    pub fn clear_name(self) -> String {
        format!("wis_{}", self.name())
    }

    /// Het opschrift boven het invoerveld.
    pub fn label(self) -> &'static str {
        match self {
            SharedField::AlbumArtist => "Albumartiest",
            SharedField::Album => "Album",
            SharedField::Year => "Jaar",
            SharedField::Genre => "Genre",
            SharedField::Disc => "Discnummer",
        }
    }

    /// Of er een getal in hoort; bepaalt de controle en het toetsenbord.
    pub fn is_numeric(self) -> bool {
        matches!(self, SharedField::Disc)
    }

    /// Wat er voor dit veld in één bestand staat.
    ///
    /// Alles komt hier als tekst terug, ook het discnummer: de invoer is tekst
    /// en het vergelijken van de selectie gebeurt op wat er te zien is.
    pub fn value_of(self, tags: &Tags) -> Option<String> {
        match self {
            SharedField::AlbumArtist => tags.album_artist.clone(),
            SharedField::Album => tags.album.clone(),
            SharedField::Year => tags.year.clone(),
            SharedField::Genre => tags.genre.clone(),
            SharedField::Disc => tags.disc.map(|number| number.to_string()),
        }
    }

    /// De plek van dit veld in de vaste arrays van [`Form`].
    fn index(self) -> usize {
        match self {
            SharedField::AlbumArtist => 0,
            SharedField::Album => 1,
            SharedField::Year => 2,
            SharedField::Genre => 3,
            SharedField::Disc => 4,
        }
    }
}

/// Een veld dat per bestand verschilt en daarom in de tabel zelf staat (FR-9).
///
/// De tegenhanger van [`SharedField`]: waar dat veld één waarde voor de hele
/// selectie zet, hoort hier per rij iets anders te kunnen staan.
/// Albumartiest staat in beide lijstjes, en dat is geen vergissing: hij is
/// meestal voor het hele album gelijk, maar de hulpactie "artiest →
/// albumartiest" (FR-10) zet er per bestand een eigen waarde in. Waar ze elkaar
/// raken wint de rij; [`intents`] legt die volgorde vast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowField {
    Track,
    Title,
    AlbumArtist,
}

impl RowField {
    /// Alle drie de velden, in de volgorde waarin ze in de tabel staan.
    pub const ALL: [RowField; 3] = [RowField::Track, RowField::Title, RowField::AlbumArtist];

    /// De naam van het veld in het tagmodel; ook de sleutel in een [`FileIntent`].
    pub fn field_name(self) -> &'static str {
        match self {
            RowField::Track => "track",
            RowField::Title => "title",
            RowField::AlbumArtist => "album_artist",
        }
    }

    /// De naam waaronder de invoer voor één bestand in het formulier staat.
    ///
    /// De bestandsnaam hoort erin: er staat één rij per bestand, en de browser
    /// verstuurt alle rijen tegelijk. Splitsen gebeurt op de eerste dubbele
    /// punt, zodat een bestandsnaam er zelf ook een mag bevatten.
    pub fn input_name(self, file: &str) -> String {
        format!("{}:{file}", self.prefix())
    }

    /// Het voorvoegsel van de formuliersleutel.
    fn prefix(self) -> &'static str {
        match self {
            RowField::Track => "nummer",
            RowField::Title => "titel",
            RowField::AlbumArtist => "albumartiest",
        }
    }

    /// Het opschrift boven de kolom.
    pub fn label(self) -> &'static str {
        match self {
            RowField::Track => "Tracknummer",
            RowField::Title => "Titel",
            RowField::AlbumArtist => "Albumartiest",
        }
    }

    /// Of er een getal in hoort; bepaalt de controle en het toetsenbord.
    pub fn is_numeric(self) -> bool {
        matches!(self, RowField::Track)
    }

    /// Wat er voor dit veld in één bestand staat.
    fn value_of(self, tags: &Tags) -> Option<String> {
        match self {
            RowField::Track => tags.track.map(|number| number.to_string()),
            RowField::Title => tags.title.clone(),
            RowField::AlbumArtist => tags.album_artist.clone(),
        }
    }

    /// De plek van dit veld in de vaste arrays van [`Override`].
    fn index(self) -> usize {
        match self {
            RowField::Track => 0,
            RowField::Title => 1,
            RowField::AlbumArtist => 2,
        }
    }
}

/// Wat er voor één bestand in de tabel is ingetikt.
///
/// Leeg betekent hier hetzelfde als bij een gedeeld veld: ongemoeid laten. Er
/// wordt dus niets voorgevuld; wat er nú in het bestand staat, staat als grijze
/// tekst in het veld.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Override {
    values: [String; 3],
}

/// Wat er met de selectie moet gebeuren voordat de pagina wordt opgebouwd.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Action {
    /// De verstuurde selectie ongewijzigd overnemen.
    #[default]
    Keep,

    /// Alles in deze map selecteren.
    All,

    /// De selectie leegmaken.
    None,

    /// De selectie opeenvolgend nummeren, in de volgorde van de tabel.
    Renumber,

    /// Per bestand de artiest als albumartiest voorstellen.
    CopyArtist,

    /// Het hoofdlettergebruik van de tekstvelden normaliseren.
    Capitalize,

    /// Alle ingevulde velden weer leegmaken.
    Reset,
}

impl Action {
    fn parse(raw: &str) -> Action {
        match raw {
            "alles" => Action::All,
            "niets" => Action::None,
            "hernummer" => Action::Renumber,
            "artiest" => Action::CopyArtist,
            "hoofdletters" => Action::Capitalize,
            "herstel" => Action::Reset,
            _ => Action::Keep,
        }
    }
}

/// Wat er bij het opslaan met één gedeeld veld zou gebeuren.
///
/// Dit is het aangrijpingspunt van de diff-preview: die vertaalt deze
/// voornemens naar een wijziging per bestand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// Het veld is leeg gelaten; de bestanden houden wat ze hebben.
    Unchanged,

    /// Voor de hele selectie deze waarde.
    Set(String),

    /// De tag uit de hele selectie verwijderen.
    Clear,
}

impl Intent {
    /// Wat er onder het formulier over dit voornemen komt te staan.
    fn describe(&self, label: &str, selected: usize) -> String {
        let files = if selected == 1 {
            "1 bestand".to_string()
        } else {
            format!("{selected} bestanden")
        };

        match self {
            Intent::Unchanged => format!("{label} blijft ongemoeid."),
            Intent::Set(value) => format!("{label} wordt “{value}” in {files}."),
            Intent::Clear => format!("{label} wordt verwijderd uit {files}."),
        }
    }
}

/// De toestand van het formulier zoals de browser hem verstuurt.
///
/// De selectie staat als herhaalde `bestand`-sleutel in de body. Dat is
/// precies wat `serde_urlencoded` — de basis onder `axum::Form` — niet naar een
/// `Vec` kan deserialiseren, en daarom leest [`Form::parse`] de body zelf.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Form {
    /// Wat er met de selectie moet gebeuren voordat de pagina wordt gebouwd.
    pub action: Action,

    /// De bestandsnamen die aangevinkt stonden.
    ///
    /// Namen en geen paden: een albumweergave gaat over één map, en daarbinnen
    /// is een naam uniek. Wat er niet in de map (meer) staat, valt bij het
    /// opbouwen vanzelf af.
    pub selected: BTreeSet<String>,

    /// De ingetikte waarde per gedeeld veld, in de volgorde van
    /// [`SharedField::ALL`].
    values: [String; 5],

    /// Of het wissen-vinkje aan stond, in dezelfde volgorde.
    clear: [bool; 5],

    /// De ingetikte overrides, per bestandsnaam (FR-9).
    ///
    /// Een bestand dat niets gekregen heeft, staat er niet in; wat er niet
    /// (meer) in de map staat, valt bij het opbouwen vanzelf af.
    overrides: BTreeMap<String, Override>,
}

impl Form {
    /// Leest een `application/x-www-form-urlencoded`-body.
    pub fn parse(body: &str) -> Form {
        let mut form = Form::default();

        for pair in body.split('&').filter(|pair| !pair.is_empty()) {
            let (key, raw) = match pair.split_once('=') {
                Some((key, value)) => (decode(key), decode(value)),
                None => (decode(pair), String::new()),
            };

            match key.as_str() {
                "actie" => form.action = Action::parse(&raw),
                "bestand" => {
                    form.selected.insert(raw);
                }
                _ => {
                    if let Some((field, file)) = row_key(&key) {
                        form.overrides.entry(file).or_default().values[field.index()] = raw;
                        continue;
                    }

                    for field in SharedField::ALL {
                        if key == field.name() {
                            form.values[field.index()] = raw;
                            break;
                        }
                        if key == field.clear_name() {
                            // Een vinkje staat alleen in de body wanneer het
                            // aan staat; de waarde erachter doet er niet toe.
                            form.clear[field.index()] = true;
                            break;
                        }
                    }
                }
            }
        }

        form
    }

    /// Het formulier zoals de albumpagina opent: alles geselecteerd, niets
    /// ingevuld.
    ///
    /// Een album corrigeren begint vrijwel altijd bij het hele album; wie er
    /// een paar bestanden uit wil halen, klikt die uit.
    pub fn select_all() -> Form {
        Form {
            action: Action::All,
            ..Form::default()
        }
    }

    /// Wat de gebruiker in dit veld heeft ingetikt.
    pub fn value(&self, field: SharedField) -> &str {
        &self.values[field.index()]
    }

    /// Of dit veld voor de hele selectie gewist moet worden.
    pub fn is_cleared(&self, field: SharedField) -> bool {
        self.clear[field.index()]
    }

    /// Wat er bij het opslaan met dit veld zou gebeuren.
    ///
    /// Een leeg veld betekent hier "ongemoeid laten" en niet "verwijderen".
    /// Dat is het omgekeerde van het bewerkformulier van één bestand, en met
    /// reden: daar staat in het veld wat er in het bestand staat, hier staat er
    /// niets voorgevuld. Wissen is daarom een aparte, expliciete keuze.
    ///
    /// Het wissen-vinkje wint van een ingetikte waarde: wie beide aanzet, heeft
    /// zichzelf tegengesproken, en dan is niets-schrijven de veilige uitkomst
    /// van de twee.
    pub fn intent(&self, field: SharedField) -> Result<Intent, String> {
        if self.is_cleared(field) {
            return Ok(Intent::Clear);
        }

        let value = self.value(field).trim();
        if value.is_empty() {
            return Ok(Intent::Unchanged);
        }

        if field.is_numeric() {
            // Controleren vóór er iets naar een bestand gaat, met dezelfde
            // melding als het bewerkformulier van één bestand.
            edit::parse_number(value, field.label())?;
        }

        Ok(Intent::Set(value.to_string()))
    }

    /// Wat de gebruiker in de tabel voor dit bestand heeft ingetikt.
    pub fn override_value(&self, file: &str, field: RowField) -> &str {
        self.overrides
            .get(file)
            .map(|entry| entry.values[field.index()].as_str())
            .unwrap_or_default()
    }

    /// Wat er bij het opslaan met dit veld van dit ene bestand zou gebeuren.
    ///
    /// Dezelfde regel als bij een gedeeld veld: leeg laat het bestand houden
    /// wat het heeft. Wissen kan hier niet — titel en tracknummer weghalen is
    /// geen batch-actie, en dat hoort in het bewerkformulier van het bestand
    /// zelf te gebeuren.
    pub fn row_intent(&self, file: &str, field: RowField) -> Result<Intent, String> {
        let value = self.override_value(file, field).trim();
        if value.is_empty() {
            return Ok(Intent::Unchanged);
        }

        if field.is_numeric() {
            edit::parse_number(value, field.label())?;
        }

        Ok(Intent::Set(value.to_string()))
    }

    /// Zet een voorstel in een invoerveld van de tabel.
    ///
    /// Een lege waarde haalt het voorstel weer weg; dat is wat een hulpactie
    /// doet met een veld waar niets aan te verbeteren valt.
    fn set_override(&mut self, file: &str, field: RowField, value: String) {
        self.overrides.entry(file.to_string()).or_default().values[field.index()] = value;
    }

    /// Het formulier zoals het is ná de aangeklikte hulpactie (FR-10).
    ///
    /// Een hulpactie vult uitsluitend invoervelden. Er gaat geen bestand open
    /// en er wordt niets geschreven: wat de actie voorstelt staat daarna gewoon
    /// in de velden, is met de hand aan te passen, en gaat met "Invoer
    /// leegmaken" in één klik weer weg.
    ///
    /// De zin die terugkomt vertelt wat de actie gedaan heeft; zonder
    /// hulpactie is er niets te melden.
    fn applied(&self, chosen: &[&TrackSummary]) -> (Form, Option<String>) {
        let mut form = self.clone();

        let notice = match self.action {
            Action::Renumber => Some(form.renumber(chosen)),
            Action::CopyArtist => Some(form.copy_artist(chosen)),
            Action::Capitalize => Some(form.capitalize(chosen)),
            Action::Reset => {
                // De selectie is geen invoer en blijft dus staan; alleen wat er
                // ingetikt of voorgesteld is, gaat weg (AC #5).
                form = Form {
                    selected: self.selected.clone(),
                    ..Form::default()
                };
                Some("De ingevulde velden zijn leeggemaakt; de selectie staat nog.".to_string())
            }
            Action::Keep | Action::All | Action::None => None,
        };

        (form, notice)
    }

    /// Nummert de selectie opeenvolgend in de volgorde van de tabel.
    ///
    /// De volgorde is die van de listing en niet die van de bestaande
    /// tracknummers: juist wanneer die nummers niet kloppen, is deze actie
    /// nodig.
    fn renumber(&mut self, chosen: &[&TrackSummary]) -> String {
        for (position, track) in chosen.iter().enumerate() {
            self.set_override(&track.name, RowField::Track, (position + 1).to_string());
        }

        match chosen.len() {
            0 => "Er is niets geselecteerd om te hernummeren.".to_string(),
            count => format!(
                "De selectie is genummerd van 1 tot en met {count}; de nummers staan als voorstel in de tabel."
            ),
        }
    }

    /// Zet per bestand de artiest als albumartiest klaar.
    ///
    /// Per bestand, want de artiesten hoeven niet gelijk te zijn; de rij wint
    /// daarom van het gedeelde veld.
    fn copy_artist(&mut self, chosen: &[&TrackSummary]) -> String {
        let mut copied = 0;
        let mut skipped = 0;

        for track in chosen {
            match &track.tags.artist {
                Some(artist) => {
                    self.set_override(&track.name, RowField::AlbumArtist, artist.clone());
                    copied += 1;
                }
                // Zonder artiest valt er niets te kopiëren, en een lege
                // albumartiest voorstellen zou een verwijdering zijn.
                None => skipped += 1,
            }
        }

        let mut notice = match copied {
            0 => "Geen enkel geselecteerd bestand heeft een artiest om te kopiëren.".to_string(),
            1 => "Bij 1 bestand staat de artiest nu als albumartiest in de tabel.".to_string(),
            count => {
                format!("Bij {count} bestanden staat de artiest nu als albumartiest in de tabel.")
            }
        };

        if copied > 0 && skipped > 0 {
            notice.push_str(&format!(" {skipped} zonder artiest zijn overgeslagen."));
        }

        notice
    }

    /// Normaliseert het hoofdlettergebruik van de tekstvelden (FR-10).
    ///
    /// Titel en albumartiest gaan per bestand; album en genre alleen wanneer de
    /// hele selectie er dezelfde waarde heeft, want één gedeeld veld kan geen
    /// twee verschillende voorstellen bevatten.
    ///
    /// Er wordt genormaliseerd over wat er al in het veld staat wanneer de
    /// gebruiker er zelf iets heeft ingetikt, en anders over wat er in het
    /// bestand staat. Levert dat niets nieuws op, dan blijft het veld leeg:
    /// een voorstel dat gelijk is aan de bestaande waarde is geen voorstel.
    fn capitalize(&mut self, chosen: &[&TrackSummary]) -> String {
        let mut proposals = 0;

        for track in chosen {
            for field in [RowField::Title, RowField::AlbumArtist] {
                let current = field.value_of(&track.tags).unwrap_or_default();
                let typed = self.override_value(&track.name, field).trim().to_string();
                let source = if typed.is_empty() { &current } else { &typed };

                let proposal = casing::normalize(source);
                let keep = proposal != current && !proposal.is_empty();

                self.set_override(
                    &track.name,
                    field,
                    if keep { proposal } else { String::new() },
                );
                proposals += usize::from(keep);
            }
        }

        for field in [SharedField::Album, SharedField::Genre] {
            let Current::Same(current) = Current::of(field, chosen) else {
                continue;
            };

            let typed = self.value(field).trim().to_string();
            let source = if typed.is_empty() { &current } else { &typed };

            let proposal = casing::normalize(source);
            let keep = proposal != current && !proposal.is_empty();

            self.values[field.index()] = if keep { proposal } else { String::new() };
            proposals += usize::from(keep);
        }

        match proposals {
            0 => "Aan het hoofdlettergebruik van de selectie valt niets te verbeteren.".to_string(),
            1 => "Eén veld heeft een voorstel gekregen; controleer het en sla het pas op als het klopt.".to_string(),
            count => format!(
                "{count} velden hebben een voorstel gekregen; controleer ze en sla ze pas op als ze kloppen."
            ),
        }
    }
}

/// Leest de sleutel van een override: welk veld, en van welk bestand.
fn row_key(key: &str) -> Option<(RowField, String)> {
    let (prefix, file) = key.split_once(':')?;

    RowField::ALL
        .into_iter()
        .find(|field| field.prefix() == prefix)
        .map(|field| (field, file.to_string()))
}

/// Wat er nú in de selectie staat voor één gedeeld veld.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Current {
    /// Er is niets geselecteerd, dus er valt niets over te zeggen.
    Nothing,

    /// Geen van de geselecteerde bestanden heeft deze tag.
    Empty,

    /// Alle geselecteerde bestanden hebben dezelfde waarde.
    Same(String),

    /// De geselecteerde bestanden lopen uiteen; dit zijn de waarden die
    /// voorkomen, inclusief het ontbreken ervan.
    Different(Vec<String>),
}

impl Current {
    /// Bepaalt de huidige stand over een selectie.
    fn of(field: SharedField, tracks: &[&TrackSummary]) -> Current {
        if tracks.is_empty() {
            return Current::Nothing;
        }

        let mut values: Vec<String> = Vec::new();
        for track in tracks {
            let value = field
                .value_of(&track.tags)
                .unwrap_or_else(|| EMPTY.to_string());
            if !values.contains(&value) {
                values.push(value);
            }
        }

        match values.as_slice() {
            [only] if only == EMPTY => Current::Empty,
            [only] => Current::Same(only.clone()),
            _ => Current::Different(values),
        }
    }

    /// Of de selectie voor dit veld uiteenloopt.
    pub fn is_different(&self) -> bool {
        matches!(self, Current::Different(_))
    }

    /// Wat er naast het invoerveld komt te staan.
    pub fn describe(&self) -> String {
        match self {
            Current::Nothing => "Er is niets geselecteerd.".to_string(),
            Current::Empty => "Nu: leeg in de hele selectie.".to_string(),
            Current::Same(value) => format!("Nu: “{value}” in de hele selectie."),
            Current::Different(values) => {
                let quoted: Vec<String> = values
                    .iter()
                    .map(|value| {
                        if value == EMPTY {
                            "leeg".to_string()
                        } else {
                            format!("“{value}”")
                        }
                    })
                    .collect();

                format!("Nu: verschillend ({}).", quoted.join(", "))
            }
        }
    }

    /// De grijze tekst in het lege invoerveld.
    ///
    /// Herhaalt bewust niet de hele stand: het veld is smal, en de volledige
    /// tekst staat er onder. Wat hier moet staan is wat er gebeurt als je niets
    /// doet.
    pub fn placeholder(&self) -> &'static str {
        match self {
            Current::Different(_) => "Verschillend — leeg laten behoudt per bestand",
            _ => "Leeg laten verandert niets",
        }
    }
}

/// Eén gedeeld veld zoals het op het scherm staat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedInput {
    /// De naam in het formulier.
    pub name: String,

    /// De naam van het wissen-vinkje.
    pub clear_name: String,

    pub label: String,

    /// Wat de gebruiker heeft ingetikt; leeg bij het openen van de pagina.
    ///
    /// Wordt nooit voorgevuld met de huidige waarde. Daardoor betekent leeg
    /// altijd hetzelfde, en dat is precies wat een batch-actie begrijpelijk
    /// houdt.
    pub value: String,

    /// Of het wissen-vinkje aan staat.
    pub cleared: bool,

    /// Of er een getal in hoort.
    pub numeric: bool,

    /// Wat er nu in de selectie staat.
    pub current: Current,

    /// Wat er bij het opslaan gebeurt, in één zin.
    pub effect: String,
}

/// Eén regel in de albumtabel.
///
/// De waarden zijn hier al tekst: de tabel toont ze rechtstreeks, en een
/// ontbrekende tag hoort als streepje zichtbaar te zijn en niet als lege cel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Bestandsnaam; ook de waarde van het selectievinkje.
    pub name: String,

    /// Of dit bestand geselecteerd is.
    pub selected: bool,

    pub track: String,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub year: String,
    pub genre: String,
    pub disc: String,

    /// Naar het bewerkformulier van dit ene bestand.
    pub edit_url: String,

    /// De naam van het tracknummerveld van deze rij in het formulier.
    pub track_name: String,

    /// De naam van het titelveld van deze rij in het formulier.
    pub title_name: String,

    /// De naam van het albumartiestveld van deze rij in het formulier.
    pub album_artist_name: String,

    /// Wat er voor deze rij is ingetikt; leeg bij het openen van de pagina.
    pub track_input: String,
    pub title_input: String,
    pub album_artist_input: String,

    /// Wat er aan de invoer van déze rij mankeert.
    ///
    /// Een fout hier blijft bij deze rij: de andere rijen en de gedeelde velden
    /// blijven gewoon opgeslagen kunnen worden.
    pub problems: Vec<String>,
}

impl Row {
    /// Of er iets in deze rij is ingetikt.
    pub fn is_overridden(&self) -> bool {
        [
            &self.track_input,
            &self.title_input,
            &self.album_artist_input,
        ]
        .iter()
        .any(|value| !value.trim().is_empty())
    }

    /// Of deze rij zo niet opgeslagen kan worden.
    pub fn has_problems(&self) -> bool {
        !self.problems.is_empty()
    }

    /// Of hier iets is ingetikt dat niet wordt opgeslagen omdat de rij niet
    /// geselecteerd staat.
    ///
    /// De invoer blijft staan — het aanvinken maakt haar zo weer geldig — maar
    /// dat ze nu niets doet, hoort zichtbaar te zijn.
    pub fn is_ignored(&self) -> bool {
        !self.selected && self.is_overridden()
    }
}

/// Wat er bij het opslaan met één bestand zou gebeuren.
///
/// De gedeelde velden gelden voor de hele selectie, de overrides voor dit ene
/// bestand. Waar ze elkaar raken wint de override: wie in de rij zelf iets
/// intikt, bedoelt dat voor dat bestand en niet voor het album. Die regel staat
/// hier, in [`intents`], en niet bij de aanroeper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIntent {
    /// Bestandsnaam binnen de map.
    pub name: String,

    /// Per veld uit het tagmodel wat ermee gebeurt. Wat ongemoeid blijft, staat
    /// er niet in.
    pub fields: BTreeMap<&'static str, Intent>,
}

/// Wat er bij het opslaan per bestand zou gebeuren (FR-9, vooruit naar FR-11).
///
/// Alleen geselecteerde bestanden doen mee, en alleen bestanden waarvan de
/// invoer klopt: een fout in één rij houdt de rest niet tegen. Bestanden
/// waaraan niets verandert, blijven weg uit de uitkomst.
pub fn intents(listing: &Listing, form: &Form) -> Vec<FileIntent> {
    let selected = resolve_selection(listing, form);

    // Een hulpactie vult velden, en die gevulde velden horen bij het plan; de
    // pagina en het plan komen zo van hetzelfde formulier.
    let (form, _) = form.applied(&chosen_tracks(listing, &selected));

    let shared: BTreeMap<&'static str, Intent> = SharedField::ALL
        .into_iter()
        .filter_map(|field| match form.intent(field) {
            Ok(Intent::Set(value)) => Some((field.name(), Intent::Set(value))),
            Ok(Intent::Clear) => Some((field.name(), Intent::Clear)),
            // Ongemoeid, of onleesbaar: dan gebeurt er niets mee. Wat er aan de
            // invoer mankeert, staat op de pagina en houdt het opslaan tegen.
            Ok(Intent::Unchanged) | Err(_) => None,
        })
        .collect();

    listing
        .tracks
        .iter()
        .filter(|track| selected.contains(&track.name))
        .filter_map(|track| {
            let mut fields = shared.clone();

            for field in RowField::ALL {
                match form.row_intent(&track.name, field) {
                    Ok(Intent::Unchanged) => {}
                    // De override overschrijft wat de gedeelde velden voor dit
                    // bestand hadden bedacht.
                    Ok(intent) => {
                        fields.insert(field.field_name(), intent);
                    }
                    // Een onleesbare invoer laat dit bestand vallen; de rij
                    // meldt zelf wat eraan mankeert.
                    Err(_) => return None,
                }
            }

            if fields.is_empty() {
                return None;
            }

            Some(FileIntent {
                name: track.name.clone(),
                fields,
            })
        })
        .collect()
}

/// Alles wat de albumpagina nodig heeft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumPage {
    /// Naam van de map.
    pub name: String,

    /// Tot en met deze map.
    pub crumbs: Vec<Crumb>,

    /// Waar het formulier naartoe post; ook de URL van deze pagina.
    pub url: String,

    /// Terug naar de gewone mapweergave.
    pub back_url: String,

    pub rows: Vec<Row>,

    /// De vijf gedeelde velden, in vaste volgorde.
    pub fields: Vec<SharedInput>,

    /// Hoeveel bestanden er geselecteerd zijn.
    pub selected: usize,

    /// Hoeveel bestanden de map bevat.
    pub total: usize,

    /// Wat er aan de invoer van de gedeelde velden mankeert; leeg wanneer alles
    /// klopt. Wat er aan een rij mankeert, staat bij die rij.
    pub problems: Vec<String>,

    /// Hoeveel geselecteerde rijen een eigen titel of tracknummer gekregen
    /// hebben.
    pub overridden: usize,

    /// Hoeveel bestanden er bij het opslaan daadwerkelijk zouden veranderen.
    ///
    /// Komt uit [`intents`], zodat het getal onder het formulier van dezelfde
    /// berekening komt als het plan dat de voorbeeldweergave straks toont.
    pub changed_files: usize,

    /// Wat de zojuist aangeklikte hulpactie gedaan heeft (FR-10); leeg wanneer
    /// er geen hulpactie is gebruikt.
    pub helper_notice: Option<String>,
}

impl AlbumPage {
    /// Of er iets geselecteerd is.
    pub fn has_selection(&self) -> bool {
        self.selected > 0
    }

    /// Of er ook maar één veld iets zou doen.
    pub fn changes_anything(&self) -> bool {
        self.overridden > 0
            || self
                .fields
                .iter()
                .any(|field| field.cleared || !field.value.trim().is_empty())
    }

    /// Of er ergens in de tabel een onbruikbare invoer staat.
    pub fn has_row_problems(&self) -> bool {
        self.rows.iter().any(Row::has_problems)
    }

    /// Hoeveel bestanden er zouden veranderen, in één zin.
    pub fn changed_files_effect(&self) -> String {
        match self.changed_files {
            0 => "Er verandert geen enkel bestand.".to_string(),
            1 => "In totaal verandert er 1 bestand.".to_string(),
            count => format!("In totaal veranderen er {count} bestanden."),
        }
    }

    /// Wat er bij het opslaan met de per-bestand overrides gebeurt, in één zin.
    pub fn overrides_effect(&self) -> String {
        match self.overridden {
            0 => "Wat er per bestand verschilt, blijft ongemoeid.".to_string(),
            1 => "1 bestand krijgt een eigen waarde uit de tabel.".to_string(),
            count => format!("{count} bestanden krijgen een eigen waarde uit de tabel."),
        }
    }
}

/// Bouwt het weergavemodel van de albumpagina.
///
/// `listing` levert de bestanden en hun tags; `form` bepaalt wat er
/// geselecteerd en ingevuld is. Er wordt hier geen bestand geopend: de tags
/// zitten al in de listing.
pub fn album(listing: &Listing, form: &Form) -> AlbumPage {
    let selected = resolve_selection(listing, form);

    // De signalering, de sortering en de tags komen allemaal uit de listing;
    // hier wordt alleen nog gekozen waar de gedeelde velden naar kijken.
    let chosen = chosen_tracks(listing, &selected);

    // Een hulpactie vult invoervelden en verandert verder niets: vanaf hier
    // wordt de pagina met het aangevulde formulier opgebouwd (FR-10).
    let (form, helper_notice) = form.applied(&chosen);
    let form = &form;

    let rows: Vec<Row> = listing
        .tracks
        .iter()
        .map(|track| Row {
            selected: selected.contains(&track.name),
            track_name: RowField::Track.input_name(&track.name),
            title_name: RowField::Title.input_name(&track.name),
            track_input: form
                .override_value(&track.name, RowField::Track)
                .to_string(),
            title_input: form
                .override_value(&track.name, RowField::Title)
                .to_string(),
            album_artist_name: RowField::AlbumArtist.input_name(&track.name),
            album_artist_input: form
                .override_value(&track.name, RowField::AlbumArtist)
                .to_string(),
            problems: RowField::ALL
                .into_iter()
                .filter_map(|field| form.row_intent(&track.name, field).err())
                .collect(),
            name: track.name.clone(),
            track: track.track_label(),
            title: track.title_label().to_string(),
            artist: track.artist_label().to_string(),
            album_artist: cell(&track.tags.album_artist),
            album: cell(&track.tags.album),
            year: cell(&track.tags.year),
            genre: cell(&track.tags.genre),
            disc: track
                .tags
                .disc
                .map(|number| number.to_string())
                .unwrap_or_else(|| EMPTY.to_string()),
            edit_url: track.edit_url.clone(),
        })
        .collect();

    let mut problems = Vec::new();
    let fields: Vec<SharedInput> = SharedField::ALL
        .into_iter()
        .map(|field| {
            let effect = match form.intent(field) {
                Ok(intent) => intent.describe(field.label(), chosen.len()),
                Err(problem) => {
                    problems.push(problem);
                    format!("{} kan zo niet opgeslagen worden.", field.label())
                }
            };

            SharedInput {
                name: field.name().to_string(),
                clear_name: field.clear_name(),
                label: field.label().to_string(),
                value: form.value(field).to_string(),
                cleared: form.is_cleared(field),
                numeric: field.is_numeric(),
                current: Current::of(field, &chosen),
                effect,
            }
        })
        .collect();

    let overridden = rows
        .iter()
        .filter(|row| row.selected && row.is_overridden() && !row.has_problems())
        .count();

    AlbumPage {
        name: listing.name.clone(),
        crumbs: listing.crumbs.clone(),
        url: browse::album_url(&listing.path),
        back_url: listing.url.clone(),
        selected: chosen.len(),
        total: rows.len(),
        rows,
        fields,
        problems,
        overridden,
        changed_files: intents(listing, form).len(),
        helper_notice,
    }
}

/// Welke bestanden er geselecteerd zijn, na het toepassen van de actie.
///
/// Een naam die niet (meer) in de map staat, verdwijnt vanzelf: er wordt alleen
/// tegen de bestanden uit de listing aan gekeken.
fn resolve_selection(listing: &Listing, form: &Form) -> BTreeSet<String> {
    match form.action {
        Action::All => listing
            .tracks
            .iter()
            .map(|track| track.name.clone())
            .collect(),
        Action::None => BTreeSet::new(),
        // Een hulpactie laat de selectie met rust: die vult alleen velden.
        _ => form.selected.clone(),
    }
}

/// De geselecteerde bestanden, in de volgorde van de tabel.
fn chosen_tracks<'a>(listing: &'a Listing, selected: &BTreeSet<String>) -> Vec<&'a TrackSummary> {
    listing
        .tracks
        .iter()
        .filter(|track| selected.contains(&track.name))
        .collect()
}

/// Een tagwaarde als tabelcel, met een streepje waar niets staat.
fn cell(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| EMPTY.to_string())
}

/// Decodeert één sleutel of waarde uit een urlencoded body.
fn decode(raw: &str) -> String {
    // In een formulierbody staat een spatie als `+`; percent-decoding kent die
    // regel niet, dus die gaat er eerst uit.
    let spaced = raw.replace('+', " ");
    percent_decode_str(&spaced).decode_utf8_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_body_selects_nothing_and_fills_nothing() {
        let form = Form::parse("");

        assert_eq!(form.action, Action::Keep);
        assert!(form.selected.is_empty());
        for field in SharedField::ALL {
            assert_eq!(form.value(field), "");
            assert!(!form.is_cleared(field));
            assert_eq!(form.intent(field), Ok(Intent::Unchanged));
        }
    }

    #[test]
    fn the_selection_is_read_from_repeated_keys() {
        // Precies wat `serde_urlencoded` niet kan, en de reden dat deze parser
        // bestaat.
        let form = Form::parse("bestand=een.mp3&bestand=twee.mp3&bestand=een.mp3");

        assert_eq!(
            form.selected,
            BTreeSet::from(["een.mp3".to_string(), "twee.mp3".to_string()])
        );
    }

    #[test]
    fn spaces_and_accents_survive_the_body() {
        let form = Form::parse("bestand=een+twee.mp3&album=Caf%C3%A9+de+Nuit");

        assert!(form.selected.contains("een twee.mp3"));
        assert_eq!(form.value(SharedField::Album), "Café de Nuit");
    }

    #[test]
    fn a_checked_box_is_recognised_by_its_name() {
        let form = Form::parse("wis_genre=aan");

        assert!(form.is_cleared(SharedField::Genre));
        assert!(!form.is_cleared(SharedField::Album));
    }

    #[test]
    fn an_unknown_key_is_ignored() {
        let form = Form::parse("onzin=1&album=Wel+dit");

        assert_eq!(form.value(SharedField::Album), "Wel dit");
    }

    #[test]
    fn a_filled_field_becomes_a_value_and_an_empty_one_changes_nothing() {
        let form = Form::parse("album=Nieuw+album&genre=+++");

        assert_eq!(
            form.intent(SharedField::Album),
            Ok(Intent::Set("Nieuw album".to_string()))
        );
        // Alleen spaties telt als leeg, net als in het bewerkformulier.
        assert_eq!(form.intent(SharedField::Genre), Ok(Intent::Unchanged));
    }

    #[test]
    fn clearing_is_a_separate_choice_and_wins_from_a_typed_value() {
        let form = Form::parse("album=Nieuw&wis_album=aan");

        assert_eq!(form.intent(SharedField::Album), Ok(Intent::Clear));
    }

    #[test]
    fn a_disc_number_that_is_not_a_number_is_refused() {
        let form = Form::parse("disc=twee");

        let problem = form
            .intent(SharedField::Disc)
            .expect_err("dit hoort een fout te zijn");

        assert!(problem.starts_with("Discnummer"), "{problem}");
        assert!(problem.contains("twee"), "{problem}");
    }

    #[test]
    fn every_intent_says_what_it_would_do() {
        assert_eq!(
            Intent::Unchanged.describe("Album", 3),
            "Album blijft ongemoeid."
        );
        assert_eq!(
            Intent::Set("Nieuw".to_string()).describe("Album", 3),
            "Album wordt “Nieuw” in 3 bestanden."
        );
        assert_eq!(
            Intent::Clear.describe("Album", 1),
            "Album wordt verwijderd uit 1 bestand."
        );
    }

    /// Bouwt een track met alleen de velden die deze tests gebruiken.
    fn track(name: &str, album: Option<&str>, disc: Option<u32>) -> TrackSummary {
        track_with(
            name,
            Tags {
                album: album.map(str::to_string),
                disc,
                ..Tags::default()
            },
        )
    }

    /// Bouwt een track met een volledig zelf bepaalde tagset.
    fn track_with(name: &str, tags: Tags) -> TrackSummary {
        TrackSummary {
            name: name.to_string(),
            path: format!("Album/{name}"),
            tags,
            issues: Vec::new(),
            duration: "0:00".to_string(),
            format: "MP3".to_string(),
            has_art: false,
            art_url: String::new(),
            edit_url: format!("/bewerk/Album/{name}"),
        }
    }

    fn listing_of(tracks: Vec<TrackSummary>) -> Listing {
        Listing {
            name: "Album".to_string(),
            path: "Artiest/Album".to_string(),
            url: "/map/Artiest/Album".to_string(),
            album_url: "/album/Artiest/Album".to_string(),
            crumbs: Vec::new(),
            folders: Vec::new(),
            tracks,
            folder_issues: Vec::new(),
            query: String::new(),
        }
    }

    fn album_with_two_albums() -> Listing {
        listing_of(vec![
            track("een.mp3", Some("Eerste"), Some(1)),
            track("twee.mp3", Some("Tweede"), Some(1)),
            track("drie.mp3", None, None),
        ])
    }

    fn field_of(page: &AlbumPage, field: SharedField) -> &SharedInput {
        page.fields
            .iter()
            .find(|input| input.name == field.name())
            .expect("elk gedeeld veld hoort op de pagina te staan")
    }

    #[test]
    fn opening_the_page_selects_everything() {
        let page = album(&album_with_two_albums(), &Form::select_all());

        assert_eq!(page.selected, 3);
        assert_eq!(page.total, 3);
        assert!(page.rows.iter().all(|row| row.selected));
        assert!(page.has_selection());
        assert!(!page.changes_anything());
    }

    #[test]
    fn the_none_action_empties_the_selection() {
        let form = Form::parse("actie=niets&bestand=een.mp3&bestand=twee.mp3");
        let page = album(&album_with_two_albums(), &form);

        assert_eq!(page.selected, 0);
        assert!(page.rows.iter().all(|row| !row.selected));
        assert!(!page.has_selection());
    }

    #[test]
    fn the_all_action_wins_from_what_was_ticked() {
        let form = Form::parse("actie=alles&bestand=een.mp3");
        let page = album(&album_with_two_albums(), &form);

        assert_eq!(page.selected, 3);
    }

    #[test]
    fn a_kept_selection_stays_exactly_as_it_was_sent() {
        let form = Form::parse("bestand=twee.mp3");
        let page = album(&album_with_two_albums(), &form);

        assert_eq!(page.selected, 1);
        let ticked: Vec<&str> = page
            .rows
            .iter()
            .filter(|row| row.selected)
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(ticked, vec!["twee.mp3"]);
    }

    #[test]
    fn a_name_that_is_no_longer_in_the_folder_falls_away() {
        let form = Form::parse("bestand=weg.mp3&bestand=een.mp3");
        let page = album(&album_with_two_albums(), &form);

        assert_eq!(page.selected, 1);
    }

    #[test]
    fn typed_values_survive_a_change_of_selection() {
        // AC #2: het aanpassen van de selectie mag de invoer niet wissen.
        let form = Form::parse("actie=niets&album=Nieuw+album&wis_genre=aan");
        let page = album(&album_with_two_albums(), &form);

        assert_eq!(field_of(&page, SharedField::Album).value, "Nieuw album");
        assert!(field_of(&page, SharedField::Genre).cleared);
        assert!(page.changes_anything());
    }

    #[test]
    fn a_shared_value_is_reported_as_shared() {
        let listing = listing_of(vec![
            track("een.mp3", Some("Hetzelfde"), Some(1)),
            track("twee.mp3", Some("Hetzelfde"), Some(1)),
        ]);

        let page = album(&listing, &Form::select_all());
        let field = field_of(&page, SharedField::Album);

        assert_eq!(field.current, Current::Same("Hetzelfde".to_string()));
        assert!(!field.current.is_different());
        assert!(field.current.describe().contains("Hetzelfde"));
    }

    #[test]
    fn differing_values_are_visible_in_the_input() {
        // AC #5: de gebruiker moet zien dát de selectie uiteenloopt, en waarin.
        let page = album(&album_with_two_albums(), &Form::select_all());
        let field = field_of(&page, SharedField::Album);

        assert!(field.current.is_different());

        let described = field.current.describe();
        assert!(described.contains("verschillend"), "{described}");
        assert!(described.contains("Eerste"), "{described}");
        assert!(described.contains("Tweede"), "{described}");
        // Het bestand zonder album hoort als "leeg" mee te tellen.
        assert!(described.contains("leeg"), "{described}");

        assert!(field.current.placeholder().contains("Verschillend"));
    }

    #[test]
    fn a_field_that_is_empty_everywhere_says_so() {
        let listing = listing_of(vec![track("een.mp3", None, None)]);
        let page = album(&listing, &Form::select_all());

        assert_eq!(field_of(&page, SharedField::Genre).current, Current::Empty);
    }

    #[test]
    fn without_a_selection_there_is_nothing_to_report() {
        let form = Form::parse("actie=niets");
        let page = album(&album_with_two_albums(), &form);

        assert_eq!(
            field_of(&page, SharedField::Album).current,
            Current::Nothing
        );
    }

    #[test]
    fn leaving_a_field_empty_differs_from_clearing_it() {
        // AC #4, en de kern van deze pagina: leeg is niet hetzelfde als weg.
        let untouched = album(&album_with_two_albums(), &Form::parse("actie=alles"));
        assert!(
            field_of(&untouched, SharedField::Album)
                .effect
                .contains("ongemoeid")
        );

        let cleared = album(
            &album_with_two_albums(),
            &Form::parse("actie=alles&wis_album=aan"),
        );
        let effect = &field_of(&cleared, SharedField::Album).effect;
        assert!(effect.contains("verwijderd"), "{effect}");
        assert!(effect.contains("3 bestanden"), "{effect}");
    }

    #[test]
    fn bad_input_is_reported_once_on_the_page() {
        let page = album(&album_with_two_albums(), &Form::parse("actie=alles&disc=x"));

        assert_eq!(page.problems.len(), 1, "{:?}", page.problems);
        assert!(page.problems[0].starts_with("Discnummer"));
    }

    fn row_of<'a>(page: &'a AlbumPage, name: &str) -> &'a Row {
        page.rows
            .iter()
            .find(|row| row.name == name)
            .expect("de rij hoort er te zijn")
    }

    #[test]
    fn an_override_is_read_per_file_and_per_field() {
        let form = Form::parse("nummer:een.mp3=3&titel:een.mp3=Stilte+in+D&titel:twee.mp3=Ruis");

        assert_eq!(form.override_value("een.mp3", RowField::Track), "3");
        assert_eq!(
            form.override_value("een.mp3", RowField::Title),
            "Stilte in D"
        );
        assert_eq!(form.override_value("twee.mp3", RowField::Title), "Ruis");
        // Wat niets gekregen heeft, blijft leeg.
        assert_eq!(form.override_value("twee.mp3", RowField::Track), "");
        assert_eq!(form.override_value("drie.mp3", RowField::Title), "");
    }

    #[test]
    fn a_file_name_may_contain_a_colon_itself() {
        // Er wordt op de eerste dubbele punt gesplitst, en die staat vast in
        // het voorvoegsel.
        let form = Form::parse("titel%3Aa%3Ab.mp3=Werkt");

        assert_eq!(form.override_value("a:b.mp3", RowField::Title), "Werkt");
    }

    #[test]
    fn an_empty_override_changes_nothing_and_a_filled_one_sets_a_value() {
        let form = Form::parse("titel:een.mp3=+++&nummer:een.mp3=7");

        assert_eq!(
            form.row_intent("een.mp3", RowField::Title),
            Ok(Intent::Unchanged)
        );
        assert_eq!(
            form.row_intent("een.mp3", RowField::Track),
            Ok(Intent::Set("7".to_string()))
        );
    }

    #[test]
    fn a_track_number_that_is_not_a_number_is_refused() {
        let form = Form::parse("nummer:een.mp3=drie");

        let problem = form
            .row_intent("een.mp3", RowField::Track)
            .expect_err("dit hoort een fout te zijn");

        assert!(problem.starts_with("Tracknummer"), "{problem}");
        assert!(problem.contains("drie"), "{problem}");
    }

    #[test]
    fn the_table_offers_a_field_per_row_with_the_current_value_as_a_hint() {
        // AC #1: titel en tracknummer zijn per rij in te tikken.
        let page = album(&album_with_two_albums(), &Form::select_all());
        let row = row_of(&page, "een.mp3");

        assert_eq!(row.track_name, "nummer:een.mp3");
        assert_eq!(row.title_name, "titel:een.mp3");
        // Niets voorgevuld: leeg betekent hier hetzelfde als bij een gedeeld
        // veld, namelijk ongemoeid laten.
        assert_eq!(row.track_input, "");
        assert_eq!(row.title_input, "");
        assert!(!row.is_overridden());
    }

    #[test]
    fn typed_overrides_survive_a_change_of_selection() {
        // AC #2: de selectie of de gedeelde velden aanpassen mag de tabel niet
        // leegvegen.
        let form = Form::parse("actie=niets&titel:een.mp3=Nieuwe+titel&album=Nieuw+album");
        let page = album(&album_with_two_albums(), &form);

        let row = row_of(&page, "een.mp3");
        assert_eq!(row.title_input, "Nieuwe titel");
        assert!(row.is_overridden());
        // Maar niet geselecteerd, dus er gebeurt niets mee — en dat staat er.
        assert!(row.is_ignored());
        assert_eq!(page.overridden, 0);
    }

    #[test]
    fn an_override_wins_from_a_shared_value_for_the_same_file() {
        // AC #3: wie in de rij zelf iets intikt, bedoelt dat voor dat bestand.
        let form =
            Form::parse("actie=alles&album=Nieuw+album&titel:een.mp3=Eigen+titel&nummer:een.mp3=4");
        let plan = intents(&album_with_two_albums(), &form);

        assert_eq!(plan.len(), 3, "{plan:?}");

        let first = plan
            .iter()
            .find(|file| file.name == "een.mp3")
            .expect("het bestand hoort in het plan te staan");
        assert_eq!(
            first.fields.get("title"),
            Some(&Intent::Set("Eigen titel".to_string()))
        );
        assert_eq!(
            first.fields.get("track"),
            Some(&Intent::Set("4".to_string()))
        );
        // Het gedeelde veld geldt nog steeds voor dit bestand.
        assert_eq!(
            first.fields.get("album"),
            Some(&Intent::Set("Nieuw album".to_string()))
        );

        // De rest van de selectie krijgt alleen het gedeelde veld.
        let second = plan
            .iter()
            .find(|file| file.name == "twee.mp3")
            .expect("het bestand hoort in het plan te staan");
        assert_eq!(second.fields.get("title"), None);
        assert_eq!(
            second.fields.get("album"),
            Some(&Intent::Set("Nieuw album".to_string()))
        );
    }

    #[test]
    fn a_file_that_changes_nothing_stays_out_of_the_plan() {
        let form = Form::parse("actie=alles&titel:een.mp3=Alleen+deze");
        let plan = intents(&album_with_two_albums(), &form);

        assert_eq!(plan.len(), 1, "{plan:?}");
        assert_eq!(plan[0].name, "een.mp3");
    }

    #[test]
    fn an_unselected_file_stays_out_of_the_plan() {
        let form = Form::parse("bestand=twee.mp3&titel:een.mp3=Wordt+niet+opgeslagen");

        assert!(intents(&album_with_two_albums(), &form).is_empty());
    }

    #[test]
    fn bad_input_in_a_row_is_reported_there_and_blocks_only_that_row() {
        // AC #4: één typefout mag de rest van de tabel niet ophouden.
        let form = Form::parse("actie=alles&nummer:een.mp3=drie&titel:twee.mp3=Wel+dit");
        let page = album(&album_with_two_albums(), &form);

        let broken = row_of(&page, "een.mp3");
        assert!(broken.has_problems());
        assert!(
            broken.problems[0].starts_with("Tracknummer"),
            "{:?}",
            broken
        );

        let fine = row_of(&page, "twee.mp3");
        assert!(!fine.has_problems());

        // De gedeelde velden blijven bruikbaar; alleen de kapotte rij valt weg.
        assert!(page.problems.is_empty(), "{:?}", page.problems);
        assert!(page.has_row_problems());

        let plan = intents(&album_with_two_albums(), &form);
        assert_eq!(plan.len(), 1, "{plan:?}");
        assert_eq!(plan[0].name, "twee.mp3");
    }

    #[test]
    fn overrides_count_towards_what_would_happen() {
        let untouched = album(&album_with_two_albums(), &Form::select_all());
        assert!(!untouched.changes_anything());
        assert!(untouched.overrides_effect().contains("blijft ongemoeid"));

        let form = Form::parse("actie=alles&titel:een.mp3=Eén&titel:twee.mp3=Twee");
        let page = album(&album_with_two_albums(), &form);

        assert!(page.changes_anything());
        assert_eq!(page.overridden, 2);
        // Het derde bestand krijgt niets en verandert dus ook niet.
        assert_eq!(page.changed_files, 2);
        assert_eq!(
            page.changed_files_effect(),
            "In totaal veranderen er 2 bestanden."
        );
        assert_eq!(
            page.overrides_effect(),
            "2 bestanden krijgen een eigen waarde uit de tabel."
        );
    }

    /// Een album waarin elk bestand zijn eigen artiest en titel heeft, en de
    /// tracknummers niet kloppen: precies waar de hulpacties voor zijn.
    fn album_that_needs_help() -> Listing {
        listing_of(vec![
            track_with(
                "een.mp3",
                Tags {
                    title: Some("STILTE IN D".to_string()),
                    artist: Some("de testartiest".to_string()),
                    album: Some("fixtures voor sleeve".to_string()),
                    track: Some(7),
                    ..Tags::default()
                },
            ),
            track_with(
                "twee.mp3",
                Tags {
                    title: Some("ruis in b".to_string()),
                    artist: Some("Een Ander".to_string()),
                    album: Some("fixtures voor sleeve".to_string()),
                    ..Tags::default()
                },
            ),
            track_with(
                "drie.mp3",
                Tags {
                    title: Some("Al Goed".to_string()),
                    album: Some("fixtures voor sleeve".to_string()),
                    ..Tags::default()
                },
            ),
        ])
    }

    #[test]
    fn renumbering_follows_the_order_of_the_table() {
        // AC #1: opeenvolgend nummeren volgens de huidige sortering, en niet
        // volgens de nummers die er nu in staan — die kloppen juist niet.
        let page = album(
            &album_that_needs_help(),
            &Form::parse("actie=hernummer&bestand=een.mp3&bestand=twee.mp3&bestand=drie.mp3"),
        );

        assert_eq!(row_of(&page, "een.mp3").track_input, "1");
        assert_eq!(row_of(&page, "twee.mp3").track_input, "2");
        assert_eq!(row_of(&page, "drie.mp3").track_input, "3");

        let notice = page.helper_notice.expect("de actie hoort iets te melden");
        assert!(notice.contains("1 tot en met 3"), "{notice}");
    }

    #[test]
    fn renumbering_leaves_what_is_not_selected_alone() {
        let form = Form::parse("actie=hernummer&bestand=een.mp3&bestand=drie.mp3");
        let page = album(&album_that_needs_help(), &form);

        assert_eq!(row_of(&page, "een.mp3").track_input, "1");
        assert_eq!(row_of(&page, "drie.mp3").track_input, "2");
        assert_eq!(row_of(&page, "twee.mp3").track_input, "");
    }

    #[test]
    fn copying_the_artist_fills_the_album_artist_per_file() {
        // AC #2: per bestand, want de artiesten hoeven niet gelijk te zijn.
        let page = album(
            &album_that_needs_help(),
            &Form::parse("actie=artiest&bestand=een.mp3&bestand=twee.mp3&bestand=drie.mp3"),
        );

        assert_eq!(
            row_of(&page, "een.mp3").album_artist_input,
            "de testartiest"
        );
        assert_eq!(row_of(&page, "twee.mp3").album_artist_input, "Een Ander");
        // Zonder artiest valt er niets te kopiëren; een lege albumartiest
        // voorstellen zou een verwijdering zijn.
        assert_eq!(row_of(&page, "drie.mp3").album_artist_input, "");

        let notice = page.helper_notice.expect("de actie hoort iets te melden");
        assert!(notice.contains("2 bestanden"), "{notice}");
        assert!(notice.contains("1 zonder artiest"), "{notice}");
    }

    #[test]
    fn a_row_wins_from_the_shared_album_artist() {
        // De keerzijde van FR-9: het gedeelde veld geldt voor de selectie, maar
        // waar de rij iets zegt, wint de rij.
        let form = Form::parse(
            "actie=artiest&bestand=een.mp3&bestand=drie.mp3&album_artist=Voor+iedereen",
        );
        let plan = intents(&album_that_needs_help(), &form);

        let first = plan
            .iter()
            .find(|file| file.name == "een.mp3")
            .expect("het bestand hoort in het plan te staan");
        assert_eq!(
            first.fields.get("album_artist"),
            Some(&Intent::Set("de testartiest".to_string()))
        );

        // Het bestand zonder artiest houdt de gedeelde waarde.
        let third = plan
            .iter()
            .find(|file| file.name == "drie.mp3")
            .expect("het bestand hoort in het plan te staan");
        assert_eq!(
            third.fields.get("album_artist"),
            Some(&Intent::Set("Voor iedereen".to_string()))
        );
    }

    #[test]
    fn normalising_capitals_proposes_a_title_per_file() {
        // AC #3: het resultaat komt als voorstel in de invoervelden te staan.
        let page = album(
            &album_that_needs_help(),
            &Form::parse("actie=hoofdletters&bestand=een.mp3&bestand=twee.mp3&bestand=drie.mp3"),
        );

        assert_eq!(row_of(&page, "een.mp3").title_input, "Stilte in D");
        assert_eq!(row_of(&page, "twee.mp3").title_input, "Ruis in B");
        // Wat al klopt, krijgt geen voorstel: dat zou geen voorstel zijn.
        assert_eq!(row_of(&page, "drie.mp3").title_input, "");
    }

    #[test]
    fn normalising_capitals_fills_a_shared_field_when_the_selection_agrees() {
        let page = album(
            &album_that_needs_help(),
            &Form::parse("actie=hoofdletters&bestand=een.mp3&bestand=twee.mp3&bestand=drie.mp3"),
        );

        assert_eq!(
            field_of(&page, SharedField::Album).value,
            "Fixtures voor Sleeve"
        );
        // Genre staat nergens ingevuld, dus valt er niets voor te stellen.
        assert_eq!(field_of(&page, SharedField::Genre).value, "");
    }

    #[test]
    fn normalising_capitals_leaves_a_shared_field_alone_when_it_differs() {
        // Eén gedeeld veld kan geen twee voorstellen bevatten; dan maar niets.
        let listing = listing_of(vec![
            track("een.mp3", Some("eerste album"), None),
            track("twee.mp3", Some("tweede album"), None),
        ]);
        let page = album(&listing, &Form::parse("actie=hoofdletters&actie=alles"));

        assert_eq!(field_of(&page, SharedField::Album).value, "");
    }

    #[test]
    fn normalising_capitals_works_on_what_was_typed_by_hand() {
        let form = Form::parse("actie=hoofdletters&bestand=een.mp3&titel:een.mp3=EEN+ANDERE+TITEL");
        let page = album(&album_that_needs_help(), &form);

        assert_eq!(row_of(&page, "een.mp3").title_input, "Een Andere Titel");
    }

    #[test]
    fn nothing_to_normalise_says_so() {
        let listing = listing_of(vec![track_with(
            "een.mp3",
            Tags {
                title: Some("Al Goed".to_string()),
                ..Tags::default()
            },
        )]);
        let page = album(&listing, &Form::parse("actie=hoofdletters&bestand=een.mp3"));

        let notice = page
            .helper_notice
            .clone()
            .expect("de actie hoort iets te melden");
        assert!(notice.contains("niets te verbeteren"), "{notice}");
        assert!(!page.changes_anything());
    }

    #[test]
    fn a_helper_action_is_undone_by_emptying_the_input() {
        // AC #5: terugdraaien gebeurt door de invoer weg te halen, en dat kan
        // zolang er niet is opgeslagen.
        let filled = album(
            &album_that_needs_help(),
            &Form::parse("actie=hernummer&bestand=een.mp3&bestand=twee.mp3"),
        );
        assert!(filled.changes_anything());
        assert_eq!(filled.overridden, 2);

        // Hetzelfde formulier, maar nu met de herstelknop: de velden zijn leeg
        // en de selectie staat er nog.
        let restored = album(
            &album_that_needs_help(),
            &Form::parse(
                "actie=herstel&bestand=een.mp3&bestand=twee.mp3&nummer:een.mp3=1&album=Nieuw",
            ),
        );

        assert_eq!(restored.selected, 2);
        assert_eq!(restored.overridden, 0);
        assert_eq!(restored.changed_files, 0);
        assert!(!restored.changes_anything());
        assert_eq!(field_of(&restored, SharedField::Album).value, "");
    }

    #[test]
    fn a_helper_action_only_fills_fields_and_keeps_the_selection() {
        // AC #4: er komt alleen invoer bij. Wat er geselecteerd was, blijft
        // geselecteerd, en het plan komt van diezelfde ingevulde velden.
        let form = Form::parse("actie=hernummer&bestand=twee.mp3");
        let page = album(&album_that_needs_help(), &form);

        assert_eq!(page.selected, 1);
        assert!(row_of(&page, "twee.mp3").selected);
        assert!(!row_of(&page, "een.mp3").selected);
        assert_eq!(page.changed_files, 1);
    }

    #[test]
    fn the_table_shows_a_dash_where_a_tag_is_missing() {
        let page = album(&album_with_two_albums(), &Form::select_all());
        let row = page
            .rows
            .iter()
            .find(|row| row.name == "drie.mp3")
            .expect("de rij hoort er te zijn");

        assert_eq!(row.album, EMPTY);
        assert_eq!(row.disc, EMPTY);
    }
}
