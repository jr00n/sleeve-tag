//! De hoesweergave van één bestand (PRD FR-12).
//!
//! Een thumbnail van veertig pixels verraadt niet of de hoes eronder 300×300 of
//! 3000×3000 is, en of daar een halve megabyte in gaat zitten. Wie een hoes
//! gaat vervangen wil dat eerst weten, dus toont deze pagina de afbeelding
//! groot met de feiten erbij.
//!
//! Hier worden geen bestanden geopend en geen pixels aangeraakt: in gaat de
//! [`ArtInfo`] die [`crate::tags`] uit het bestand las, uit komt tekst die een
//! template rechtstreeks kan tonen.
//!
//! Het rapport per bestand komt uit [`crate::batch`]: dat model is daar
//! ontstaan voor de batch-tagbewerking, en een hoes in twaalf tracks zetten
//! stelt precies dezelfde vraag — wat is er per bestand gebeurd.

use crate::batch::SaveReport;
use crate::browse::Crumb;
use crate::tags::ArtInfo;

/// Alles wat de hoespagina van één bestand nodig heeft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverPage {
    /// Bestandsnaam, als kop van de pagina.
    pub name: String,

    /// Tot en met de map waarin het bestand staat.
    pub crumbs: Vec<Crumb>,

    /// De hoes op ware grootte.
    pub art_url: String,

    /// Terug naar het bewerkformulier van dit bestand.
    pub edit_url: String,

    /// Waar het formulier naartoe post; ook de URL van deze pagina.
    pub url: String,

    /// Terug naar de map waarin het bestand staat.
    pub back_url: String,

    /// Hoeveel bewerkbare bestanden er in deze map staan, dit bestand
    /// meegerekend.
    ///
    /// Bepaalt of het zin heeft om "in alle tracks" aan te bieden: in een map
    /// met één bestand is dat dezelfde knop twee keer.
    pub tracks_in_folder: usize,

    /// Wat er over de hoes bekend is; `None` wanneer het bestand er geen heeft.
    pub details: Option<CoverDetails>,

    /// Wat er met een zojuist aangeleverde afbeelding gebeurd is; leeg zolang
    /// er niets is geüpload.
    pub notice: Option<Notice>,

    /// Hoe het schrijven per bestand is afgelopen (FR-13, FR-16).
    pub report: Option<SaveReport>,
}

impl CoverPage {
    /// Of er een hoes te tonen is.
    pub fn has_art(&self) -> bool {
        self.details.is_some()
    }

    /// Of het zin heeft om de hele map als doel aan te bieden.
    pub fn has_siblings(&self) -> bool {
        self.tracks_in_folder > 1
    }

    /// Het opschrift van de knop die de hele map bedient.
    pub fn all_tracks_label(&self) -> String {
        format!("Alle {} tracks in deze map", self.tracks_in_folder)
    }
}

/// Wat er boven de hoespagina staat na een upload of een verwijdering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// De afbeelding is aangenomen; dit is wat ermee gebeurd is.
    Accepted(String),

    /// Er is niets geschreven, en de bestanden zijn ongemoeid gebleven.
    Refused(String),
}

impl Notice {
    /// De melding dat een afbeelding is aangenomen, met wat ermee gebeurd is.
    ///
    /// Verkleinen is een wijziging die de gebruiker niet zelf vroeg; die hoort
    /// er dus bij te staan, met de afmetingen ervoor en erna.
    pub fn accepted(prepared: &crate::art::Prepared) -> Notice {
        let (from_width, from_height) = prepared.original;

        if prepared.is_resized() {
            Notice::Accepted(format!(
                "De afbeelding is verkleind van {from_width} × {from_height} naar {} × {} pixels ({}).",
                prepared.width,
                prepared.height,
                format_bytes(prepared.data.len())
            ))
        } else {
            Notice::Accepted(format!(
                "De afbeelding is overgenomen zoals hij is: {} × {} pixels ({}).",
                prepared.width,
                prepared.height,
                format_bytes(prepared.data.len())
            ))
        }
    }

    /// Of dit een bevestiging is; de opmaak hangt ervan af.
    pub fn is_accepted(&self) -> bool {
        matches!(self, Notice::Accepted(_))
    }

    pub fn line(&self) -> &str {
        match self {
            Notice::Accepted(line) | Notice::Refused(line) => line,
        }
    }
}

/// De feiten over één hoes, klaar om te tonen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverDetails {
    /// Het formaat in gewone taal: `JPEG`, `PNG`.
    pub format: String,

    /// Het MIME-type zoals het in het bestand staat.
    pub mime: String,

    pub width: u32,
    pub height: u32,

    /// De omvang in bytes, leesbaar opgemaakt.
    pub size: String,
}

impl CoverDetails {
    pub fn of(art: &ArtInfo) -> CoverDetails {
        CoverDetails {
            format: format_of(&art.mime),
            mime: art.mime.clone(),
            width: art.width,
            height: art.height,
            size: format_bytes(art.bytes),
        }
    }

    /// De afmetingen als één regel.
    pub fn dimensions(&self) -> String {
        format!("{} × {} pixels", self.width, self.height)
    }

    /// Of deze hoes vierkant is.
    ///
    /// Vrijwel elke speler toont een hoes in een vierkant vak; een afbeelding
    /// die dat niet is, wordt daar bijgesneden of uitgerekt. Dat is geen fout,
    /// maar wel iets om te weten vóór je hem laat staan.
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }
}

/// Het formaat in gewone taal, afgeleid uit het MIME-type.
///
/// Valt het type niet te herkennen, dan komt het ruwe type terug: liever iets
/// raadselachtigs dan een onwaarheid, want dit is een pagina om op af te gaan.
fn format_of(mime: &str) -> String {
    match mime.to_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "JPEG".to_string(),
        "image/png" => "PNG".to_string(),
        "image/gif" => "GIF".to_string(),
        "image/webp" => "WebP".to_string(),
        "image/bmp" => "BMP".to_string(),
        other => other.to_string(),
    }
}

/// Een omvang in bytes als leesbare tekst.
///
/// Duizendtallen, net als de Finder en vrijwel elk ander programma waar de
/// gebruiker zijn muziek in ziet staan; een decimaal met een komma, want de UI
/// is Nederlands.
pub fn format_bytes(bytes: usize) -> String {
    const UNIT: f64 = 1000.0;

    let bytes = bytes as f64;

    if bytes < UNIT {
        return format!("{bytes:.0} bytes");
    }

    let (value, unit) = if bytes < UNIT * UNIT {
        (bytes / UNIT, "kB")
    } else {
        (bytes / (UNIT * UNIT), "MB")
    };

    // Eén cijfer achter de komma is genoeg om "412,3 kB" van "1,2 MB" te
    // onderscheiden; meer suggereert een precisie die niemand nodig heeft.
    format!("{value:.1} {unit}").replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn art(mime: &str, width: u32, height: u32, bytes: usize) -> ArtInfo {
        ArtInfo {
            mime: mime.to_string(),
            width,
            height,
            bytes,
        }
    }

    #[test]
    fn the_facts_of_a_cover_are_ready_to_show() {
        let details = CoverDetails::of(&art("image/jpeg", 1000, 1000, 412_345));

        assert_eq!(details.format, "JPEG");
        assert_eq!(details.mime, "image/jpeg");
        assert_eq!(details.dimensions(), "1000 × 1000 pixels");
        assert_eq!(details.size, "412,3 kB");
        assert!(details.is_square());
    }

    #[test]
    fn a_cover_that_is_not_square_says_so() {
        let details = CoverDetails::of(&art("image/png", 1400, 1000, 900));

        assert_eq!(details.format, "PNG");
        assert!(!details.is_square());
    }

    #[test]
    fn an_unknown_type_comes_back_as_it_is() {
        // Liever raadselachtig dan onwaar: dit is een pagina om op af te gaan.
        let details = CoverDetails::of(&art("application/octet-stream", 10, 10, 10));

        assert_eq!(details.format, "application/octet-stream");
    }

    #[test]
    fn sizes_are_readable() {
        assert_eq!(format_bytes(0), "0 bytes");
        assert_eq!(format_bytes(999), "999 bytes");
        assert_eq!(format_bytes(1_000), "1,0 kB");
        assert_eq!(format_bytes(12_345), "12,3 kB");
        assert_eq!(format_bytes(1_500_000), "1,5 MB");
        assert_eq!(format_bytes(25_000_000), "25,0 MB");
    }
}
