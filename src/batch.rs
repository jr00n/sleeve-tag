//! De albumweergave: een selectie bestanden, de velden die ze delen (FR-8) en
//! wat er per bestand van afwijkt (FR-9).
//!
//! Waar [`crate::edit`] één bestand bedient, gaat het hier om een map vol
//! bestanden tegelijk. Deze module vertaalt tussen het verstuurde formulier en
//! een weergavemodel dat de templates rechtstreeks kunnen renderen, en bepaalt
//! per veld wat er met de selectie zou gebeuren. Tracknummer, titel, artiest,
//! albumartiest, album, jaar en genre kunnen per bestand verschillen en zijn
//! daarom in de tabel zelf in te tikken; zo'n override wint van een gedeelde
//! waarde voor datzelfde bestand.
//!
//! De hulpacties uit FR-10 horen hier ook: hernummeren (over de selectie of per
//! schijf), een schijf een nummer geven, de disctotalen invullen, de titel uit
//! de bestandsnaam lezen, artiest → albumartiest en hoofdletters normaliseren
//! vullen invoervelden van datzelfde formulier en doen verder niets.
//!
//! Er wordt hier niets geschreven en er gaat geen bestand open: in en uit gaan
//! een [`Listing`] en een [`Form`]. Ook [`preview`] stelt alleen voor — het
//! daadwerkelijk wegschrijven doet de handler, en alleen langs die
//! voorbeeldweergave, zodat een batch-wijziging nooit ongezien plaatsvindt.

use std::collections::{BTreeMap, BTreeSet};

use percent_encoding::percent_decode_str;

use crate::browse::{self, Crumb, DiscGroup, Listing, TrackSummary};
use crate::casing;
use crate::edit;
use crate::naming;
use crate::tags::Tags;

/// Wat er in een invoerveld staat waar de selectie niets te melden heeft.
const EMPTY: &str = "—";

/// Het voorvoegsel van de knop die één schijf aan- of uitvinkt.
///
/// De groep staat erachter: het discnummer, of niets voor de bestanden die er
/// geen hebben. Zo is er één knopnaam nodig in plaats van een veld per groep.
const GROUP_ACTION: &str = "schijf:";

/// Een veld dat een heel album deelt (PRD FR-8).
///
/// Titel, artiest en tracknummer horen hier bewust níét bij: die verschillen
/// per bestand. Wat hier wél staat, staat óók als kolom in de tabel — behalve
/// het discnummer en het aantal discs, die voor een hele schijf gelden en niet
/// voor één bestand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedField {
    AlbumArtist,
    Album,
    Year,
    Genre,
    Disc,
    DiscTotal,
}

impl SharedField {
    /// Alle gedeelde velden, in de volgorde waarin ze op het scherm staan.
    pub const ALL: [SharedField; 6] = [
        SharedField::AlbumArtist,
        SharedField::Album,
        SharedField::Year,
        SharedField::Genre,
        SharedField::Disc,
        SharedField::DiscTotal,
    ];

    /// De naam waaronder het veld in het formulier staat.
    pub fn name(self) -> &'static str {
        match self {
            SharedField::AlbumArtist => "album_artist",
            SharedField::Album => "album",
            SharedField::Year => "year",
            SharedField::Genre => "genre",
            SharedField::Disc => "disc",
            SharedField::DiscTotal => "disc_total",
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
            SharedField::DiscTotal => "Aantal discs",
        }
    }

    /// Of er een getal in hoort; bepaalt de controle en het toetsenbord.
    pub fn is_numeric(self) -> bool {
        matches!(self, SharedField::Disc | SharedField::DiscTotal)
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
            SharedField::DiscTotal => tags.disc_total.map(|number| number.to_string()),
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
            SharedField::DiscTotal => 5,
        }
    }
}

/// Een veld dat per bestand verschilt en daarom in de tabel zelf staat (FR-9).
///
/// De tegenhanger van [`SharedField`]: waar dat veld één waarde voor de hele
/// selectie zet, hoort hier per rij iets anders te kunnen staan. Een
/// compilatie waarin elke track een andere artiest heeft, is anders alleen
/// bestand voor bestand recht te zetten — terwijl de tabel er al staat.
///
/// Albumartiest, album, jaar en genre staan in beide lijstjes, en dat is geen
/// vergissing: ze zijn meestal voor het hele album gelijk, en dan is het
/// gedeelde veld de kortste weg. Waar ze elkaar raken wint de rij; [`intents`]
/// legt die volgorde vast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowField {
    Track,
    Title,
    Artist,
    AlbumArtist,
    Album,
    Year,
    Genre,
}

impl RowField {
    /// Alle velden, in de volgorde waarin ze in de tabel staan.
    pub const ALL: [RowField; 7] = [
        RowField::Track,
        RowField::Title,
        RowField::Artist,
        RowField::AlbumArtist,
        RowField::Album,
        RowField::Year,
        RowField::Genre,
    ];

    /// De naam van het veld in het tagmodel; ook de sleutel in een [`FileIntent`].
    pub fn field_name(self) -> &'static str {
        match self {
            RowField::Track => "track",
            RowField::Title => "title",
            RowField::Artist => "artist",
            RowField::AlbumArtist => "album_artist",
            RowField::Album => "album",
            RowField::Year => "year",
            RowField::Genre => "genre",
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
    ///
    /// Bewust niet gelijk aan [`SharedField::name`]: de rijen en de gedeelde
    /// velden zitten in dezelfde body, en `album` mag daar niet twee dingen
    /// betekenen. Vandaar `albumtitel` voor de kolom en `album` voor het
    /// gedeelde veld.
    fn prefix(self) -> &'static str {
        match self {
            RowField::Track => "nummer",
            RowField::Title => "titel",
            RowField::Artist => "artiest",
            RowField::AlbumArtist => "albumartiest",
            RowField::Album => "albumtitel",
            RowField::Year => "jaar",
            RowField::Genre => "genre",
        }
    }

    /// De volledige naam van het veld, zoals hij in een melding staat.
    pub fn label(self) -> &'static str {
        match self {
            RowField::Track => "Tracknummer",
            RowField::Title => "Titel",
            RowField::Artist => "Artiest",
            RowField::AlbumArtist => "Albumartiest",
            RowField::Album => "Album",
            RowField::Year => "Jaar",
            RowField::Genre => "Genre",
        }
    }

    /// Het opschrift boven de kolom.
    ///
    /// Korter dan [`RowField::label`] waar dat kan: een kolomkop staat boven
    /// een smalle kolom, en "#" zegt boven een tracknummer genoeg.
    pub fn column(self) -> &'static str {
        match self {
            RowField::Track => "#",
            other => other.label(),
        }
    }

    /// Of er een getal in hoort; bepaalt de controle en het toetsenbord.
    ///
    /// Het jaar hoort er níét bij, hoe getalachtig het er ook uitziet: in het
    /// tagmodel is het tekst, omdat ID3v2.4 en Vorbis er een volledige datum
    /// in kunnen zetten. Er een `u32` van eisen zou een bestaande
    /// `2024-05-01` onbewerkbaar maken. Het gedeelde veld Jaar leest om
    /// dezelfde reden vrij.
    pub fn is_numeric(self) -> bool {
        matches!(self, RowField::Track)
    }

    /// Hoe breed het invoerveld in de tabel is; het achtervoegsel van de
    /// CSS-klasse.
    ///
    /// Zeven invoervelden naast elkaar passen alleen als ze niet allemaal even
    /// breed zijn: een jaartal heeft aan vier tekens genoeg, een titel niet.
    pub fn size(self) -> &'static str {
        match self {
            RowField::Track => "nummer",
            RowField::Year => "kort",
            RowField::Genre => "middel",
            RowField::Title | RowField::Artist | RowField::AlbumArtist | RowField::Album => "tekst",
        }
    }

    /// Wat er voor dit veld in één bestand staat.
    fn value_of(self, tags: &Tags) -> Option<String> {
        match self {
            RowField::Track => tags.track.map(|number| number.to_string()),
            RowField::Title => tags.title.clone(),
            RowField::Artist => tags.artist.clone(),
            RowField::AlbumArtist => tags.album_artist.clone(),
            RowField::Album => tags.album.clone(),
            RowField::Year => tags.year.clone(),
            RowField::Genre => tags.genre.clone(),
        }
    }

    /// De plek van dit veld in de vaste arrays van [`Override`].
    fn index(self) -> usize {
        match self {
            RowField::Track => 0,
            RowField::Title => 1,
            RowField::Artist => 2,
            RowField::AlbumArtist => 3,
            RowField::Album => 4,
            RowField::Year => 5,
            RowField::Genre => 6,
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
    values: [String; 7],
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

    /// Eén schijf in zijn geheel aan- of uitvinken (FR-8).
    ///
    /// Welke van de twee het wordt, volgt uit wat er al aanstaat: staat de hele
    /// groep aan, dan gaat hij eraf, en anders erbij. De knop kan dat dus
    /// vooraf in zijn opschrift zetten. Wat er buiten de groep geselecteerd
    /// stond, blijft in beide gevallen staan.
    ToggleGroup(Option<u32>),

    /// De selectie opeenvolgend nummeren, in de volgorde van de tabel.
    Renumber,

    /// Per schijf een eigen reeks vanaf 1, in de volgorde van de tabel.
    RenumberPerDisc,

    /// De hele selectie op hetzelfde discnummer zetten.
    SetDisc,

    /// Het aantal schijven van deze map in het veld "aantal discs" zetten.
    FillDiscTotals,

    /// Voor bestanden zonder titel de titel uit de bestandsnaam voorstellen.
    TitleFromName,

    /// Per bestand de artiest als albumartiest voorstellen.
    CopyArtist,

    /// Het hoofdlettergebruik van de tekstvelden normaliseren.
    Capitalize,

    /// Alle ingevulde velden weer leegmaken.
    Reset,

    /// De voorbeeldweergave tonen: wat krijgt welk bestand (FR-11).
    Preview,

    /// Wegschrijven wat het voorbeeld liet zien.
    Save,
}

impl Action {
    fn parse(raw: &str) -> Action {
        // De knop van een groep draagt zijn discnummer in de waarde mee; een
        // lege rest is de groep zonder discnummer. Wat daar geen getal is, komt
        // niet uit een knop van deze pagina en verandert dus niets.
        if let Some(rest) = raw.strip_prefix(GROUP_ACTION) {
            return match rest {
                "" => Action::ToggleGroup(None),
                number => match number.parse::<u32>() {
                    Ok(disc) => Action::ToggleGroup(Some(disc)),
                    Err(_) => Action::Keep,
                },
            };
        }

        match raw {
            "alles" => Action::All,
            "niets" => Action::None,
            "hernummer" => Action::Renumber,
            "hernummer-disc" => Action::RenumberPerDisc,
            "disc" => Action::SetDisc,
            "disctotaal" => Action::FillDiscTotals,
            "titelnaam" => Action::TitleFromName,
            "artiest" => Action::CopyArtist,
            "hoofdletters" => Action::Capitalize,
            "herstel" => Action::Reset,
            "voorbeeld" => Action::Preview,
            "opslaan" => Action::Save,
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
    values: [String; 6],

    /// Of het wissen-vinkje aan stond, in dezelfde volgorde.
    clear: [bool; 6],

    /// De ingetikte overrides, per bestandsnaam (FR-9).
    ///
    /// Een bestand dat niets gekregen heeft, staat er niet in; wat er niet
    /// (meer) in de map staat, valt bij het opbouwen vanzelf af.
    overrides: BTreeMap<String, Override>,

    /// Of de meegestuurde hoes ook als los bestand in de map moet komen.
    ///
    /// Dezelfde keuze als op de hoespagina, met dezelfde standaard: nee. De
    /// afbeelding zelf zit niet in dit formulier — die reist apart mee, want
    /// alleen de laatste stap draagt hem.
    pub folder_cover: bool,

    /// Of een bestaande losse hoes overschreven mag worden.
    pub overwrite_folder_cover: bool,
}

impl Form {
    /// Leest een `application/x-www-form-urlencoded`-body.
    pub fn parse(body: &str) -> Form {
        let pairs: Vec<(String, String)> = body
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| match pair.split_once('=') {
                Some((key, value)) => (decode(key), decode(value)),
                None => (decode(pair), String::new()),
            })
            .collect();

        Form::from_pairs(pairs)
    }

    /// Bouwt het formulier uit al gedecodeerde sleutel-waardeparen.
    ///
    /// Apart van [`Form::parse`] omdat dezelfde velden ook uit een
    /// `multipart/form-data`-body kunnen komen: dat is de vorm die de
    /// voorbeeldweergave gebruikt zodra er een hoes meegaat.
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Form {
        let mut form = Form::default();

        for (key, raw) in pairs {
            match key.as_str() {
                "mapbestand" => form.folder_cover = true,
                "overschrijf" => form.overwrite_folder_cover = true,
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

    /// Hetzelfde formulier, maar zonder ingevulde velden.
    ///
    /// Wat er na het opslaan in de velden stond, is verwerkt; het nog eens
    /// tonen zou dezelfde wijziging opnieuw voorstellen. De selectie blijft wel
    /// staan: die zegt waar de gebruiker mee bezig is.
    pub fn without_input(&self) -> Form {
        Form {
            selected: self.selected.clone(),
            ..Form::default()
        }
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
    /// wat het heeft. Wissen kan hier niet — een veld van één bestand weghalen
    /// is geen batch-actie, en dat hoort in het bewerkformulier van dat bestand
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
    fn applied(&self, listing: &Listing, chosen: &[&TrackSummary]) -> (Form, Option<String>) {
        let mut form = self.clone();

        let notice = match self.action {
            Action::Renumber => Some(form.renumber(chosen)),
            Action::RenumberPerDisc => Some(form.renumber_per_disc(chosen)),
            Action::SetDisc => Some(form.set_disc(listing, chosen)),
            Action::FillDiscTotals => Some(form.fill_disc_totals(listing)),
            Action::TitleFromName => Some(form.title_from_name(chosen)),
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
            // De voorbeeldweergave en het opslaan veranderen niets aan het
            // formulier: ze werken juist met precies wat erin staat, en een
            // knop die de selectie zet, laat de invoervelden met rust.
            Action::Keep
            | Action::All
            | Action::None
            | Action::ToggleGroup(_)
            | Action::Preview
            | Action::Save => None,
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

    /// Nummert de selectie per schijf, elke schijf vanaf 1.
    ///
    /// Doorgeteld over de hele selectie begint de tweede schijf bij 13 in
    /// plaats van bij 1; dat is wat deze actie rechtzet. De volgorde binnen een
    /// schijf is die van de tabel, net als bij [`Form::renumber`]. Bestanden
    /// zonder discnummer horen bij elkaar en vormen samen één reeks: er valt
    /// niets anders over te zeggen dan dat ze niet bij een genummerde schijf
    /// zijn ondergebracht.
    ///
    /// Een bestand dat al op het voorgestelde nummer staat, krijgt geen
    /// voorstel: dat zou geen voorstel zijn.
    fn renumber_per_disc(&mut self, chosen: &[&TrackSummary]) -> String {
        let mut positions: BTreeMap<Option<u32>, u32> = BTreeMap::new();
        let mut proposals = 0;

        for track in chosen {
            let position = positions.entry(track.tags.disc).or_default();
            *position += 1;

            let proposal = position.to_string();
            let keep = track.tags.track != Some(*position);

            self.set_override(
                &track.name,
                RowField::Track,
                if keep { proposal } else { String::new() },
            );
            proposals += usize::from(keep);
        }

        if chosen.is_empty() {
            return "Er is niets geselecteerd om te hernummeren.".to_string();
        }

        let discs = positions.len();
        let schijven = if discs == 1 {
            "1 schijf".to_string()
        } else {
            format!("{discs} schijven")
        };

        match proposals {
            0 => format!(
                "De selectie beslaat {schijven} en staat daarbinnen al doorlopend genummerd."
            ),
            1 => format!(
                "De selectie is per schijf genummerd vanaf 1 ({schijven}); 1 veld heeft een voorstel gekregen."
            ),
            count => format!(
                "De selectie is per schijf genummerd vanaf 1 ({schijven}); {count} velden hebben een voorstel gekregen."
            ),
        }
    }

    /// Zet de hele selectie op hetzelfde discnummer.
    ///
    /// Welk nummer dat is, volgt uit wat er al ligt — zie
    /// [`disc_suggestion`] — zodat de knop het vooraf kan tonen. Staat de
    /// selectie er al op, dan valt er niets voor te stellen.
    fn set_disc(&mut self, listing: &Listing, chosen: &[&TrackSummary]) -> String {
        if chosen.is_empty() {
            return "Er is niets geselecteerd om op een schijf te zetten.".to_string();
        }

        let number = disc_suggestion(listing, chosen);

        if let Current::Same(current) = Current::of(SharedField::Disc, chosen)
            && current == number.to_string()
        {
            self.values[SharedField::Disc.index()] = String::new();
            return format!("De selectie staat al op schijf {number}.");
        }

        self.values[SharedField::Disc.index()] = number.to_string();

        format!(
            "Schijf {number} staat als voorstel bij Discnummer; het geldt voor de geselecteerde bestanden."
        )
    }

    /// Zet het aantal schijven van deze map in het veld "aantal discs".
    ///
    /// Het totaal hoort in élk bestand van de set te staan — zonder dat totaal
    /// weten spelers niet dat de set compleet is. Daarom gaat deze actie over
    /// de hele map en niet over een deel ervan; [`resolve_selection`] vinkt
    /// daarom alles aan.
    fn fill_disc_totals(&mut self, listing: &Listing) -> String {
        if listing.tracks.is_empty() {
            return "Deze map bevat geen bewerkbare bestanden.".to_string();
        }

        let discs = disc_count(listing);
        let schijven = if discs == 1 {
            "1 schijf".to_string()
        } else {
            format!("{discs} schijven")
        };

        if listing
            .tracks
            .iter()
            .all(|track| track.tags.disc_total == Some(discs))
        {
            self.values[SharedField::DiscTotal.index()] = String::new();
            return format!("Elk bestand in deze map heeft het aantal discs al op {discs} staan.");
        }

        self.values[SharedField::DiscTotal.index()] = discs.to_string();

        let unnumbered = listing
            .tracks
            .iter()
            .filter(|track| track.tags.disc.is_none())
            .count();

        let mut notice = format!(
            "Deze map bevat {schijven}; dat aantal staat als voorstel bij Aantal discs, voor alle bestanden in de map."
        );

        if unnumbered > 0 {
            notice.push_str(&format!(
                " {unnumbered} bestanden hebben zelf nog geen discnummer."
            ));
        }

        notice
    }

    /// Stelt voor bestanden zonder titel de bestandsnaam als titel voor.
    ///
    /// Alleen voor bestanden die geen titel hebben: waar er wel één staat, is
    /// de tag betrouwbaarder dan de bestandsnaam, en die wordt dus niet
    /// overschreven. Wat er uit de naam te lezen valt, bepaalt
    /// [`crate::naming`]; valt er niets uit te halen, dan blijft het veld leeg.
    fn title_from_name(&mut self, chosen: &[&TrackSummary]) -> String {
        let mut proposals = 0;
        let mut titled = 0;
        let mut empty_handed = 0;

        for track in chosen {
            if track.tags.title.is_some() {
                titled += 1;
                continue;
            }

            match naming::title_from_filename(&track.name) {
                Some(title) => {
                    self.set_override(&track.name, RowField::Title, title);
                    proposals += 1;
                }
                None => empty_handed += 1,
            }
        }

        let mut notice = match proposals {
            0 => "Er valt geen titel uit een bestandsnaam te halen.".to_string(),
            1 => "Bij 1 bestand staat de titel uit de bestandsnaam in de tabel.".to_string(),
            count => {
                format!("Bij {count} bestanden staat de titel uit de bestandsnaam in de tabel.")
            }
        };

        if titled > 0 {
            notice.push_str(&format!(" {titled} met een titel zijn ongemoeid gelaten."));
        }

        if empty_handed > 0 {
            notice.push_str(&format!(
                " Uit {empty_handed} bestandsnamen viel geen titel te lezen."
            ));
        }

        notice
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

/// Welk discnummer de selectie in één klik zou krijgen (FR-10).
///
/// Staat de selectie al op één schijf, dan die: dan is de actie een bevestiging
/// en geen verplaatsing. Anders de eerstvolgende die in deze map nog niet in
/// gebruik is, want een nieuwe schijf hoort een nummer te krijgen dat nog vrij
/// is. Bestanden zonder discnummer tellen daarbij niet mee — die zitten juist
/// op geen enkele schijf.
///
/// Publiek omdat de knop het nummer vooraf toont: wat er gaat gebeuren hoort te
/// lezen te zijn voordat er geklikt wordt (AC #2).
pub fn disc_suggestion(listing: &Listing, chosen: &[&TrackSummary]) -> u32 {
    let mut in_selection: BTreeSet<u32> = BTreeSet::new();
    for track in chosen {
        if let Some(number) = track.tags.disc {
            in_selection.insert(number);
        }
    }

    if let [only] = in_selection
        .iter()
        .copied()
        .collect::<Vec<u32>>()
        .as_slice()
    {
        return *only;
    }

    let in_use: BTreeSet<u32> = listing
        .tracks
        .iter()
        .filter_map(|track| track.tags.disc)
        .collect();

    (1u32..)
        .find(|number| !in_use.contains(number))
        .unwrap_or(1)
}

/// Hoeveel schijven deze map bevat.
///
/// Het aantal verschillende discnummers dat erin voorkomt. Bestanden zonder
/// discnummer maken er geen schijf bij: waar ze bij horen is niet te zeggen, en
/// er een extra schijf van maken zou een set van twee cd's stilzwijgend op drie
/// zetten. Staat er nergens een discnummer, dan is het één schijf — dat is wat
/// een gewoon album is.
fn disc_count(listing: &Listing) -> u32 {
    let discs: BTreeSet<u32> = listing
        .tracks
        .iter()
        .filter_map(|track| track.tags.disc)
        .collect();

    discs.len().max(1) as u32
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

/// Eén invoerveld in één rij van de tabel (FR-9).
///
/// De tegenhanger van [`SharedInput`]: waar dat veld één waarde over de hele
/// selectie zet, gaat dit over dit ene bestand. Wat er nú in het bestand staat,
/// staat als grijze tekst in het veld en niet als waarde — precies zoals bij
/// een gedeeld veld, zodat leeg overal hetzelfde betekent: ongemoeid laten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowInput {
    /// De naam in het formulier: het veld en de bestandsnaam.
    pub name: String,

    /// Wat een schermlezer voorleest.
    ///
    /// De kolomkop staat er al boven, maar in een tabel vol invoervelden hoort
    /// elk veld ook te zeggen bij welk bestand het hoort.
    pub label: String,

    /// Wat de gebruiker heeft ingetikt; leeg bij het openen van de pagina.
    pub value: String,

    /// Wat er nu in het bestand staat; een streepje wanneer de tag ontbreekt.
    pub placeholder: String,

    /// Of er een getal in hoort.
    pub numeric: bool,

    /// Het achtervoegsel van de breedteklasse; zie [`RowField::size`].
    pub size: String,

    /// Wat er aan déze invoer mankeert.
    ///
    /// Bij het veld zelf en niet bij de rij als geheel: met zeven kolommen is
    /// "er klopt iets niet in deze rij" geen bruikbare melding meer.
    pub problem: Option<String>,
}

/// De kop boven één schijf in de albumtabel.
///
/// Wat er in staat komt uit [`DiscGroup`]; wat de knop ernaast doet, hangt af
/// van de selectie en wordt daarom hier bepaald. De kop hangt aan de rij waar
/// de groep begint: hij staat als eigen rij in dezelfde tabel, en zo blijft de
/// tabel één opsomming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupHeading {
    /// Het discnummer van deze groep; `None` voor de bestanden zonder.
    pub disc: Option<u32>,

    /// Het opschrift: "Schijf 1", of "Zonder discnummer".
    pub label: String,

    /// De telling en wat er aandacht vraagt, in één zin.
    pub summary: String,

    /// De waarde van de knop; hij zegt om welke groep het gaat.
    pub action: String,

    /// Het opschrift van de knop, dat zegt wat een klik doet.
    pub button: String,
}

impl GroupHeading {
    /// Bouwt de kop van één groep, gegeven wat er geselecteerd staat.
    ///
    /// Staat de hele groep al aan, dan vinkt de knop hem uit; anders vinkt hij
    /// hem aan. Zo is er één knop per groep die zichzelf verklaart, en raakt
    /// een klik nooit de bestanden van een andere schijf (AC #4).
    fn of(group: &DiscGroup, listing: &Listing, selected: &BTreeSet<String>) -> GroupHeading {
        let label = group.label();

        let complete = listing
            .tracks
            .iter()
            .filter(|track| track.tags.disc == group.disc)
            .all(|track| selected.contains(&track.name));

        GroupHeading {
            disc: group.disc,
            action: format!("{GROUP_ACTION}{}", group.key()),
            button: if complete {
                format!("{label} uitvinken")
            } else {
                format!("{label} selecteren")
            },
            summary: group.describe(),
            label,
        }
    }
}

/// Eén regel in de albumtabel.
///
/// De waarden zijn hier al tekst: de tabel toont ze rechtstreeks, en een
/// ontbrekende tag hoort als streepje zichtbaar te zijn en niet als lege cel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Bestandsnaam; ook de waarde van het selectievinkje.
    pub name: String,

    /// De kop die vóór deze rij hoort; alleen bij het eerste bestand van een
    /// groep, en alleen wanneer de map überhaupt op schijven uiteenvalt.
    pub group: Option<GroupHeading>,

    /// Of dit bestand geselecteerd is.
    pub selected: bool,

    /// De invulbare velden van deze rij, in de volgorde van [`RowField::ALL`].
    pub inputs: Vec<RowInput>,

    /// Het discnummer zoals het in het bestand staat.
    ///
    /// Als tekst en niet als invoerveld: een schijfnummer geldt voor een hele
    /// schijf, en staat daarom bij de gedeelde velden en bij de hulpacties.
    pub disc: String,

    /// Naar het bewerkformulier van dit ene bestand.
    pub edit_url: String,
}

impl Row {
    /// Het invoerveld van dit veld in deze rij.
    ///
    /// Het template loopt over [`Row::inputs`] en heeft deze weg niet nodig;
    /// een test wel, want die wijst juist één bepaalde kolom aan.
    #[cfg(test)]
    pub fn input(&self, field: RowField) -> &RowInput {
        &self.inputs[field.index()]
    }

    /// Of er iets in deze rij is ingetikt.
    pub fn is_overridden(&self) -> bool {
        self.inputs
            .iter()
            .any(|input| !input.value.trim().is_empty())
    }

    /// Wat er aan de invoer van deze rij mankeert.
    ///
    /// Een fout blijft bij de rij waar hij gemaakt is: de andere rijen en de
    /// gedeelde velden blijven gewoon opgeslagen kunnen worden.
    pub fn problems(&self) -> Vec<String> {
        self.inputs
            .iter()
            .filter_map(|input| input.problem.clone())
            .collect()
    }

    /// Of deze rij zo niet opgeslagen kan worden.
    pub fn has_problems(&self) -> bool {
        self.inputs.iter().any(|input| input.problem.is_some())
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

    /// Pad relatief aan `MUSIC_ROOT`; het handvat waarmee er straks geschreven
    /// wordt.
    pub path: String,

    /// Per veld uit het tagmodel wat ermee gebeurt. Wat ongemoeid blijft, staat
    /// er niet in.
    pub fields: BTreeMap<&'static str, Intent>,
}

impl FileIntent {
    /// Het tagmodel zoals het na het opslaan hoort te zijn.
    ///
    /// Begint bij wat er nú in het bestand staat en zet daar de voornemens
    /// overheen; velden waar de batch niets over zegt, blijven dus zoals ze
    /// zijn. Het normaliseren doet [`Tags::normalized`], zodat leeg overal
    /// hetzelfde betekent: die tag hoort er niet te staan.
    pub fn wanted(&self, current: &Tags) -> Result<Tags, String> {
        let mut wanted = current.clone();

        for (field, intent) in &self.fields {
            let value = match intent {
                Intent::Set(value) => Some(value.as_str()),
                Intent::Clear => None,
                Intent::Unchanged => continue,
            };

            set_field(&mut wanted, field, value)?;
        }

        Ok(wanted.normalized())
    }
}

/// Zet één veld van het tagmodel op een waarde uit het formulier.
///
/// De numerieke velden worden hier gecontroleerd, met dezelfde melding als het
/// bewerkformulier van één bestand.
fn set_field(tags: &mut Tags, field: &str, value: Option<&str>) -> Result<(), String> {
    let text = || value.map(str::to_string);

    match field {
        "title" => tags.title = text(),
        "artist" => tags.artist = text(),
        "album_artist" => tags.album_artist = text(),
        "album" => tags.album = text(),
        "year" => tags.year = text(),
        "genre" => tags.genre = text(),
        "track" => tags.track = number(value, RowField::Track.label())?,
        "disc" => tags.disc = number(value, SharedField::Disc.label())?,
        "disc_total" => tags.disc_total = number(value, SharedField::DiscTotal.label())?,
        other => {
            // De sleutels komen uit `SharedField` en `RowField`; iets anders
            // kan hier niet binnenkomen. Stil negeren zou een schrijffout
            // onzichtbaar maken, en dat is precies wat een batch niet mag.
            return Err(format!("Sleeve kent het veld “{other}” niet."));
        }
    }

    Ok(())
}

/// Leest een getal uit een formulierwaarde; leeg blijft leeg.
fn number(value: Option<&str>, label: &str) -> Result<Option<u32>, String> {
    match value {
        Some(raw) => edit::parse_number(raw, label),
        None => Ok(None),
    }
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
    let (form, _) = form.applied(listing, &chosen_tracks(listing, &selected));

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
                path: track.path.clone(),
                fields,
            })
        })
        .collect()
}

/// Het plan, aangevuld met de aangevinkte bestanden die geen tagwijziging
/// krijgen.
///
/// [`intents`] laat een bestand vallen zodra er niets aan zijn velden verandert
/// — terecht, want dan valt er niets te schrijven. Gaat er een hoes mee, dan
/// ligt dat anders: die geldt voor de hele selectie, ook voor een bestand
/// waarvan de tags al kloppen. De volgorde van de map blijft behouden, zodat
/// het rapport leest als de lijst op het scherm.
pub fn intents_with_selection(listing: &Listing, form: &Form) -> Vec<FileIntent> {
    let plan = intents(listing, form);
    let selected = resolve_selection(listing, form);

    listing
        .tracks
        .iter()
        .filter(|track| selected.contains(&track.name))
        .map(
            |track| match plan.iter().find(|file| file.name == track.name) {
                Some(intent) => intent.clone(),
                None => FileIntent {
                    name: track.name.clone(),
                    path: track.path.clone(),
                    fields: BTreeMap::new(),
                },
            },
        )
        .collect()
}

/// De velden die een batch kan aanraken, in de volgorde van de tabel.
///
/// Afgeleid uit [`RowField`] en [`SharedField`], zodat er geen tweede lijst
/// ontstaat die uit de pas kan gaan lopen. Wat in allebei staat — albumartiest,
/// album, jaar en genre — hoort er maar één keer in.
fn touched_fields() -> Vec<&'static str> {
    let mut fields: Vec<&'static str> = RowField::ALL
        .into_iter()
        .map(RowField::field_name)
        .collect();

    for field in SharedField::ALL {
        if !fields.contains(&field.name()) {
            fields.push(field.name());
        }
    }

    fields
}

/// Het opschrift van een veld uit het tagmodel.
fn label_of(field: &str) -> &'static str {
    RowField::ALL
        .into_iter()
        .find(|row| row.field_name() == field)
        .map(RowField::label)
        .or_else(|| {
            SharedField::ALL
                .into_iter()
                .find(|shared| shared.name() == field)
                .map(SharedField::label)
        })
        .unwrap_or("Veld")
}

/// Wat er in dit veld van dit tagmodel staat.
fn value_of(field: &str, tags: &Tags) -> Option<String> {
    RowField::ALL
        .into_iter()
        .find(|row| row.field_name() == field)
        .map(|row| row.value_of(tags))
        .or_else(|| {
            SharedField::ALL
                .into_iter()
                .find(|shared| shared.name() == field)
                .map(|shared| shared.value_of(tags))
        })
        .flatten()
}

/// Eén veld dat verandert, zoals de voorbeeldweergave het toont (FR-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldChange {
    pub label: String,

    /// Wat er nu in het bestand staat; een streepje wanneer de tag ontbreekt.
    pub before: String,

    /// Wat er komt te staan; leeg wanneer het veld verdwijnt.
    pub after: String,

    /// Of dit een verwijdering is.
    ///
    /// Expliciet en niet af te leiden uit een lege `after`: een veld dat
    /// verdwijnt is de ingrijpendste wijziging die een batch kan maken, en
    /// hoort als zodanig op het scherm te staan (AC #2).
    pub removed: bool,
}

/// Wat er met één bestand gebeurt, zoals de voorbeeldweergave het toont.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub name: String,

    /// Pad relatief aan `MUSIC_ROOT`.
    pub path: String,

    /// De velden die veranderen; leeg wanneer dit bestand ongemoeid blijft.
    pub changes: Vec<FieldChange>,

    /// Waarom dit bestand niet opgeslagen kan worden; leeg wanneer alles klopt.
    pub problem: Option<String>,

    /// Of dit bestand aangevinkt stond.
    ///
    /// Bij de tags is dat af te leiden uit `changes`, maar een hoes geldt voor
    /// de hele selectie — ook voor een bestand waaraan verder niets verandert.
    pub selected: bool,

    /// Wat er nu aan hoes in het bestand zit, kort beschreven; `None` als er
    /// geen is.
    ///
    /// Genoeg om te weten of een meegestuurde hoes iets toevoegt of iets
    /// vervangt, en wát er dan vervangen wordt.
    pub art: Option<String>,
}

impl FileDiff {
    /// Of er aan dit bestand iets verandert.
    pub fn changes_anything(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Wat een meegestuurde hoes met dit bestand zou doen.
    ///
    /// Alleen voor de aangevinkte bestanden, en alleen als tekst: deze module
    /// opent geen bestanden en schrijft niets.
    pub fn art_effect(&self) -> Option<String> {
        if !self.selected {
            return None;
        }

        Some(match &self.art {
            Some(huidige) => format!("hoes wordt vervangen (nu {huidige})"),
            None => "hoes wordt toegevoegd".to_string(),
        })
    }
}

/// De velden die verschillen tussen wat er staat en wat er komt te staan.
///
/// Zowel de voorbeeldweergave als het resultaatoverzicht kijkt hiernaar, zodat
/// er achteraf hetzelfde over een bestand gezegd wordt als er vooraf beloofd is.
pub fn changes_between(current: &Tags, wanted: &Tags) -> Vec<FieldChange> {
    touched_fields()
        .into_iter()
        .filter_map(|field| field_change(field, current, wanted))
        .collect()
}

/// Eén veldwijziging, of niets wanneer het veld hetzelfde blijft.
fn field_change(field: &str, current: &Tags, wanted: &Tags) -> Option<FieldChange> {
    let before = value_of(field, current);
    let after = value_of(field, wanted);

    if before == after {
        return None;
    }

    Some(FieldChange {
        label: label_of(field).to_string(),
        before: before.unwrap_or_else(|| EMPTY.to_string()),
        after: after.clone().unwrap_or_default(),
        removed: after.is_none(),
    })
}

/// Alles wat de voorbeeldweergave nodig heeft (FR-11).
///
/// Dit is de enige route waarlangs een batch wordt weggeschreven: wie hier niet
/// langs is geweest, heeft niet gezien wat er gaat gebeuren.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// Naam van de map.
    pub name: String,

    /// Tot en met deze map.
    pub crumbs: Vec<Crumb>,

    /// Waar het formulier naartoe post; ook de URL van de albumpagina.
    pub url: String,

    /// Terug naar de gewone mapweergave.
    pub back_url: String,

    /// Elk bestand in de map, met wat het krijgt. Bestanden zonder wijziging
    /// staan er ook in: dat er níéts met ze gebeurt, is de helft van wat een
    /// voorbeeld moet vertellen (AC #3).
    pub files: Vec<FileDiff>,

    /// Het formulier waar dit voorbeeld op gebaseerd is, als verborgen velden.
    ///
    /// Zo gaat precies dezelfde invoer mee naar het opslaan, en kan er niets
    /// anders geschreven worden dan wat hier te zien is.
    pub hidden: Vec<HiddenField>,

    /// Wat het opslaan tegenhoudt; leeg wanneer alles klopt.
    pub problems: Vec<String>,

    /// De bovengrens aan een upload in megabytes, uit `MAX_UPLOAD_MB`.
    ///
    /// Deze pagina is de enige stap waarin een hoes meereist, dus ook de enige
    /// die de grens nodig heeft om hem in de browser te kunnen afdwingen.
    pub max_upload_mb: u32,

    /// Wat er nu als losse hoes in de map staat; `None` als er geen is.
    ///
    /// Bepaalt of er om bevestiging gevraagd wordt voordat er iets overheen
    /// gaat — dezelfde regel als op de hoespagina (FR-14).
    pub folder_cover: Option<String>,
}

impl Preview {
    /// Hoeveel bestanden er aangevinkt staan.
    ///
    /// Bepaalt of er iets te doen valt: ook zonder tagwijziging kan er een hoes
    /// in die bestanden gezet worden, en dan hoort de opslaanknop er te staan.
    pub fn selected(&self) -> usize {
        self.files.iter().filter(|file| file.selected).count()
    }

    /// Hoeveel bestanden er veranderen.
    pub fn changing(&self) -> usize {
        self.files
            .iter()
            .filter(|file| file.changes_anything())
            .count()
    }

    /// Of er iets te doen valt.
    pub fn changes_anything(&self) -> bool {
        self.changing() > 0
    }

    /// Hoeveel bestanden er ongemoeid blijven.
    pub fn unchanged(&self) -> usize {
        self.files.len() - self.changing()
    }

    /// Wat er in één zin gaat gebeuren.
    pub fn summary(&self) -> String {
        match self.changing() {
            0 => "Er verandert niets; er valt dus niets op te slaan.".to_string(),
            1 => "1 bestand wordt gewijzigd.".to_string(),
            count => format!("{count} bestanden worden gewijzigd."),
        }
    }
}

/// Eén verborgen veld dat de formulierstaat meedraagt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenField {
    pub name: String,
    pub value: String,
}

/// Wat er per bestand zou veranderen, gerekend vanuit wat er nú in staat.
///
/// Dit is de enige plek waar dat wordt uitgerekend. De voorbeeldweergave zet
/// er de velden bij; de balk onder de albumweergave telt er alleen de
/// veranderende bestanden uit. Zo kunnen die twee niet uiteenlopen: wat de balk
/// belooft, is precies wat het voorbeeld erna laat zien.
///
/// Een ingevulde waarde die gelijk is aan wat er al staat, levert geen
/// wijziging op — dat volgt uit [`changes_between`] en hoeft hier niet apart
/// geregeld te worden.
///
/// Er gaat geen bestand open: de tags zitten al in de listing.
fn diffs(listing: &Listing, form: &Form) -> Vec<FileDiff> {
    let plan = intents(listing, form);
    let chosen = resolve_selection(listing, form);

    listing
        .tracks
        .iter()
        .map(|track| {
            let intent = plan.iter().find(|file| file.name == track.name);

            let (changes, problem) = match intent {
                Some(intent) => match intent.wanted(&track.tags) {
                    Ok(wanted) => (changes_between(&track.tags, &wanted), None),
                    Err(problem) => (Vec::new(), Some(problem)),
                },
                None => (Vec::new(), None),
            };

            FileDiff {
                name: track.name.clone(),
                path: track.path.clone(),
                changes,
                problem,
                selected: chosen.contains(&track.name),
                // De beschrijving komt van `cover::`, want dat is de module die
                // van een `ArtInfo` tekst maakt. Deze module opent zelf geen
                // bestand en rekent niets aan pixels uit.
                art: track.art.as_ref().map(|art| {
                    let details = crate::cover::CoverDetails::of(art);
                    format!("{} {}×{}", details.format, details.width, details.height)
                }),
            }
        })
        .collect()
}

/// Bouwt de voorbeeldweergave: per bestand wat er verandert (FR-11).
///
/// Er gaat hier geen bestand open. De huidige waarden komen uit de listing, en
/// wat eruit komt is een voorstel: het schrijven gebeurt pas als de gebruiker
/// het voorbeeld heeft gezien en op opslaan drukt.
pub fn preview(listing: &Listing, form: &Form) -> Preview {
    let page = album(listing, form);
    let files = diffs(listing, form);

    // Wat de invoer tegenhoudt, staat op de albumpagina al per veld en per rij;
    // hier wordt het herhaald zodat de knop "Definitief opslaan" nooit boven een
    // half plan staat.
    let mut problems = page.problems.clone();
    problems.extend(
        page.rows
            .iter()
            .filter(|row| row.selected)
            .flat_map(Row::problems),
    );
    problems.extend(files.iter().filter_map(|file| file.problem.clone()));

    Preview {
        name: listing.name.clone(),
        crumbs: listing.crumbs.clone(),
        url: browse::album_url(&listing.path),
        back_url: listing.url.clone(),
        files,
        hidden: hidden_fields(listing, form),
        problems,
        // Worden door de webhandler ingevuld: die kent de configuratie en weet
        // wat er in de map staat. Deze module opent geen bestanden.
        max_upload_mb: 0,
        folder_cover: None,
    }
}

/// De formulierstaat als verborgen velden, klaar om mee te gaan naar het
/// opslaan.
///
/// Het is de staat ná een eventuele hulpactie: het voorbeeld toont wat er in de
/// velden stond, en precies dat hoort mee te gaan.
fn hidden_fields(listing: &Listing, form: &Form) -> Vec<HiddenField> {
    let selected = resolve_selection(listing, form);
    let (form, _) = form.applied(listing, &chosen_tracks(listing, &selected));

    let mut fields: Vec<HiddenField> = selected
        .iter()
        .map(|name| HiddenField {
            name: "bestand".to_string(),
            value: name.clone(),
        })
        .collect();

    for field in SharedField::ALL {
        if form.is_cleared(field) {
            fields.push(HiddenField {
                name: field.clear_name(),
                value: "aan".to_string(),
            });
        }

        let value = form.value(field);
        if !value.trim().is_empty() {
            fields.push(HiddenField {
                name: field.name().to_string(),
                value: value.to_string(),
            });
        }
    }

    for track in &listing.tracks {
        for field in RowField::ALL {
            let value = form.override_value(&track.name, field);
            if !value.trim().is_empty() {
                fields.push(HiddenField {
                    name: field.input_name(&track.name),
                    value: value.to_string(),
                });
            }
        }
    }

    fields
}

/// Hoe het opslaan van één bestand is afgelopen (FR-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Het bestand is bijgewerkt; dit zijn de velden die veranderd zijn.
    Saved(Vec<String>),

    /// Er viel niets te wijzigen; het bestand is niet aangeraakt.
    Unchanged,

    /// Er is niets geschreven, en het bestand is onveranderd gebleven.
    Failed(String),
}

/// Wat er met één bestand gebeurd is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveResult {
    pub name: String,
    pub outcome: Outcome,
}

impl SaveResult {
    /// Of dit bestand is bijgewerkt.
    pub fn is_saved(&self) -> bool {
        matches!(self.outcome, Outcome::Saved(_))
    }

    /// Of dit bestand niet opgeslagen kon worden.
    pub fn is_failed(&self) -> bool {
        matches!(self.outcome, Outcome::Failed(_))
    }

    /// Wat er over dit bestand te melden is.
    pub fn describe(&self) -> String {
        match &self.outcome {
            Outcome::Saved(fields) => format!("Bijgewerkt: {}.", fields.join(", ")),
            Outcome::Unchanged => "Er viel niets te wijzigen; niet aangeraakt.".to_string(),
            Outcome::Failed(reason) => {
                format!("Niet opgeslagen: {reason} Het bestand is onveranderd gebleven.")
            }
        }
    }
}

/// Hoe de hele batch is afgelopen.
///
/// Bestand voor bestand, en een fout bij het ene bestand houdt het andere niet
/// tegen — dat is de regel uit FR-11, en dit rapport is waar hij zichtbaar
/// wordt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveReport {
    pub results: Vec<SaveResult>,
}

impl SaveReport {
    pub fn saved(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.is_saved())
            .count()
    }

    pub fn failed(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.is_failed())
            .count()
    }

    /// Of er iets misgegaan is; bepaalt de opmaak van de melding.
    pub fn has_failures(&self) -> bool {
        self.failed() > 0
    }

    /// De kop boven het overzicht.
    pub fn summary(&self) -> String {
        let saved = match self.saved() {
            0 => "Er is geen bestand bijgewerkt".to_string(),
            1 => "1 bestand bijgewerkt".to_string(),
            count => format!("{count} bestanden bijgewerkt"),
        };

        match self.failed() {
            0 => format!("{saved}."),
            1 => format!("{saved}; 1 bestand is niet opgeslagen."),
            count => format!("{saved}; {count} bestanden zijn niet opgeslagen."),
        }
    }
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

    /// De koppen van de invulbare kolommen, in dezelfde volgorde als de velden
    /// in elke rij.
    ///
    /// Afgeleid uit [`RowField::ALL`], zodat kop en cel niet uit de pas kunnen
    /// gaan lopen wanneer er een kolom bij komt.
    pub columns: Vec<String>,

    /// De gedeelde velden, in vaste volgorde.
    pub fields: Vec<SharedInput>,

    /// Het discnummer dat "deze schijf nummer N geven" zou voorstellen.
    ///
    /// Staat in het opschrift van die knop: wat er gaat gebeuren hoort te lezen
    /// te zijn voordat er geklikt wordt (AC #2).
    pub disc_suggestion: u32,

    /// Hoeveel bestanden er geselecteerd zijn.
    pub selected: usize,

    /// Hoeveel bestanden de map bevat.
    pub total: usize,

    /// Wat er aan de invoer van de gedeelde velden mankeert; leeg wanneer alles
    /// klopt. Wat er aan een rij mankeert, staat bij die rij.
    pub problems: Vec<String>,

    /// Hoeveel geselecteerde rijen een eigen waarde uit de tabel gekregen
    /// hebben.
    pub overridden: usize,

    /// Hoeveel bestanden er bij het opslaan daadwerkelijk zouden veranderen.
    ///
    /// Komt uit [`diffs`] — dezelfde berekening als de voorbeeldweergave — en
    /// wordt geteld zoals [`Preview::changing`] telt. Een ingevulde waarde die
    /// gelijk is aan wat er al in het bestand staat, telt dus niet mee: dat
    /// bestand verandert niet.
    pub changed_files: usize,

    /// Wat de zojuist aangeklikte hulpactie gedaan heeft (FR-10); leeg wanneer
    /// er geen hulpactie is gebruikt.
    pub helper_notice: Option<String>,

    /// Hoe een zojuist uitgevoerde batch is afgelopen (FR-11); leeg zolang er
    /// niets is opgeslagen.
    pub report: Option<SaveReport>,
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

    /// Of er iets openstaat: een selectie met invoer die nog nergens heen is.
    ///
    /// Niet hetzelfde als "er verandert iets". Wie invult wat er al staat, heeft
    /// nog steeds iets openstaan: de voorbeeldweergave blijft bereikbaar, want
    /// daar hangt ook de hoes aan, die ook een bestand met kloppende tags nog
    /// iets te geven heeft. Hoeveel bestanden er werkelijk veranderen, staat als
    /// getal in de balk — en dat mag nul zijn.
    pub fn is_pending(&self) -> bool {
        self.has_selection() && self.changes_anything()
    }

    /// Wat de balk onder de albumweergave zegt.
    ///
    /// Hetzelfde getal als de voorbeeldweergave straks toont, want het komt uit
    /// dezelfde berekening; hier alleen geteld in plaats van uitgeschreven.
    pub fn pending_summary(&self) -> String {
        if !self.has_selection() {
            return "Er is niets geselecteerd, dus er staat niets open.".to_string();
        }

        if !self.changes_anything() {
            return "Er is nog niets ingevuld, dus er staat niets open.".to_string();
        }

        match self.changed_files {
            // Het geval dat pas in de voorbeeldweergave zichtbaar was: wat er
            // is ingevuld, staat er al.
            0 => "Geen enkel bestand krijgt een wijziging: wat er is ingevuld, staat er al."
                .to_string(),
            1 => "1 bestand krijgt een wijziging.".to_string(),
            count => format!("{count} bestanden krijgen een wijziging."),
        }
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
    let (form, helper_notice) = form.applied(listing, &chosen);
    let form = &form;

    let rows: Vec<Row> = listing
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| Row {
            // De koppen staan tussen de rijen; het weergavemodel bepaalt waar
            // een groep begint, deze pagina alleen wat de knop ernaast doet.
            group: listing
                .group_starting_at(index)
                .map(|group| GroupHeading::of(group, listing, &selected)),
            selected: selected.contains(&track.name),
            inputs: RowField::ALL
                .into_iter()
                .map(|field| RowInput {
                    name: field.input_name(&track.name),
                    label: format!("{} van {}", field.label(), track.name),
                    value: form.override_value(&track.name, field).to_string(),
                    placeholder: field
                        .value_of(&track.tags)
                        .unwrap_or_else(|| EMPTY.to_string()),
                    numeric: field.is_numeric(),
                    size: field.size().to_string(),
                    problem: form.row_intent(&track.name, field).err(),
                })
                .collect(),
            name: track.name.clone(),
            disc: track
                .tags
                .disc
                .map(|number| number.to_string())
                .unwrap_or_else(|| EMPTY.to_string()),
            // Met de herkomst erbij: wie hiervandaan een bestand bewerkt, wil
            // terug naar deze weergave en niet naar de kale maplijst.
            edit_url: crate::browse::edit_url_from_album(&track.path),
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
        columns: RowField::ALL
            .into_iter()
            .map(|field| field.column().to_string())
            .collect(),
        fields,
        disc_suggestion: disc_suggestion(listing, &chosen),
        problems,
        overridden,
        // Van dezelfde berekening als de voorbeeldweergave, en op dezelfde
        // manier geteld: de balk onder het formulier en het voorbeeld erna
        // kunnen zo niet uiteenlopen.
        changed_files: diffs(listing, form)
            .iter()
            .filter(|file| file.changes_anything())
            .count(),
        helper_notice,
        report: None,
    }
}

/// Welke bestanden er geselecteerd zijn, na het toepassen van de actie.
///
/// Een naam die niet (meer) in de map staat, verdwijnt vanzelf: er wordt alleen
/// tegen de bestanden uit de listing aan gekeken.
///
/// "Disctotalen invullen" selecteert alles, en dat is geen uitzondering op de
/// regel eronder maar de actie zelf: het aantal schijven van een set hoort in
/// elk bestand van die set te staan, en niet in het deel dat toevallig
/// aangevinkt stond.
fn resolve_selection(listing: &Listing, form: &Form) -> BTreeSet<String> {
    match form.action {
        Action::All | Action::FillDiscTotals => listing
            .tracks
            .iter()
            .map(|track| track.name.clone())
            .collect(),
        Action::None => BTreeSet::new(),
        Action::ToggleGroup(disc) => toggle_group(listing, form, disc),
        // Een hulpactie laat de selectie met rust: die vult alleen velden.
        _ => form.selected.clone(),
    }
}

/// De selectie nadat er op de knop van één schijf is geklikt (AC #4).
///
/// Staat de hele groep al aan, dan gaat hij eraf; anders gaat hij erbij. Wat
/// er buiten de groep aanstond, blijft in beide gevallen staan: deze knop gaat
/// over deze schijf, en over de rest van de map heeft de gebruiker al beslist.
fn toggle_group(listing: &Listing, form: &Form, disc: Option<u32>) -> BTreeSet<String> {
    let mut selection = form.selected.clone();

    let group: Vec<&String> = listing
        .tracks
        .iter()
        .filter(|track| track.tags.disc == disc)
        .map(|track| &track.name)
        .collect();

    let complete = !group.is_empty() && group.iter().all(|name| selection.contains(*name));

    for name in group {
        if complete {
            selection.remove(name);
        } else {
            selection.insert(name.clone());
        }
    }

    selection
}

/// De geselecteerde bestanden, in de volgorde van de tabel.
fn chosen_tracks<'a>(listing: &'a Listing, selected: &BTreeSet<String>) -> Vec<&'a TrackSummary> {
    listing
        .tracks
        .iter()
        .filter(|track| selected.contains(&track.name))
        .collect()
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
            foreign_tags: Vec::new(),
            duration: "0:00".to_string(),
            format: "MP3".to_string(),
            art: None,
            art_url: String::new(),
            edit_url: format!("/bewerk/Album/{name}"),
        }
    }

    fn listing_of(tracks: Vec<TrackSummary>) -> Listing {
        Listing {
            // Dezelfde groepering als in een echte listing: de tracks staan
            // hier al op schijf gesorteerd, net zoals `browse::listing` ze
            // aanlevert.
            groups: browse::disc_groups(&tracks),
            name: "Album".to_string(),
            path: "Artiest/Album".to_string(),
            url: "/map/Artiest/Album".to_string(),
            album_url: "/album/Artiest/Album".to_string(),
            crumbs: Vec::new(),
            folders: Vec::new(),
            tracks,
            folder_issues: Vec::new(),
            query: String::new(),
            flagged_count: 0,
            only_flagged: false,
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
        // AC #1: elk veld dat per bestand kan verschillen, is per rij in te
        // tikken.
        let page = album(&album_with_two_albums(), &Form::select_all());
        let row = row_of(&page, "een.mp3");

        assert_eq!(row.inputs.len(), RowField::ALL.len());
        assert_eq!(row.input(RowField::Track).name, "nummer:een.mp3");
        assert_eq!(row.input(RowField::Title).name, "titel:een.mp3");
        assert_eq!(row.input(RowField::Artist).name, "artiest:een.mp3");
        assert_eq!(row.input(RowField::Album).name, "albumtitel:een.mp3");
        assert_eq!(row.input(RowField::Year).name, "jaar:een.mp3");
        assert_eq!(row.input(RowField::Genre).name, "genre:een.mp3");

        // Niets voorgevuld: leeg betekent hier hetzelfde als bij een gedeeld
        // veld, namelijk ongemoeid laten. Wat er in het bestand staat, staat
        // als grijze tekst in het veld.
        assert!(row.inputs.iter().all(|input| input.value.is_empty()));
        assert_eq!(row.input(RowField::Album).placeholder, "Eerste");
        assert_eq!(row.input(RowField::Genre).placeholder, EMPTY);
        assert!(!row.is_overridden());

        // En de koppen komen uit hetzelfde lijstje als de velden.
        assert_eq!(page.columns.len(), RowField::ALL.len());
        assert_eq!(page.columns[0], "#");
        assert_eq!(page.columns[1], "Titel");
    }

    #[test]
    fn artist_album_year_and_genre_are_typed_in_the_row_itself() {
        // AC #1 en #2: de vier nieuwe kolommen lopen door dezelfde keten als
        // titel en tracknummer, en gelden voor dat ene bestand.
        let form = Form::parse(
            "actie=alles&artiest:een.mp3=Een+Ander&albumtitel:een.mp3=Eigen+album\
             &jaar:een.mp3=1999&genre:een.mp3=Jazz",
        );
        let page = album(&album_with_two_albums(), &form);

        let row = row_of(&page, "een.mp3");
        assert_eq!(row.input(RowField::Artist).value, "Een Ander");
        assert_eq!(row.input(RowField::Album).value, "Eigen album");
        assert_eq!(row.input(RowField::Year).value, "1999");
        assert_eq!(row.input(RowField::Genre).value, "Jazz");
        assert!(row.is_overridden());
        assert_eq!(page.overridden, 1);

        // Alleen dit ene bestand; de rest van de tabel blijft leeg.
        assert!(!row_of(&page, "twee.mp3").is_overridden());

        let plan = intents(&album_with_two_albums(), &form);
        let first = plan
            .iter()
            .find(|file| file.name == "een.mp3")
            .expect("het bestand hoort in het plan te staan");

        assert_eq!(
            first.fields.get("artist"),
            Some(&Intent::Set("Een Ander".to_string()))
        );
        assert_eq!(
            first.fields.get("album"),
            Some(&Intent::Set("Eigen album".to_string()))
        );
        assert_eq!(
            first.fields.get("year"),
            Some(&Intent::Set("1999".to_string()))
        );
        assert_eq!(
            first.fields.get("genre"),
            Some(&Intent::Set("Jazz".to_string()))
        );
    }

    #[test]
    fn a_row_field_beats_the_shared_field_of_the_same_name() {
        // AC #2: album, jaar en genre staan in beide, en dan wint de rij — maar
        // alleen voor het bestand waar iets is ingetikt.
        let form = Form::parse(
            "actie=alles&album=Gedeeld+album&year=2001&genre=Ambient\
             &albumtitel:een.mp3=Eigen+album&jaar:een.mp3=1999",
        );
        let plan = intents(&album_with_two_albums(), &form);

        let first = plan
            .iter()
            .find(|file| file.name == "een.mp3")
            .expect("het bestand hoort in het plan te staan");
        assert_eq!(
            first.fields.get("album"),
            Some(&Intent::Set("Eigen album".to_string()))
        );
        assert_eq!(
            first.fields.get("year"),
            Some(&Intent::Set("1999".to_string()))
        );
        // Waar de rij niets zegt, geldt het gedeelde veld gewoon.
        assert_eq!(
            first.fields.get("genre"),
            Some(&Intent::Set("Ambient".to_string()))
        );

        let second = plan
            .iter()
            .find(|file| file.name == "twee.mp3")
            .expect("het bestand hoort in het plan te staan");
        assert_eq!(
            second.fields.get("album"),
            Some(&Intent::Set("Gedeeld album".to_string()))
        );
        assert_eq!(
            second.fields.get("year"),
            Some(&Intent::Set("2001".to_string()))
        );
    }

    #[test]
    fn a_row_field_beats_a_shared_field_that_is_being_cleared() {
        // AC #2: ook het wissen-vinkje is iets wat de gedeelde velden voor dit
        // bestand zouden doen, en dus wint de rij er ook van.
        let form = Form::parse("actie=alles&wis_genre=aan&genre:een.mp3=Jazz");
        let plan = intents(&album_with_two_albums(), &form);

        let first = plan
            .iter()
            .find(|file| file.name == "een.mp3")
            .expect("het bestand hoort in het plan te staan");
        assert_eq!(
            first.fields.get("genre"),
            Some(&Intent::Set("Jazz".to_string()))
        );

        let second = plan
            .iter()
            .find(|file| file.name == "twee.mp3")
            .expect("het bestand hoort in het plan te staan");
        assert_eq!(second.fields.get("genre"), Some(&Intent::Clear));
    }

    #[test]
    fn a_year_may_be_a_full_date_because_the_tag_model_allows_one() {
        // Het jaar is in het tagmodel tekst: ID3v2.4 en Vorbis kunnen er een
        // volledige datum in zetten. Er een getal van eisen zou een bestaande
        // waarde onbewerkbaar maken.
        let form = Form::parse("actie=alles&jaar:een.mp3=2024-05-01");

        assert!(!RowField::Year.is_numeric());
        assert_eq!(
            form.row_intent("een.mp3", RowField::Year),
            Ok(Intent::Set("2024-05-01".to_string()))
        );

        let page = album(&album_with_two_albums(), &form);
        assert!(!row_of(&page, "een.mp3").has_problems());
    }

    #[test]
    fn typed_overrides_survive_a_change_of_selection() {
        // AC #2: de selectie of de gedeelde velden aanpassen mag de tabel niet
        // leegvegen.
        let form = Form::parse("actie=niets&titel:een.mp3=Nieuwe+titel&album=Nieuw+album");
        let page = album(&album_with_two_albums(), &form);

        let row = row_of(&page, "een.mp3");
        assert_eq!(row.input(RowField::Title).value, "Nieuwe titel");
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
        // AC #4: één typefout mag de rest van de tabel niet ophouden. De rij
        // met de fout heeft ook goede velden; ook die gaan mee de fout in,
        // want half uitvoeren van een plan voor één bestand is erger dan niets
        // doen.
        let form = Form::parse(
            "actie=alles&nummer:een.mp3=drie&genre:een.mp3=Jazz&titel:twee.mp3=Wel+dit",
        );
        let page = album(&album_with_two_albums(), &form);

        let broken = row_of(&page, "een.mp3");
        assert!(broken.has_problems());
        assert!(
            broken.problems()[0].starts_with("Tracknummer"),
            "{:?}",
            broken
        );
        // De melding staat bij het veld waarin hij is ingetikt, en niet bij de
        // zes andere kolommen van dezelfde rij.
        assert!(broken.input(RowField::Track).problem.is_some());
        assert!(broken.input(RowField::Genre).problem.is_none());

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

    #[test]
    fn the_bar_says_nothing_is_pending_until_something_is_filled_in() {
        // AC #2: zonder invoer valt er niets voor te bereiden, en dan is er ook
        // niets aan te klikken.
        let empty = album(&album_with_two_albums(), &Form::select_all());

        assert!(!empty.is_pending());
        assert_eq!(
            empty.pending_summary(),
            "Er is nog niets ingevuld, dus er staat niets open."
        );

        let nothing_chosen = album(&album_with_two_albums(), &Form::parse("album=Nieuw"));

        assert!(!nothing_chosen.is_pending());
        assert_eq!(
            nothing_chosen.pending_summary(),
            "Er is niets geselecteerd, dus er staat niets open."
        );
    }

    #[test]
    fn the_bar_counts_the_files_that_get_a_change() {
        // AC #1: het aantal staat er terwijl je bezig bent, en niet pas in de
        // voorbeeldweergave.
        let listing = album_with_two_albums();
        let form = Form::parse("actie=alles&album=Nachtmuziek");
        let page = album(&listing, &form);

        assert!(page.is_pending());
        assert_eq!(page.changed_files, 3);
        assert_eq!(page.pending_summary(), "3 bestanden krijgen een wijziging.");
    }

    #[test]
    fn a_value_that_is_already_there_changes_no_file() {
        // De reden dat de balk uit de diff telt en niet uit het plan: wie
        // invult wat er al staat, verandert niets — en dat hoort er meteen te
        // staan, niet pas als het voorbeeld leeg blijkt.
        let listing = listing_of(vec![
            track("een.mp3", Some("Eerste"), Some(1)),
            track("twee.mp3", Some("Eerste"), Some(1)),
        ]);
        let form = Form::parse("actie=alles&album=Eerste");
        let page = album(&listing, &form);

        // Er staat wél iets open — het voorbeeld blijft bereikbaar, want daar
        // hangt ook de hoes aan — maar er verandert geen enkel bestand.
        assert!(page.is_pending());
        assert_eq!(page.changed_files, 0);
        assert_eq!(
            page.pending_summary(),
            "Geen enkel bestand krijgt een wijziging: wat er is ingevuld, staat er al."
        );
        assert_eq!(
            page.changed_files_effect(),
            "Er verandert geen enkel bestand."
        );

        // Eén bestand dat wél iets nieuws krijgt, en de telling loopt mee.
        let half = album(
            &listing,
            &Form::parse("actie=alles&album=Eerste&genre=Jazz"),
        );
        assert_eq!(half.changed_files, 2);
        assert_eq!(half.pending_summary(), "2 bestanden krijgen een wijziging.");
    }

    #[test]
    fn the_bar_and_the_preview_never_disagree() {
        // De balk en het voorbeeld komen van dezelfde berekening; deze test
        // houdt vast dat dat zo blijft.
        let listing = album_with_two_albums();

        for body in [
            "actie=alles",
            "actie=alles&album=Eerste",
            "actie=alles&album=Nieuw",
            "actie=alles&wis_genre=aan",
            "actie=alles&titel:een.mp3=E%C3%A9n&nummer:twee.mp3=2",
            "bestand=een.mp3&album=Nieuw",
            // Een rij met een onbruikbaar nummer valt in allebei even hard weg.
            "actie=alles&nummer:een.mp3=twee&album=Nieuw",
        ] {
            let form = Form::parse(body);
            let page = album(&listing, &form);
            let view = preview(&listing, &form);

            assert_eq!(
                page.changed_files,
                view.changing(),
                "balk en voorbeeld lopen uiteen bij “{body}”"
            );
        }
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

        assert_eq!(row_of(&page, "een.mp3").input(RowField::Track).value, "1");
        assert_eq!(row_of(&page, "twee.mp3").input(RowField::Track).value, "2");
        assert_eq!(row_of(&page, "drie.mp3").input(RowField::Track).value, "3");

        let notice = page.helper_notice.expect("de actie hoort iets te melden");
        assert!(notice.contains("1 tot en met 3"), "{notice}");
    }

    #[test]
    fn renumbering_leaves_what_is_not_selected_alone() {
        let form = Form::parse("actie=hernummer&bestand=een.mp3&bestand=drie.mp3");
        let page = album(&album_that_needs_help(), &form);

        assert_eq!(row_of(&page, "een.mp3").input(RowField::Track).value, "1");
        assert_eq!(row_of(&page, "drie.mp3").input(RowField::Track).value, "2");
        assert_eq!(row_of(&page, "twee.mp3").input(RowField::Track).value, "");
    }

    #[test]
    fn copying_the_artist_fills_the_album_artist_per_file() {
        // AC #2: per bestand, want de artiesten hoeven niet gelijk te zijn.
        let page = album(
            &album_that_needs_help(),
            &Form::parse("actie=artiest&bestand=een.mp3&bestand=twee.mp3&bestand=drie.mp3"),
        );

        assert_eq!(
            row_of(&page, "een.mp3").input(RowField::AlbumArtist).value,
            "de testartiest"
        );
        assert_eq!(
            row_of(&page, "twee.mp3").input(RowField::AlbumArtist).value,
            "Een Ander"
        );
        // Zonder artiest valt er niets te kopiëren; een lege albumartiest
        // voorstellen zou een verwijdering zijn.
        assert_eq!(
            row_of(&page, "drie.mp3").input(RowField::AlbumArtist).value,
            ""
        );

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

        assert_eq!(
            row_of(&page, "een.mp3").input(RowField::Title).value,
            "Stilte in D"
        );
        assert_eq!(
            row_of(&page, "twee.mp3").input(RowField::Title).value,
            "Ruis in B"
        );
        // Wat al klopt, krijgt geen voorstel: dat zou geen voorstel zijn.
        assert_eq!(row_of(&page, "drie.mp3").input(RowField::Title).value, "");
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

        assert_eq!(
            row_of(&page, "een.mp3").input(RowField::Title).value,
            "Een Andere Titel"
        );
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

    /// Een set van twee schijven, met één bestand dat nergens bij hoort.
    ///
    /// De tracknummers tellen door over beide schijven — precies de fout die
    /// "hernummeren per schijf" rechtzet.
    ///
    /// Elke hulpactie wordt met een expliciete selectie aangeroepen: `actie`
    /// draagt de hulpactie, dus "alles selecteren" kan er niet naast.
    fn everything_in(listing: &Listing) -> String {
        listing
            .tracks
            .iter()
            .map(|track| format!("&bestand={}", track.name))
            .collect()
    }

    fn set_of_two_discs() -> Listing {
        listing_of(vec![
            track_with(
                "01 - Eerste.mp3",
                Tags {
                    title: Some("Eerste".to_string()),
                    track: Some(1),
                    disc: Some(1),
                    ..Tags::default()
                },
            ),
            track_with(
                "02 - Tweede.mp3",
                Tags {
                    title: Some("Tweede".to_string()),
                    track: Some(2),
                    disc: Some(1),
                    ..Tags::default()
                },
            ),
            track_with(
                "03 - Derde.mp3",
                Tags {
                    title: Some("Derde".to_string()),
                    track: Some(3),
                    disc: Some(2),
                    ..Tags::default()
                },
            ),
            track_with(
                "04 - Vierde.mp3",
                Tags {
                    title: Some("Vierde".to_string()),
                    track: Some(4),
                    ..Tags::default()
                },
            ),
        ])
    }

    #[test]
    fn renumbering_per_disc_starts_every_disc_at_one() {
        // AC #1: elke schijf zijn eigen reeks; doortellen is juist de fout.
        let listing = set_of_two_discs();
        let page = album(
            &listing,
            &Form::parse(&format!("actie=hernummer-disc{}", everything_in(&listing))),
        );

        assert_eq!(
            row_of(&page, "01 - Eerste.mp3")
                .input(RowField::Track)
                .value,
            ""
        );
        assert_eq!(
            row_of(&page, "02 - Tweede.mp3")
                .input(RowField::Track)
                .value,
            ""
        );
        // De tweede schijf begon bij 3 en hoort bij 1 te beginnen.
        assert_eq!(
            row_of(&page, "03 - Derde.mp3").input(RowField::Track).value,
            "1"
        );
        // Zonder discnummer: samen één reeks, dus ook vanaf 1.
        assert_eq!(
            row_of(&page, "04 - Vierde.mp3")
                .input(RowField::Track)
                .value,
            "1"
        );

        let notice = page.helper_notice.expect("de actie hoort iets te melden");
        assert!(notice.contains("3 schijven"), "{notice}");
    }

    #[test]
    fn renumbering_per_disc_leaves_a_number_that_is_already_right_alone() {
        // AC #6: een voorstel dat gelijk is aan wat er staat, is geen voorstel.
        let listing = listing_of(vec![
            track_with(
                "een.mp3",
                Tags {
                    track: Some(1),
                    disc: Some(1),
                    ..Tags::default()
                },
            ),
            track_with(
                "twee.mp3",
                Tags {
                    track: Some(1),
                    disc: Some(2),
                    ..Tags::default()
                },
            ),
        ]);
        let page = album(
            &listing,
            &Form::parse(&format!("actie=hernummer-disc{}", everything_in(&listing))),
        );

        assert!(!page.changes_anything());
        let notice = page.helper_notice.expect("de actie hoort iets te melden");
        assert!(notice.contains("al doorlopend genummerd"), "{notice}");
    }

    #[test]
    fn the_suggested_disc_is_the_one_the_selection_already_sits_on() {
        // AC #2: staat de selectie al op één schijf, dan die.
        let listing = set_of_two_discs();
        let form = Form::parse("bestand=03 - Derde.mp3");
        let page = album(&listing, &form);

        assert_eq!(page.disc_suggestion, 2);
    }

    #[test]
    fn the_suggested_disc_is_the_first_free_one_otherwise() {
        // Schijf 1 en 2 zijn in gebruik in deze map, dus 3 is de eerstvolgende.
        let page = album(&set_of_two_discs(), &Form::select_all());

        assert_eq!(page.disc_suggestion, 3);
    }

    #[test]
    fn setting_the_disc_fills_the_shared_field_with_that_number() {
        let listing = set_of_two_discs();
        let page = album(&listing, &Form::parse("actie=disc&bestand=04 - Vierde.mp3"));

        assert_eq!(field_of(&page, SharedField::Disc).value, "3");

        let notice = page.helper_notice.expect("de actie hoort iets te melden");
        assert!(notice.contains("Schijf 3"), "{notice}");
    }

    #[test]
    fn setting_the_disc_proposes_nothing_when_the_selection_is_already_there() {
        // AC #6, en de reden dat de knop het nummer vooraf toont.
        let listing = set_of_two_discs();
        let form = Form::parse("actie=disc&bestand=01 - Eerste.mp3&bestand=02 - Tweede.mp3");
        let page = album(&listing, &form);

        assert_eq!(field_of(&page, SharedField::Disc).value, "");
        assert!(!page.changes_anything());

        let notice = page.helper_notice.expect("de actie hoort iets te melden");
        assert!(notice.contains("staat al op schijf 1"), "{notice}");
    }

    #[test]
    fn filling_the_disc_totals_covers_the_whole_folder() {
        // AC #3: het totaal hoort in élk bestand van de set te staan.
        let listing = set_of_two_discs();
        let page = album(
            &listing,
            &Form::parse("actie=disctotaal&bestand=01 - Eerste.mp3"),
        );

        assert_eq!(page.selected, 4);
        assert_eq!(field_of(&page, SharedField::DiscTotal).value, "2");
        assert_eq!(page.changed_files, 4);

        let notice = page.helper_notice.expect("de actie hoort iets te melden");
        assert!(notice.contains("2 schijven"), "{notice}");
        // Eén bestand heeft nog geen discnummer, en dat hoort erbij te staan.
        assert!(notice.contains("nog geen discnummer"), "{notice}");
    }

    #[test]
    fn filling_the_disc_totals_proposes_nothing_when_they_are_already_right() {
        // AC #6.
        let listing = listing_of(vec![track_with(
            "een.mp3",
            Tags {
                disc: Some(1),
                disc_total: Some(1),
                ..Tags::default()
            },
        )]);
        let page = album(&listing, &Form::parse("actie=disctotaal"));

        assert_eq!(field_of(&page, SharedField::DiscTotal).value, "");
        assert!(!page.changes_anything());
    }

    #[test]
    fn a_disc_total_reaches_the_plan_as_a_number() {
        let plan = intents(&set_of_two_discs(), &Form::parse("actie=disctotaal"));

        assert_eq!(plan.len(), 4, "{plan:?}");
        assert_eq!(
            plan[0].fields.get("disc_total"),
            Some(&Intent::Set("2".to_string()))
        );

        let wanted = plan[0]
            .wanted(&Tags::default())
            .expect("een getal hoort door de controle te komen");
        assert_eq!(wanted.disc_total, Some(2));
    }

    #[test]
    fn a_title_is_read_from_the_file_name_only_where_there_is_none() {
        // AC #4: een bestaande titel is betrouwbaarder dan een bestandsnaam.
        let listing = listing_of(vec![
            track_with(
                "01 - Kind of Blue.flac",
                Tags {
                    title: Some("Kind of Blue".to_string()),
                    ..Tags::default()
                },
            ),
            track_with("02 - So What.flac", Tags::default()),
            track_with("03.flac", Tags::default()),
        ]);
        let page = album(
            &listing,
            &Form::parse(&format!("actie=titelnaam{}", everything_in(&listing))),
        );

        // Heeft al een titel: niets voorstellen.
        assert_eq!(
            row_of(&page, "01 - Kind of Blue.flac")
                .input(RowField::Title)
                .value,
            ""
        );
        assert_eq!(
            row_of(&page, "02 - So What.flac")
                .input(RowField::Title)
                .value,
            "So What"
        );
        // Uit een naam die alleen een tracknummer is, valt niets te halen.
        assert_eq!(row_of(&page, "03.flac").input(RowField::Title).value, "");

        let notice = page.helper_notice.expect("de actie hoort iets te melden");
        assert!(notice.contains("Bij 1 bestand"), "{notice}");
        assert!(notice.contains("1 met een titel"), "{notice}");
        assert!(notice.contains("Uit 1 bestandsnamen"), "{notice}");
    }

    #[test]
    fn a_title_from_a_file_name_is_a_proposal_and_nothing_more() {
        // AC #5: alleen invoervelden, en met "Invoer leegmaken" weer weg.
        let listing = listing_of(vec![track_with("07 - So What.flac", Tags::default())]);

        let filled = album(
            &listing,
            &Form::parse("actie=titelnaam&bestand=07 - So What.flac"),
        );
        assert!(filled.changes_anything());
        assert_eq!(filled.overridden, 1);

        let restored = album(
            &listing,
            &Form::parse("actie=herstel&bestand=07 - So What.flac&titel:07 - So What.flac=So+What"),
        );
        assert_eq!(restored.selected, 1);
        assert_eq!(restored.overridden, 0);
        assert!(!restored.changes_anything());
    }

    fn file_in<'a>(preview: &'a Preview, name: &str) -> &'a FileDiff {
        preview
            .files
            .iter()
            .find(|file| file.name == name)
            .expect("het bestand hoort in het voorbeeld te staan")
    }

    fn change_of<'a>(file: &'a FileDiff, label: &str) -> &'a FieldChange {
        file.changes
            .iter()
            .find(|change| change.label == label)
            .unwrap_or_else(|| panic!("veld '{label}' hoort te veranderen: {:?}", file.changes))
    }

    #[test]
    fn a_plan_is_applied_on_top_of_what_is_already_there() {
        let form =
            Form::parse("actie=alles&album=Nieuw+album&wis_genre=aan&titel:een.mp3=Nieuwe+titel");
        let plan = intents(&album_that_needs_help(), &form);

        let first = plan
            .iter()
            .find(|file| file.name == "een.mp3")
            .expect("het bestand hoort in het plan te staan");

        let current = Tags {
            title: Some("Oud".to_string()),
            artist: Some("Blijft staan".to_string()),
            genre: Some("Weg hiermee".to_string()),
            composer: Some("Ook ongemoeid".to_string()),
            ..Tags::default()
        };

        let wanted = first.wanted(&current).expect("dit plan hoort te kloppen");

        assert_eq!(wanted.title, Some("Nieuwe titel".to_string()));
        assert_eq!(wanted.album, Some("Nieuw album".to_string()));
        // Wissen betekent verwijderen, en niet een lege waarde opslaan.
        assert_eq!(wanted.genre, None);
        // Waar de batch niets over zegt, blijft staan.
        assert_eq!(wanted.artist, Some("Blijft staan".to_string()));
        assert_eq!(wanted.composer, Some("Ook ongemoeid".to_string()));
    }

    #[test]
    fn a_plan_with_an_impossible_number_is_refused_before_anything_is_written() {
        let mut fields = BTreeMap::new();
        fields.insert("track", Intent::Set("drie".to_string()));

        let intent = FileIntent {
            name: "een.mp3".to_string(),
            path: "Album/een.mp3".to_string(),
            fields,
        };

        let problem = intent
            .wanted(&Tags::default())
            .expect_err("dit hoort een fout te zijn");
        assert!(problem.starts_with("Tracknummer"), "{problem}");
    }

    #[test]
    fn a_change_shows_what_was_there_and_what_comes_instead() {
        let current = Tags {
            album: Some("Oud album".to_string()),
            genre: Some("Weg hiermee".to_string()),
            ..Tags::default()
        };
        let wanted = Tags {
            album: Some("Nieuw album".to_string()),
            title: Some("Erbij".to_string()),
            ..Tags::default()
        };

        let changes = changes_between(&current, &wanted);

        let album = changes
            .iter()
            .find(|change| change.label == "Album")
            .expect("het album verandert");
        assert_eq!(album.before, "Oud album");
        assert_eq!(album.after, "Nieuw album");
        assert!(!album.removed);

        // AC #2: een veld dat verdwijnt is expliciet een verwijdering.
        let genre = changes
            .iter()
            .find(|change| change.label == "Genre")
            .expect("het genre verdwijnt");
        assert!(genre.removed);
        assert_eq!(genre.before, "Weg hiermee");
        assert_eq!(genre.after, "");

        // Een veld dat er nog niet was, is geen verwijdering maar een toevoeging.
        let title = changes
            .iter()
            .find(|change| change.label == "Titel")
            .expect("de titel komt erbij");
        assert_eq!(title.before, EMPTY);
        assert_eq!(title.after, "Erbij");
        assert!(!title.removed);
    }

    #[test]
    fn the_preview_shows_every_file_including_the_ones_that_stay_as_they_are() {
        // AC #1 en #3.
        let form = Form::parse("actie=alles&album=Eerste");
        let view = preview(&album_with_two_albums(), &form);

        assert_eq!(view.files.len(), 3);
        assert_eq!(view.changing(), 2);
        assert_eq!(view.unchanged(), 1);

        // Dit bestand heeft het album al; er is niets te doen.
        assert!(!file_in(&view, "een.mp3").changes_anything());

        let second = file_in(&view, "twee.mp3");
        let change = change_of(second, "Album");
        assert_eq!(change.before, "Tweede");
        assert_eq!(change.after, "Eerste");

        // En het bestand zonder album krijgt er een.
        assert_eq!(change_of(file_in(&view, "drie.mp3"), "Album").before, EMPTY);

        assert!(view.summary().contains("2 bestanden"), "{}", view.summary());
    }

    #[test]
    fn a_file_outside_the_selection_stays_out_of_the_changes() {
        let form = Form::parse("bestand=twee.mp3&album=Nieuw");
        let view = preview(&album_with_two_albums(), &form);

        assert_eq!(view.changing(), 1);
        assert!(file_in(&view, "twee.mp3").changes_anything());
        assert!(!file_in(&view, "een.mp3").changes_anything());
    }

    #[test]
    fn the_preview_carries_the_whole_form_along() {
        // Wat er opgeslagen wordt, mag niet kunnen afwijken van wat er te zien
        // is; daarom gaat de hele formulierstaat verborgen mee.
        let form = Form::parse("bestand=een.mp3&album=Nieuw&wis_genre=aan&titel:een.mp3=Eigen");
        let view = preview(&album_with_two_albums(), &form);

        let carried: Vec<(String, String)> = view
            .hidden
            .iter()
            .map(|field| (field.name.clone(), field.value.clone()))
            .collect();

        for expected in [
            ("bestand", "een.mp3"),
            ("album", "Nieuw"),
            ("wis_genre", "aan"),
            ("titel:een.mp3", "Eigen"),
        ] {
            let expected = (expected.0.to_string(), expected.1.to_string());
            assert!(carried.contains(&expected), "{expected:?} in {carried:?}");
        }
    }

    #[test]
    fn a_helper_action_is_carried_along_as_the_values_it_filled_in() {
        // Het voorbeeld toont het voorstel; precies dat hoort mee te gaan, en
        // niet de knop die het maakte.
        let form = Form::parse("actie=hernummer&bestand=een.mp3&bestand=twee.mp3");
        let view = preview(&album_that_needs_help(), &form);

        let carried: Vec<&str> = view
            .hidden
            .iter()
            .filter(|field| field.name == "nummer:een.mp3")
            .map(|field| field.value.as_str())
            .collect();
        assert_eq!(carried, vec!["1"]);

        assert!(
            !view.hidden.iter().any(|field| field.name == "actie"),
            "de hulpactie zelf hoort niet mee te gaan: {:?}",
            view.hidden
        );
    }

    #[test]
    fn a_preview_with_a_broken_field_says_what_is_wrong() {
        let form = Form::parse("actie=alles&disc=twee");
        let view = preview(&album_with_two_albums(), &form);

        assert_eq!(view.problems.len(), 1, "{:?}", view.problems);
        assert!(view.problems[0].starts_with("Discnummer"));
    }

    #[test]
    fn a_broken_row_blocks_the_batch_but_only_names_itself() {
        let form = Form::parse("actie=alles&nummer:een.mp3=drie&album=Nieuw");
        let view = preview(&album_with_two_albums(), &form);

        assert_eq!(view.problems.len(), 1, "{:?}", view.problems);
        assert!(view.problems[0].starts_with("Tracknummer"));

        // De rij zelf valt uit het plan; de andere bestanden staan er nog in.
        assert!(!file_in(&view, "een.mp3").changes_anything());
        assert!(file_in(&view, "twee.mp3").changes_anything());
    }

    #[test]
    fn a_report_counts_what_went_well_and_what_did_not() {
        let report = SaveReport {
            results: vec![
                SaveResult {
                    name: "een.mp3".to_string(),
                    outcome: Outcome::Saved(vec!["Album".to_string()]),
                },
                SaveResult {
                    name: "twee.mp3".to_string(),
                    outcome: Outcome::Unchanged,
                },
                SaveResult {
                    name: "drie.mp3".to_string(),
                    outcome: Outcome::Failed("de map is alleen-lezen".to_string()),
                },
            ],
        };

        assert_eq!(report.saved(), 1);
        assert_eq!(report.failed(), 1);
        assert!(report.has_failures());
        assert_eq!(
            report.summary(),
            "1 bestand bijgewerkt; 1 bestand is niet opgeslagen."
        );

        assert!(report.results[0].describe().contains("Album"));
        assert!(report.results[1].describe().contains("niet aangeraakt"));
        assert!(report.results[2].describe().contains("alleen-lezen"));
        assert!(
            report.results[2].describe().contains("onveranderd"),
            "een mislukking hoort te zeggen dat het bestand heel is gebleven"
        );
    }

    #[test]
    fn the_table_shows_a_dash_where_a_tag_is_missing() {
        let page = album(&album_with_two_albums(), &Form::select_all());
        let row = page
            .rows
            .iter()
            .find(|row| row.name == "drie.mp3")
            .expect("de rij hoort er te zijn");

        // In een invulbare kolom is het streepje de grijze tekst in het veld;
        // het discnummer staat er als gewone cel.
        assert_eq!(row.input(RowField::Album).placeholder, EMPTY);
        assert_eq!(row.disc, EMPTY);
    }

    // ── De tabel per schijf ───────────────────────────────────────────────

    /// De kop die bij dit bestand hoort; paniekt wanneer die er niet is.
    fn heading_of<'a>(page: &'a AlbumPage, name: &str) -> &'a GroupHeading {
        row_of(page, name)
            .group
            .as_ref()
            .unwrap_or_else(|| panic!("boven '{name}' hoort een kop te staan"))
    }

    #[test]
    fn the_table_gets_a_heading_at_the_start_of_every_disc() {
        // AC #1 en #2: een kop per schijf, en de bestanden zonder discnummer
        // als eigen groep achteraan.
        let listing = set_of_two_discs();
        let page = album(&listing, &Form::parse("actie=alles"));

        let first = heading_of(&page, "01 - Eerste.mp3");
        assert_eq!(first.label, "Schijf 1");
        assert_eq!(first.summary, "2 bestanden");

        assert!(
            row_of(&page, "02 - Tweede.mp3").group.is_none(),
            "een kop hoort alleen boven het eerste bestand van een groep"
        );

        assert_eq!(heading_of(&page, "03 - Derde.mp3").label, "Schijf 2");

        let last = heading_of(&page, "04 - Vierde.mp3");
        assert_eq!(last.label, "Zonder discnummer");
        assert_eq!(last.action, "schijf:");
    }

    #[test]
    fn a_folder_without_disc_numbers_has_no_headings() {
        // AC #5: dan ziet de tabel eruit als altijd.
        let listing = listing_of(vec![
            track("een.mp3", Some("Album"), None),
            track("twee.mp3", Some("Album"), None),
        ]);

        let page = album(&listing, &Form::parse("actie=alles"));

        assert!(page.rows.iter().all(|row| row.group.is_none()));
    }

    #[test]
    fn the_button_of_a_group_ticks_it_and_leaves_the_rest_alone() {
        // AC #4: één klik voor een hele schijf, en de rest van de selectie
        // blijft precies zoals hij stond.
        let listing = set_of_two_discs();

        // Alleen het bestand zonder discnummer stond aan.
        let page = album(
            &listing,
            &Form::parse("actie=schijf:1&bestand=04 - Vierde.mp3"),
        );

        assert!(row_of(&page, "01 - Eerste.mp3").selected);
        assert!(row_of(&page, "02 - Tweede.mp3").selected);
        assert!(
            !row_of(&page, "03 - Derde.mp3").selected,
            "de tweede schijf hoort deze knop niets aan te gaan"
        );
        assert!(
            row_of(&page, "04 - Vierde.mp3").selected,
            "wat er buiten de groep aanstond, hoort te blijven staan"
        );
    }

    #[test]
    fn the_button_of_a_selected_group_unticks_it() {
        let listing = set_of_two_discs();
        let page = album(
            &listing,
            &Form::parse(&format!("actie=schijf:1{}", everything_in(&listing))),
        );

        assert!(!row_of(&page, "01 - Eerste.mp3").selected);
        assert!(!row_of(&page, "02 - Tweede.mp3").selected);
        assert!(row_of(&page, "03 - Derde.mp3").selected);
        assert!(row_of(&page, "04 - Vierde.mp3").selected);
    }

    #[test]
    fn the_button_says_what_a_click_would_do() {
        let listing = set_of_two_discs();

        let all = album(&listing, &Form::parse("actie=alles"));
        assert_eq!(
            heading_of(&all, "01 - Eerste.mp3").button,
            "Schijf 1 uitvinken"
        );

        let none = album(&listing, &Form::parse("actie=niets"));
        assert_eq!(
            heading_of(&none, "01 - Eerste.mp3").button,
            "Schijf 1 selecteren"
        );

        // Half aangevinkt is niet aangevinkt: dan vult de knop de groep aan.
        let half = album(&listing, &Form::parse("bestand=01 - Eerste.mp3"));
        assert_eq!(
            heading_of(&half, "01 - Eerste.mp3").button,
            "Schijf 1 selecteren"
        );
    }

    #[test]
    fn the_group_without_a_disc_number_has_a_button_of_its_own() {
        let listing = set_of_two_discs();
        let page = album(&listing, &Form::parse("actie=schijf:"));

        assert!(row_of(&page, "04 - Vierde.mp3").selected);
        assert!(!row_of(&page, "01 - Eerste.mp3").selected);
        assert_eq!(page.selected, 1);
    }

    #[test]
    fn a_group_button_fills_in_nothing() {
        // De knop zet de selectie en verder niets: geen invoerveld, geen
        // melding van een hulpactie.
        let listing = set_of_two_discs();
        let page = album(
            &listing,
            &Form::parse("actie=schijf:2&nummer:01 - Eerste.mp3=7"),
        );

        assert!(page.helper_notice.is_none());
        assert_eq!(
            row_of(&page, "01 - Eerste.mp3")
                .input(RowField::Track)
                .value,
            "7"
        );
        assert!(page.fields.iter().all(|field| field.value.is_empty()));
    }

    #[test]
    fn an_unreadable_group_changes_nothing() {
        // Een waarde die niet uit een knop van deze pagina komt, hoort de
        // selectie met rust te laten.
        let listing = set_of_two_discs();
        let page = album(
            &listing,
            &Form::parse("actie=schijf:tweede&bestand=01 - Eerste.mp3"),
        );

        assert_eq!(page.selected, 1);
        assert!(row_of(&page, "01 - Eerste.mp3").selected);
    }

    #[test]
    fn a_heading_counts_what_needs_attention_in_its_group() {
        // AC #3: het oordeel komt uit de signalering die al in de lijst zit.
        let mut tracks = vec![
            track_with(
                "een.mp3",
                Tags {
                    disc: Some(1),
                    track: Some(1),
                    ..Tags::default()
                },
            ),
            track_with(
                "twee.mp3",
                Tags {
                    disc: Some(1),
                    track: Some(2),
                    ..Tags::default()
                },
            ),
        ];
        tracks[0].issues = vec![crate::checks::TrackIssue::MissingTitle];

        let listing = listing_of(tracks);
        let page = album(&listing, &Form::parse("actie=alles"));

        assert_eq!(
            heading_of(&page, "een.mp3").summary,
            "2 bestanden, 1 vraagt aandacht"
        );
    }
}
