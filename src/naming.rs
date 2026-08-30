//! Wat er in een bestandsnaam over de titel staat.
//!
//! Voor een bestand zonder titeltag is de bestandsnaam de enige plek waar de
//! titel nog staat. Deze module leest hem daaruit, en verder niets: ze kent
//! geen tags en opent geen bestanden — in en uit gaat tekst.
//!
//! Net als [`crate::casing`] raadt ze. Wat ze raadt hoort zichtbaar en terug te
//! draaien te zijn, en daarom is de uitkomst een voorstel in een invoerveld en
//! nooit een schrijfactie.

/// De tekens die tussen een leidend tracknummer en de titel mogen staan.
const SEPARATORS: [char; 6] = ['-', '–', '.', ')', ']', '_'];

/// De titel zoals die uit een bestandsnaam te lezen valt.
///
/// De extensie gaat eraf, underscores worden spaties, en een leidend
/// tracknummer met zijn scheidingsteken verdwijnt: `03 - Kind of Blue.flac`
/// levert `Kind of Blue`. Blijft er niets over, dan valt er geen titel uit deze
/// naam te halen en komt er `None` terug — beter niets voorstellen dan een
/// leeg veld voorstellen.
pub fn title_from_filename(name: &str) -> Option<String> {
    let stem = stem_of(name)?;

    let spaced: String = stem
        .chars()
        .map(|letter| if letter == '_' { ' ' } else { letter })
        .collect();

    let title = collapse(strip_leading_number(&spaced));
    if title.is_empty() { None } else { Some(title) }
}

/// De bestandsnaam zonder extensie.
///
/// Blijft er dan niets over, dan bestond de naam alleen uit een extensie en
/// valt er niets te lezen.
fn stem_of(name: &str) -> Option<&str> {
    let stem = match name.rsplit_once('.') {
        Some((stem, _)) => stem,
        None => name,
    };

    if stem.trim().is_empty() {
        None
    } else {
        Some(stem)
    }
}

/// Haalt een leidend tracknummer met zijn scheidingsteken weg.
///
/// Hoogstens drie cijfers, want een tracknummer van vier cijfers bestaat niet
/// en een jaartal wel: `2001 A Space Odyssey` houdt zo zijn begin. Er moet een
/// scheidingsteken of spatie op volgen, anders is het geen nummer maar het
/// begin van een woord (`12Stones`).
///
/// Blijft er na het nummer niets over, dan bestaat de naam alleen uit een
/// tracknummer en komt er niets terug: een nummer is geen titel.
fn strip_leading_number(text: &str) -> &str {
    let mut rest = text;

    loop {
        let trimmed = rest.trim_start();

        let digits = trimmed
            .char_indices()
            .take_while(|(index, letter)| *index < 3 && letter.is_ascii_digit())
            .count();
        if digits == 0 {
            return rest;
        }

        let after = &trimmed[digits..];
        if after.trim().is_empty() {
            return "";
        }

        let stripped = after.trim_start_matches(|letter: char| {
            letter.is_whitespace() || SEPARATORS.contains(&letter)
        });

        // Zonder scheidingsteken hoort het cijfer bij het woord erachter.
        if stripped.len() == after.len() {
            return rest;
        }

        if stripped.trim().is_empty() {
            return "";
        }

        rest = stripped;
    }
}

/// Trimt en klapt opeenvolgende spaties in tot één.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_extension_and_leading_track_number() {
        assert_eq!(
            title_from_filename("03 - Kind of Blue.flac").as_deref(),
            Some("Kind of Blue")
        );
        assert_eq!(
            title_from_filename("01. Blue in Green.mp3").as_deref(),
            Some("Blue in Green")
        );
        assert_eq!(
            title_from_filename("07_So_What.mp3").as_deref(),
            Some("So What")
        );
        assert_eq!(
            title_from_filename("12) Flamenco Sketches.mp3").as_deref(),
            Some("Flamenco Sketches")
        );
    }

    #[test]
    fn keeps_a_name_that_is_only_a_title() {
        assert_eq!(
            title_from_filename("So What.flac").as_deref(),
            Some("So What")
        );
    }

    #[test]
    fn keeps_a_year_at_the_start() {
        // Vier cijfers is geen tracknummer; die naam begint met een jaartal.
        assert_eq!(
            title_from_filename("2001 A Space Odyssey.mp3").as_deref(),
            Some("2001 A Space Odyssey")
        );
    }

    #[test]
    fn strips_a_disc_and_track_prefix() {
        assert_eq!(
            title_from_filename("1-04 - Milestones.flac").as_deref(),
            Some("Milestones")
        );
    }

    #[test]
    fn has_nothing_to_offer_for_a_name_without_a_title() {
        // Alleen een nummer: dat is geen titel maar een tracknummer.
        assert_eq!(title_from_filename("01.mp3"), None);
        assert_eq!(title_from_filename("07 - .flac"), None);
        assert_eq!(title_from_filename(".mp3"), None);
        assert_eq!(title_from_filename("   .flac"), None);
        assert_eq!(title_from_filename(""), None);
    }

    #[test]
    fn a_number_glued_to_a_word_stays_put() {
        assert_eq!(
            title_from_filename("12Stones.mp3").as_deref(),
            Some("12Stones")
        );
    }
}
