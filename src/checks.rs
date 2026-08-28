//! Signalering van ontbrekende en onderling afwijkende tags (PRD FR-4).
//!
//! De maplijst hoort zelf aan te wijzen waar iets mis is, zodat de gebruiker
//! niet elk bestand hoeft te openen om dat te ontdekken. Wat hier gevonden
//! wordt is **puur informatief**: deze module leest geen bestanden, schrijft
//! niets en stelt niets voor. Ze rekent alleen op het genormaliseerde tagmodel.
//!
//! Twee niveaus:
//! - per bestand wat er aan dat bestand zelf mankeert;
//! - per map wat er tussen de bestanden onderling niet klopt.

use std::collections::HashMap;
use std::fmt;

use crate::tags::{ArtInfo, Tags};

/// Hoeveel afwijkende waarden er hoogstens bij naam genoemd worden.
///
/// Een map met dertig verschillende albumtitels levert anders één onleesbare
/// regel op; het aantal zegt dan meer dan de opsomming.
const MAX_NAMED_VALUES: usize = 3;

/// De feiten over één bestand die de signalering nodig heeft.
#[derive(Debug, Clone, Copy)]
pub struct Entry<'a> {
    pub tags: &'a Tags,

    /// Wat er over de embedded hoes bekend is; `None` wanneer het bestand er
    /// geen heeft.
    pub art: Option<&'a ArtInfo>,

    /// Tagblokken die niet bij het formaat van dit bestand horen, bij naam.
    ///
    /// Komt zo uit [`crate::tags`]; deze module leest zelf geen bestanden.
    pub foreign_tags: &'a [String],
}

/// Wat er aan één bestand mankeert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrackIssue {
    MissingTitle,
    MissingArtist,
    MissingAlbum,
    MissingArt,
    MissingTrackNumber,

    /// Het tracknummer van dit bestand komt in deze map vaker voor.
    DuplicateTrackNumber,

    /// Er zit een tagblok in dat niet bij dit bestandsformaat hoort.
    ///
    /// In de praktijk een ID3-blok vóór een FLAC. Zo'n blok wordt niet gelezen
    /// en niet bijgewerkt, en zegt na een wijziging dus iets anders dan de tag
    /// die er wél toe doet. Welke van de twee een speler kiest, is niet te
    /// voorspellen — vandaar dat het opvalt vóórdat er iets bewerkt wordt.
    ForeignTagBlock,
}

impl TrackIssue {
    /// Korte tekst voor het label in de lijst.
    pub fn label(&self) -> &'static str {
        match self {
            TrackIssue::MissingTitle => "geen titel",
            TrackIssue::MissingArtist => "geen artiest",
            TrackIssue::MissingAlbum => "geen album",
            TrackIssue::MissingArt => "geen hoes",
            TrackIssue::MissingTrackNumber => "geen tracknummer",
            TrackIssue::DuplicateTrackNumber => "dubbel tracknummer",
            TrackIssue::ForeignTagBlock => "tagblok dat er niet hoort",
        }
    }
}

impl fmt::Display for TrackIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Een veld dat binnen één map voor alle bestanden gelijk hoort te zijn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedField {
    Album,
    AlbumArtist,
    Year,
}

impl SharedField {
    /// Meervoud, zoals het in de melding wordt gebruikt.
    fn plural(&self) -> &'static str {
        match self {
            SharedField::Album => "albumtitels",
            SharedField::AlbumArtist => "albumartiesten",
            SharedField::Year => "jaartallen",
        }
    }
}

/// Wat er tussen de bestanden van één map niet klopt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderIssue {
    /// Een gedeeld veld heeft meer dan één ingevulde waarde.
    DifferentValues {
        field: SharedField,
        values: Vec<String>,
    },

    /// Zoveel bestanden hebben helemaal geen tracknummer.
    MissingTrackNumbers(usize),

    /// Deze tracknummers komen meer dan eens voor.
    DuplicateTrackNumbers(Vec<u32>),

    /// De bestanden met een hoes hebben er niet allemaal dezelfde (FR-12).
    ///
    /// Zoveel verschillende hoezen zijn er geteld. Bestanden zonder hoes tellen
    /// niet mee: die hebben hun eigen melding.
    DifferentArt(usize),
}

impl FolderIssue {
    /// Een hele zin die zegt wat er aan de hand is.
    pub fn describe(&self) -> String {
        match self {
            FolderIssue::DifferentValues { field, values } => {
                format!(
                    "{} verschillende {} in deze map: {}",
                    values.len(),
                    field.plural(),
                    enumerate(values)
                )
            }

            FolderIssue::MissingTrackNumbers(1) => "1 bestand heeft geen tracknummer".to_string(),
            FolderIssue::MissingTrackNumbers(count) => {
                format!("{count} bestanden hebben geen tracknummer")
            }

            FolderIssue::DifferentArt(count) => {
                format!("{count} verschillende hoezen in deze map")
            }

            FolderIssue::DuplicateTrackNumbers(numbers) => {
                let numbers: Vec<String> = numbers.iter().map(u32::to_string).collect();
                if numbers.len() == 1 {
                    format!("tracknummer {} komt meer dan eens voor", numbers[0])
                } else {
                    format!(
                        "deze tracknummers komen meer dan eens voor: {}",
                        enumerate(&numbers)
                    )
                }
            }
        }
    }
}

impl fmt::Display for FolderIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

/// Het oordeel over één map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Review {
    /// Per bestand, in dezelfde volgorde als de invoer.
    pub tracks: Vec<Vec<TrackIssue>>,

    /// Wat er tussen de bestanden onderling niet klopt.
    pub folder: Vec<FolderIssue>,
}

/// Beoordeelt alle bestanden van één map.
///
/// De invoer is de volledige inhoud van de map, ook wanneer de gebruiker een
/// filter heeft staan: "twee verschillende albumtitels" hoort niet te
/// verdwijnen zodra er gezocht wordt, want aan de map is dan niets veranderd.
pub fn review(entries: &[Entry<'_>]) -> Review {
    let duplicates = duplicate_track_numbers(entries);

    let tracks = entries
        .iter()
        .map(|entry| track_issues(entry, &duplicates))
        .collect();

    let mut folder = Vec::new();

    for field in [
        SharedField::Album,
        SharedField::AlbumArtist,
        SharedField::Year,
    ] {
        let values = distinct_values(entries, field);
        if values.len() > 1 {
            folder.push(FolderIssue::DifferentValues { field, values });
        }
    }

    let missing = entries
        .iter()
        .filter(|entry| entry.tags.track.is_none())
        .count();
    if missing > 0 {
        folder.push(FolderIssue::MissingTrackNumbers(missing));
    }

    if !duplicates.is_empty() {
        folder.push(FolderIssue::DuplicateTrackNumbers(duplicates));
    }

    let covers = distinct_covers(entries);
    if covers > 1 {
        folder.push(FolderIssue::DifferentArt(covers));
    }

    Review { tracks, folder }
}

/// Hoeveel verschillende hoezen er in deze map zitten.
///
/// Vergeleken wordt op type, afmetingen en omvang, en niet op de pixels zelf:
/// twee hoezen die daarin gelijk zijn, zijn in de praktijk dezelfde
/// afbeelding, en de bytes uitpakken om dat zeker te weten is voor een
/// constatering te veel werk. Bestanden zonder hoes tellen niet mee; die
/// hebben hun eigen melding.
fn distinct_covers(entries: &[Entry<'_>]) -> usize {
    let mut seen: Vec<(&str, u32, u32, usize)> = Vec::new();

    for art in entries.iter().filter_map(|entry| entry.art) {
        let fingerprint = (art.mime.as_str(), art.width, art.height, art.bytes);
        if !seen.contains(&fingerprint) {
            seen.push(fingerprint);
        }
    }

    seen.len()
}

/// Wat er aan één bestand mankeert.
fn track_issues(entry: &Entry<'_>, duplicates: &[u32]) -> Vec<TrackIssue> {
    let mut issues = Vec::new();
    let tags = entry.tags;

    if tags.title.is_none() {
        issues.push(TrackIssue::MissingTitle);
    }
    if tags.artist.is_none() {
        issues.push(TrackIssue::MissingArtist);
    }
    if tags.album.is_none() {
        issues.push(TrackIssue::MissingAlbum);
    }
    if entry.art.is_none() {
        issues.push(TrackIssue::MissingArt);
    }

    match tags.track {
        None => issues.push(TrackIssue::MissingTrackNumber),
        Some(number) if duplicates.contains(&number) => {
            issues.push(TrackIssue::DuplicateTrackNumber);
        }
        Some(_) => {}
    }

    // Dit gaat niet over wat er in de tag staat maar over hoeveel tags er zijn:
    // een blok dat niet bij het formaat hoort, loopt na de eerste bewerking uit
    // de pas met de tag die wél gelezen wordt.
    if !entry.foreign_tags.is_empty() {
        issues.push(TrackIssue::ForeignTagBlock);
    }

    issues
}

/// De ingevulde waarden van een gedeeld veld, ontdubbeld en gesorteerd.
///
/// Een ontbrekende waarde telt niet mee: dat is een ontbrekend veld op dat ene
/// bestand, en niet een tegenstrijdigheid tussen bestanden. Zonder dat
/// onderscheid zou elke map met één ongetagd bestand als inconsistent gelden.
fn distinct_values(entries: &[Entry<'_>], field: SharedField) -> Vec<String> {
    let mut values: Vec<String> = entries
        .iter()
        .filter_map(|entry| match field {
            SharedField::Album => entry.tags.album.clone(),
            SharedField::AlbumArtist => entry.tags.album_artist.clone(),
            SharedField::Year => entry.tags.year.clone(),
        })
        .collect();

    values.sort();
    values.dedup();
    values
}

/// De tracknummers die in deze map vaker dan één keer voorkomen, oplopend.
fn duplicate_track_numbers(entries: &[Entry<'_>]) -> Vec<u32> {
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for entry in entries {
        if let Some(number) = entry.tags.track {
            *counts.entry(number).or_default() += 1;
        }
    }

    let mut duplicates: Vec<u32> = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(number, _)| number)
        .collect();

    duplicates.sort_unstable();
    duplicates
}

/// Somt waarden op, met een grens aan hoeveel er bij naam genoemd worden.
fn enumerate(values: &[String]) -> String {
    let named: Vec<String> = values
        .iter()
        .take(MAX_NAMED_VALUES)
        .map(|value| format!("“{value}”"))
        .collect();

    let rest = values.len().saturating_sub(named.len());
    match rest {
        0 => named.join(", "),
        1 => format!("{} en nog 1", named.join(", ")),
        _ => format!("{} en nog {rest}", named.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bouwt tags met alleen de velden die de signalering bekijkt.
    fn tags(album: &str, album_artist: &str, year: &str, track: Option<u32>) -> Tags {
        Tags {
            title: Some("Een titel".to_string()),
            artist: Some("Een artiest".to_string()),
            album: Some(album.to_string()),
            album_artist: Some(album_artist.to_string()),
            year: Some(year.to_string()),
            track,
            ..Tags::default()
        }
    }

    /// Een compleet getagd bestand met hoes en het opgegeven tracknummer.
    fn complete(track: u32) -> Tags {
        tags("Het Album", "De Albumartiest", "1999", Some(track))
    }

    /// Een hoes zoals de fixtures die hebben; alle tracks dezelfde.
    fn cover() -> ArtInfo {
        ArtInfo {
            mime: "image/jpeg".to_string(),
            width: 300,
            height: 300,
            bytes: 12_345,
        }
    }

    fn entries<'a>(tags: &'a [Tags], art: Option<&'a ArtInfo>) -> Vec<Entry<'a>> {
        tags.iter()
            .map(|tags| Entry {
                tags,
                art,
                foreign_tags: &[],
            })
            .collect()
    }

    #[test]
    fn a_foreign_tag_block_is_reported_before_anything_is_edited() {
        // Een ID3-blok in een FLAC wordt niet gelezen en niet bijgewerkt: na de
        // eerste wijziging zegt het iets anders dan de tag die er wél toe doet.
        // Dat hoort op te vallen vóórdat iemand gaat bewerken, en niet pas als
        // een speler de verkeerde titel toont.
        let files = [complete(1)];
        let id3 = ["ID3v2".to_string()];

        let with_id3 = review(&[Entry {
            tags: &files[0],
            art: Some(&cover()),
            foreign_tags: &id3,
        }]);

        assert_eq!(with_id3.tracks[0], vec![TrackIssue::ForeignTagBlock]);

        // En zonder zo'n blok blijft het stil: dit is geen melding die elk
        // bestand krijgt.
        let clean = review(&entries(&files, Some(&cover())));
        assert!(
            clean.tracks[0].is_empty(),
            "gevonden: {:?}",
            clean.tracks[0]
        );
    }

    #[test]
    fn a_consistent_folder_has_nothing_to_report() {
        let files = [complete(1), complete(2), complete(3)];

        let review = review(&entries(&files, Some(&cover())));

        assert!(review.folder.is_empty(), "gevonden: {:?}", review.folder);
        assert!(
            review.tracks.iter().all(|issues| issues.is_empty()),
            "gevonden: {:?}",
            review.tracks
        );
    }

    #[test]
    fn an_untagged_folder_reports_every_missing_field() {
        let files = [Tags::default(), Tags::default()];

        let review = review(&entries(&files, None));

        for issues in &review.tracks {
            assert_eq!(
                issues,
                &vec![
                    TrackIssue::MissingTitle,
                    TrackIssue::MissingArtist,
                    TrackIssue::MissingAlbum,
                    TrackIssue::MissingArt,
                    TrackIssue::MissingTrackNumber,
                ]
            );
        }

        // Zonder ingevulde waarden valt er niets te vergelijken; alleen de
        // ontbrekende tracknummers zijn een mapbrede constatering.
        assert_eq!(
            review.folder,
            vec![FolderIssue::MissingTrackNumbers(2)],
            "een lege map is niet inconsistent, alleen leeg"
        );
    }

    #[test]
    fn one_cover_for_the_whole_folder_is_nothing_to_report() {
        let files = [complete(1), complete(2), complete(3)];

        assert!(review(&entries(&files, Some(&cover()))).folder.is_empty());
    }

    #[test]
    fn different_covers_in_one_folder_are_reported() {
        // FR-12: twee tracks van hetzelfde album horen dezelfde hoes te hebben.
        let files = [complete(1), complete(2), complete(3)];
        let cover = cover();
        let other = ArtInfo {
            width: 500,
            height: 500,
            mime: "image/png".to_string(),
            bytes: 1_860,
        };

        let mut entries = entries(&files, Some(&cover));
        entries[2].art = Some(&other);

        let review = review(&entries);

        assert_eq!(review.folder, vec![FolderIssue::DifferentArt(2)]);
        assert_eq!(
            review.folder[0].describe(),
            "2 verschillende hoezen in deze map"
        );
    }

    #[test]
    fn a_file_without_a_cover_does_not_count_as_a_different_one() {
        // Dat er een hoes ontbreekt, is de melding van dat ene bestand; het
        // maakt de map niet inconsistent.
        let files = [complete(1), complete(2)];
        let cover = cover();
        let mut entries = entries(&files, Some(&cover));
        entries[1].art = None;

        let review = review(&entries);

        assert!(
            !review
                .folder
                .iter()
                .any(|issue| matches!(issue, FolderIssue::DifferentArt(_))),
            "{:?}",
            review.folder
        );
    }

    #[test]
    fn missing_art_is_reported_per_file() {
        let files = [complete(1), complete(2)];
        let cover = cover();
        let mut entries = entries(&files, Some(&cover));
        entries[1].art = None;

        let review = review(&entries);

        assert!(review.tracks[0].is_empty());
        assert_eq!(review.tracks[1], vec![TrackIssue::MissingArt]);
    }

    #[test]
    fn different_albums_are_reported_at_folder_level() {
        let files = [
            tags("Eerste album", "Dezelfde", "1999", Some(1)),
            tags("Tweede album", "Dezelfde", "1999", Some(2)),
        ];

        let review = review(&entries(&files, Some(&cover())));

        assert_eq!(
            review.folder,
            vec![FolderIssue::DifferentValues {
                field: SharedField::Album,
                values: vec!["Eerste album".to_string(), "Tweede album".to_string()],
            }]
        );
        assert!(
            review.tracks.iter().all(|issues| issues.is_empty()),
            "een afwijking tussen bestanden is geen gebrek van één bestand"
        );
    }

    #[test]
    fn album_artist_and_year_are_checked_too() {
        let files = [
            tags("Het Album", "Eerste artiest", "1999", Some(1)),
            tags("Het Album", "Tweede artiest", "2001", Some(2)),
        ];

        let review = review(&entries(&files, Some(&cover())));

        assert_eq!(
            review.folder,
            vec![
                FolderIssue::DifferentValues {
                    field: SharedField::AlbumArtist,
                    values: vec!["Eerste artiest".to_string(), "Tweede artiest".to_string()],
                },
                FolderIssue::DifferentValues {
                    field: SharedField::Year,
                    values: vec!["1999".to_string(), "2001".to_string()],
                },
            ]
        );
    }

    #[test]
    fn a_missing_value_is_not_an_inconsistency() {
        // Eén bestand zonder album naast bestanden mét hetzelfde album: dat is
        // een ontbrekend veld op dat bestand, geen tegenstrijdigheid.
        let mut incomplete = complete(2);
        incomplete.album = None;
        let files = [complete(1), incomplete];

        let review = review(&entries(&files, Some(&cover())));

        assert_eq!(review.tracks[1], vec![TrackIssue::MissingAlbum]);
        assert!(
            review.folder.is_empty(),
            "onterecht als inconsistentie gemeld: {:?}",
            review.folder
        );
    }

    #[test]
    fn missing_and_duplicate_track_numbers_are_reported() {
        let mut without_number = complete(9);
        without_number.track = None;

        let files = [complete(1), complete(1), complete(2), without_number];

        let review = review(&entries(&files, Some(&cover())));

        assert_eq!(review.tracks[0], vec![TrackIssue::DuplicateTrackNumber]);
        assert_eq!(review.tracks[1], vec![TrackIssue::DuplicateTrackNumber]);
        assert!(
            review.tracks[2].is_empty(),
            "nummer 2 komt maar één keer voor"
        );
        assert_eq!(review.tracks[3], vec![TrackIssue::MissingTrackNumber]);

        assert_eq!(
            review.folder,
            vec![
                FolderIssue::MissingTrackNumbers(1),
                FolderIssue::DuplicateTrackNumbers(vec![1]),
            ]
        );
    }

    #[test]
    fn a_single_file_folder_is_never_inconsistent() {
        let files = [complete(1)];

        let review = review(&entries(&files, Some(&cover())));

        assert!(review.folder.is_empty());
        assert!(review.tracks[0].is_empty());
    }

    #[test]
    fn an_empty_folder_has_nothing_to_report() {
        let review = review(&[]);

        assert_eq!(review, Review::default());
    }

    #[test]
    fn descriptions_say_what_is_wrong() {
        assert_eq!(
            FolderIssue::DifferentValues {
                field: SharedField::Album,
                values: vec!["A".to_string(), "B".to_string()],
            }
            .describe(),
            "2 verschillende albumtitels in deze map: “A”, “B”"
        );

        assert_eq!(
            FolderIssue::MissingTrackNumbers(1).describe(),
            "1 bestand heeft geen tracknummer"
        );
        assert_eq!(
            FolderIssue::MissingTrackNumbers(4).describe(),
            "4 bestanden hebben geen tracknummer"
        );

        assert_eq!(
            FolderIssue::DuplicateTrackNumbers(vec![3]).describe(),
            "tracknummer 3 komt meer dan eens voor"
        );
        assert_eq!(
            FolderIssue::DuplicateTrackNumbers(vec![1, 3]).describe(),
            "deze tracknummers komen meer dan eens voor: “1”, “3”"
        );
    }

    #[test]
    fn long_lists_of_values_are_summarised() {
        let values: Vec<String> = (1..=6).map(|number| format!("Album {number}")).collect();

        let described = FolderIssue::DifferentValues {
            field: SharedField::Album,
            values,
        }
        .describe();

        assert_eq!(
            described,
            "6 verschillende albumtitels in deze map: “Album 1”, “Album 2”, “Album 3” en nog 3"
        );
    }

    #[test]
    fn labels_are_short_enough_for_a_list_row() {
        for issue in [
            TrackIssue::MissingTitle,
            TrackIssue::MissingArtist,
            TrackIssue::MissingAlbum,
            TrackIssue::MissingArt,
            TrackIssue::MissingTrackNumber,
            TrackIssue::DuplicateTrackNumber,
        ] {
            let label = issue.label();
            assert!(!label.is_empty());
            assert!(
                label.chars().count() <= 20,
                "'{label}' is te lang voor een label naast een bestandsnaam"
            );
        }
    }
}
