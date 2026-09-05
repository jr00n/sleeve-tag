//! Het bewerkformulier van één bestand (PRD FR-5 en FR-6).
//!
//! Deze module vertaalt tussen het genormaliseerde tagmodel en de tekst die in
//! een HTML-formulier staat. Dat is niet hetzelfde: een invoerveld kent geen
//! `None`, alleen lege tekst, en een tracknummer is daar tekst en geen getal.
//! Precies die twee verschillen zitten hier, zodat de handler ze niet hoeft te
//! kennen.
//!
//! Bestanden worden hier niet geopend: in en uit gaan een [`Tags`] en een
//! [`Form`].

use crate::browse::Crumb;
use crate::tags::Tags;

/// De twaalf kernvelden, zoals ze in het formulier staan.
///
/// Alles is tekst. Een leeg veld betekent "deze tag hoort niet in het bestand"
/// — dat is de regel uit PRD §7, en [`Form::to_tags`] maakt er `None` van.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Form {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub album_artist: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub track: String,
    #[serde(default)]
    pub track_total: String,
    #[serde(default)]
    pub disc: String,
    #[serde(default)]
    pub disc_total: String,
    #[serde(default)]
    pub year: String,
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub composer: String,
    #[serde(default)]
    pub comment: String,
}

impl Form {
    /// Vult het formulier met wat er in het bestand staat.
    pub fn from_tags(tags: &Tags) -> Self {
        Self {
            title: text(&tags.title),
            artist: text(&tags.artist),
            album_artist: text(&tags.album_artist),
            album: text(&tags.album),
            track: number(tags.track),
            track_total: number(tags.track_total),
            disc: number(tags.disc),
            disc_total: number(tags.disc_total),
            year: text(&tags.year),
            genre: text(&tags.genre),
            composer: text(&tags.composer),
            comment: text(&tags.comment),
        }
    }

    /// Maakt er een tagmodel van, of levert leesbare meldingen op.
    ///
    /// De numerieke velden worden hier gecontroleerd, dus vóórdat er ook maar
    /// iets naar het bestand gaat. Een typefout in een tracknummer hoort geen
    /// schrijfactie te starten die halverwege afketst.
    ///
    /// Het trimmen en het leeg-is-verwijderen laat deze functie aan
    /// [`Tags::normalized`] over: die regel hoort op één plek te staan.
    pub fn to_tags(&self) -> Result<Tags, Vec<String>> {
        let mut problems = Vec::new();

        let tags = Tags {
            title: value(&self.title),
            artist: value(&self.artist),
            album_artist: value(&self.album_artist),
            album: value(&self.album),
            track: parse(&self.track, "Tracknummer", &mut problems),
            track_total: parse(&self.track_total, "Aantal tracks", &mut problems),
            disc: parse(&self.disc, "Discnummer", &mut problems),
            disc_total: parse(&self.disc_total, "Aantal discs", &mut problems),
            year: value(&self.year),
            genre: value(&self.genre),
            composer: value(&self.composer),
            comment: value(&self.comment),
        };

        if problems.is_empty() {
            Ok(tags.normalized())
        } else {
            Err(problems)
        }
    }
}

/// Wat er boven het formulier staat na een poging tot opslaan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// Het opslaan is gelukt; de getoonde waarden komen uit het bestand.
    ///
    /// Meer dan één regel, omdat er soms iets bij te melden valt: een tagblok
    /// dat er niet in hoorde en bij deze schrijfactie is verdwenen, hoort de
    /// gebruiker te zien en niet alleen in het logboek te staan.
    Saved(Vec<String>),

    /// Er is niets geschreven, en het bestand is ongemoeid gebleven.
    Failed(Vec<String>),
}

impl Notice {
    /// Of dit een bevestiging is; de opmaak hangt ervan af.
    pub fn is_saved(&self) -> bool {
        matches!(self, Notice::Saved(_))
    }

    /// De regels die getoond worden.
    pub fn lines(&self) -> Vec<String> {
        match self {
            Notice::Saved(lines) => lines.clone(),
            Notice::Failed(lines) => lines.clone(),
        }
    }
}

/// Alles wat de bewerkpagina van één bestand nodig heeft.
#[derive(Debug, Clone)]
pub struct EditPage {
    /// Bestandsnaam, als kop van de pagina.
    pub name: String,

    /// Tot en met de map waarin het bestand staat.
    pub crumbs: Vec<Crumb>,

    /// Waar het formulier naartoe post; ook de URL van deze pagina.
    pub url: String,

    /// Naar de geavanceerde weergave met alle ruwe tags.
    pub raw_url: String,

    /// De volledige hoes, wanneer het bestand er een heeft.
    pub art_url: String,
    pub has_art: bool,

    /// Naar de hoesweergave met formaat, afmetingen en grootte (FR-12).
    ///
    /// Ook zonder hoes: daar staat dan dat er geen is, en straks de manier om
    /// er een toe te voegen.
    pub cover_url: String,

    pub format: String,
    pub duration: String,

    /// Tagblokken die niet bij dit bestandsformaat horen, bij naam.
    pub foreign_tags: Vec<String>,

    /// Aparte POST-actie die alleen de ongewenste tagblokken opruimt.
    pub cleanup_url: String,

    /// Wat er in de invoervelden staat.
    pub fields: Form,

    /// Bevestiging of foutmelding; leeg bij het openen van de pagina.
    pub notice: Option<Notice>,

    /// Waar de knop "terug" heen leidt, en hoe hij heet.
    ///
    /// Wie uit de albumweergave komt, heeft daar net een selectie gemaakt en
    /// wil daarheen terug; wie uit de maplijst komt, naar de map. Welke van de
    /// twee het is, staat in de URL waarmee deze pagina is geopend.
    pub back_url: String,
    pub back_label: String,

    /// De bovengrens aan een upload in megabytes, uit `MAX_UPLOAD_MB`.
    ///
    /// Nodig omdat er ook op deze pagina een hoes neergezet kan worden. De
    /// controle op omvang gebeurt in de browser: een upload boven de grens
    /// wordt door de server afgekapt terwijl de browser nog verstuurt, en dan
    /// komt de uitleg die hij meestuurt nooit aan.
    pub max_upload_mb: u32,
}

impl EditPage {
    pub fn foreign_tags_label(&self) -> String {
        self.foreign_tags.join(", ")
    }
}

/// Een tekstveld uit het model, of lege tekst.
fn text(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

/// Een getal uit het model, of lege tekst.
fn number(value: Option<u32>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

/// Een ingevuld tekstveld, of `None` wanneer er niets staat.
fn value(raw: &str) -> Option<String> {
    Some(raw.to_string()).filter(|value| !value.trim().is_empty())
}

/// Leest een getal uit een invoerveld en meldt het wanneer dat niet lukt.
fn parse(raw: &str, label: &str, problems: &mut Vec<String>) -> Option<u32> {
    match parse_number(raw, label) {
        Ok(number) => number,
        Err(problem) => {
            problems.push(problem);
            None
        }
    }
}

/// Leest een getal uit een invoerveld, of levert de melding erover.
///
/// Een leeg veld is geen fout maar een ontbrekende waarde. De melding staat
/// hier en niet bij de aanroeper, zodat de albumweergave een verkeerd
/// discnummer op precies dezelfde manier afkeurt als dit formulier.
pub fn parse_number(raw: &str, label: &str) -> Result<Option<u32>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }

    raw.parse::<u32>()
        .map(Some)
        .map_err(|_| format!("{label} moet een getal van 0 of hoger zijn; “{raw}” is dat niet."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled_tags() -> Tags {
        Tags {
            title: Some("Stilte in D".to_string()),
            artist: Some("De Testartiest".to_string()),
            album_artist: Some("De Albumartiest".to_string()),
            album: Some("Fixtures voor Sleeve".to_string()),
            track: Some(3),
            track_total: Some(12),
            disc: Some(1),
            disc_total: Some(2),
            year: Some("2024".to_string()),
            genre: Some("Ambient".to_string()),
            composer: Some("De Componist".to_string()),
            comment: Some("Een commentaar".to_string()),
        }
    }

    #[test]
    fn a_filled_model_survives_the_trip_to_the_form_and_back() {
        let form = Form::from_tags(&filled_tags());

        assert_eq!(form.title, "Stilte in D");
        assert_eq!(form.track, "3");
        assert_eq!(form.track_total, "12");

        assert_eq!(form.to_tags().expect("moet geldig zijn"), filled_tags());
    }

    #[test]
    fn an_empty_model_gives_empty_fields() {
        let form = Form::from_tags(&Tags::default());

        assert_eq!(form, Form::default());
        assert_eq!(form.to_tags().expect("moet geldig zijn"), Tags::default());
    }

    #[test]
    fn an_emptied_field_becomes_a_removal() {
        // De regel uit PRD §7: leeg betekent "deze tag hoort er niet te staan".
        let mut form = Form::from_tags(&filled_tags());
        form.composer = String::new();
        // Alleen spaties telt ook als leeg.
        form.genre = "   ".to_string();

        let tags = form.to_tags().expect("moet geldig zijn");

        assert_eq!(tags.composer, None);
        assert_eq!(tags.genre, None);
        assert_eq!(tags.title, Some("Stilte in D".to_string()));
    }

    #[test]
    fn surrounding_spaces_are_trimmed() {
        let form = Form {
            title: "  Met spaties  ".to_string(),
            ..Form::default()
        };

        assert_eq!(
            form.to_tags().expect("moet geldig zijn").title,
            Some("Met spaties".to_string())
        );
    }

    #[test]
    fn every_numeric_field_reports_its_own_name() {
        for (field, label) in [
            ("track", "Tracknummer"),
            ("track_total", "Aantal tracks"),
            ("disc", "Discnummer"),
            ("disc_total", "Aantal discs"),
        ] {
            let mut form = Form::default();
            match field {
                "track" => form.track = "drie".to_string(),
                "track_total" => form.track_total = "drie".to_string(),
                "disc" => form.disc = "drie".to_string(),
                _ => form.disc_total = "drie".to_string(),
            }

            let problems = form.to_tags().expect_err("dit hoort een fout te zijn");

            assert_eq!(problems.len(), 1, "{field}: {problems:?}");
            assert!(
                problems[0].starts_with(label),
                "{field}: de melding noemt het veld niet: {}",
                problems[0]
            );
            assert!(
                problems[0].contains("drie"),
                "{field}: de melding herhaalt de invoer niet: {}",
                problems[0]
            );
        }
    }

    #[test]
    fn a_negative_number_is_refused() {
        // Een tracknummer van -1 bestaat niet; dat hoort te stranden vóór er
        // iets naar het bestand gaat.
        let form = Form {
            track: "-1".to_string(),
            ..Form::default()
        };

        assert!(form.to_tags().is_err());
    }

    #[test]
    fn several_mistakes_are_reported_together() {
        // Eén veld per keer corrigeren omdat de app maar één fout tegelijk
        // meldt, is onnodig vervelend.
        let form = Form {
            track: "een".to_string(),
            disc: "twee".to_string(),
            ..Form::default()
        };

        let problems = form.to_tags().expect_err("dit hoort een fout te zijn");
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn spaces_around_a_number_are_allowed() {
        let form = Form {
            track: " 7 ".to_string(),
            ..Form::default()
        };

        assert_eq!(form.to_tags().expect("moet geldig zijn").track, Some(7));
    }

    #[test]
    fn a_notice_knows_what_it_is() {
        let saved = Notice::Saved(vec!["Opgeslagen.".to_string()]);
        assert!(saved.is_saved());
        assert_eq!(saved.lines(), vec!["Opgeslagen.".to_string()]);

        let failed = Notice::Failed(vec!["Er ging iets mis.".to_string()]);
        assert!(!failed.is_saved());
        assert_eq!(failed.lines().len(), 1);
    }
}
