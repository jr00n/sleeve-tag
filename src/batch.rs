//! De albumweergave: een selectie bestanden en de velden die ze delen (FR-8).
//!
//! Waar [`crate::edit`] één bestand bedient, gaat het hier om een map vol
//! bestanden tegelijk. Deze module vertaalt tussen het verstuurde formulier en
//! een weergavemodel dat de templates rechtstreeks kunnen renderen, en bepaalt
//! per gedeeld veld wat er met de selectie zou gebeuren.
//!
//! Er wordt hier niets geschreven en er gaat geen bestand open: in en uit gaan
//! een [`Listing`] en een [`Form`]. Het daadwerkelijk wegschrijven hoort bij de
//! diff-preview, zodat een batch-wijziging nooit zonder voorbeeld plaatsvindt.

use std::collections::BTreeSet;

use percent_encoding::percent_decode_str;

use crate::browse::{self, Crumb, Listing, TrackSummary};
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
}

impl Action {
    fn parse(raw: &str) -> Action {
        match raw {
            "alles" => Action::All,
            "niets" => Action::None,
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

    /// Wat er aan de invoer mankeert; leeg wanneer alles klopt.
    pub problems: Vec<String>,
}

impl AlbumPage {
    /// Of er iets geselecteerd is.
    pub fn has_selection(&self) -> bool {
        self.selected > 0
    }

    /// Of er ook maar één gedeeld veld iets zou doen.
    pub fn changes_anything(&self) -> bool {
        self.fields
            .iter()
            .any(|field| field.cleared || !field.value.trim().is_empty())
    }
}

/// Bouwt het weergavemodel van de albumpagina.
///
/// `listing` levert de bestanden en hun tags; `form` bepaalt wat er
/// geselecteerd en ingevuld is. Er wordt hier geen bestand geopend: de tags
/// zitten al in de listing.
pub fn album(listing: &Listing, form: &Form) -> AlbumPage {
    let selected = resolve_selection(listing, form);

    let rows: Vec<Row> = listing
        .tracks
        .iter()
        .map(|track| Row {
            selected: selected.contains(&track.name),
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

    // De signalering, de sortering en de tags komen allemaal uit de listing;
    // hier wordt alleen nog gekozen waar de gedeelde velden naar kijken.
    let chosen: Vec<&TrackSummary> = listing
        .tracks
        .iter()
        .filter(|track| selected.contains(&track.name))
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
        Action::Keep => form.selected.clone(),
    }
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
        TrackSummary {
            name: name.to_string(),
            path: format!("Album/{name}"),
            tags: Tags {
                album: album.map(str::to_string),
                disc,
                ..Tags::default()
            },
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
