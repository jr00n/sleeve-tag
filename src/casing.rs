//! Hoofdlettergebruik van een tagwaarde normaliseren (PRD FR-10).
//!
//! Een bibliotheek die uit verschillende bronnen komt, staat vol met
//! `STILTE IN D` en `stilte in d` naast elkaar. Deze module maakt daar
//! `Stilte in D` van: elk woord met een hoofdletter, behalve de kleine woorden
//! die er middenin staan.
//!
//! Wat er níét gebeurt is minstens zo belangrijk. Een afkorting blijft een
//! afkorting (`DJ`, `R.E.M.`, `AC/DC`), en een naam die zijn eigen
//! hoofdletters draagt blijft zoals hij is (`McCartney`, `iPhone`, `d'Angelo`).
//! Sleeve raadt hier, en raden hoort zichtbaar en terug te draaien te zijn:
//! [`crate::batch`] zet de uitkomst als voorstel in de invoervelden en schrijft
//! niets.
//!
//! Deze module kent geen tags en geen bestanden: in en uit gaat tekst.

/// Woorden die middenin een titel klein blijven.
///
/// Nederlands en Engels door elkaar, want een bibliotheek is dat ook. Aan het
/// begin of het eind van de tekst krijgen ze wél een hoofdletter: "Van Halen"
/// en "Waar Ik Van Hou" horen allebei te kloppen.
const SMALL_WORDS: &[&str] = &[
    // Nederlands
    "de", "het", "een", "en", "of", "van", "der", "den", "des", "te", "ten", "ter", "tot", "met",
    "bij", "aan", "op", "in", "om", "uit", "voor", "door", "over", "als", "dan", "naar", "je",
    "ze", "'t", "'n", // Engels
    "the", "a", "an", "and", "or", "nor", "but", "of", "on", "at", "to", "for", "from", "with",
    "by", "into", "onto", "as", "than", "per", "via", "vs",
];

/// Normaliseert het hoofdlettergebruik van één tekstwaarde.
///
/// Losse spaties aan de randen en dubbele spaties ertussen verdwijnen meteen:
/// wie een titel toch al laat herschrijven, heeft aan `Stilte  in D` niets.
pub fn normalize(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let last = words.len().saturating_sub(1);

    words
        .iter()
        .enumerate()
        .map(|(position, word)| {
            let edge = position == 0 || position == last;

            if is_small_word(word) {
                if edge {
                    capitalize(word)
                } else {
                    word.to_lowercase()
                }
            } else if keeps_its_own_casing(word) {
                (*word).to_string()
            } else {
                capitalize(word)
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// Of dit woord middenin klein hoort te blijven.
///
/// De leestekens eromheen tellen niet mee, zodat `(van` net zo goed herkend
/// wordt als `van`.
fn is_small_word(word: &str) -> bool {
    let bare = word
        .trim_matches(|character: char| !character.is_alphanumeric() && character != '\'')
        .to_lowercase();

    SMALL_WORDS.contains(&bare.as_str())
}

/// Of dit woord met rust gelaten moet worden.
///
/// Twee gevallen, en allebei even hardnekkig als je ze wél aanraakt:
///
/// - een korte reeks hoofdletters is vrijwel altijd een afkorting of een
///   bandnaam (`DJ`, `EP`, `USA`, `ABBA`, `R.E.M.`, `AC/DC`). Vier letters is
///   de grens: `BEATLES` in kapitalen is geen afkorting maar geschreeuw, en
///   daar is deze actie juist voor bedoeld;
/// - een woord met een hoofdletter verderop draagt zijn eigen vorm
///   (`McCartney`, `iPhone`, `d'Angelo`).
fn keeps_its_own_casing(word: &str) -> bool {
    let letters: Vec<char> = word
        .chars()
        .filter(|character| character.is_alphabetic())
        .collect();
    if letters.is_empty() {
        // Een streepje of een cijfer valt niets aan te passen.
        return true;
    }

    let all_uppercase = letters.iter().all(|letter| letter.is_uppercase());
    if all_uppercase {
        return letters.len() <= 4;
    }

    letters[1..].iter().any(|letter| letter.is_uppercase())
}

/// Eén woord met een hoofdletter, en de rest klein.
///
/// De hoofdletter gaat naar de eerste letter en niet naar het eerste teken:
/// `(live)` hoort `(Live)` te worden.
fn capitalize(word: &str) -> String {
    let mut seen_letter = false;

    word.chars()
        .flat_map(|character| {
            if character.is_alphabetic() && !seen_letter {
                seen_letter = true;
                character.to_uppercase().collect::<Vec<char>>()
            } else {
                character.to_lowercase().collect::<Vec<char>>()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shouted_title_becomes_readable() {
        assert_eq!(normalize("STILTE IN D"), "Stilte in D");
        assert_eq!(normalize("stilte in d"), "Stilte in D");
    }

    #[test]
    fn a_title_that_is_already_right_comes_back_unchanged() {
        // De actie stelt alleen iets voor waar iets te veranderen valt.
        for text in [
            "Stilte in D",
            "De Nachtwacht",
            "Waar Ik van Hou",
            "Live at the BBC",
        ] {
            assert_eq!(normalize(text), text, "{text}");
        }
    }

    #[test]
    fn small_words_stay_small_in_the_middle() {
        assert_eq!(normalize("het lied van de zee"), "Het Lied van de Zee");
        assert_eq!(normalize("a night at the opera"), "A Night at the Opera");
    }

    #[test]
    fn a_small_word_at_the_edge_still_gets_a_capital() {
        // "Van Halen" is een naam, en "Waar Ik Van Hou" eindigt er niet zomaar
        // op; aan de randen wint de hoofdletter.
        assert_eq!(normalize("van halen"), "Van Halen");
        assert_eq!(normalize("the boy in"), "The Boy In");
        assert_eq!(normalize("de"), "De");
    }

    #[test]
    fn an_abbreviation_keeps_its_capitals() {
        // AC #6: bestaande afkortingen zijn geen schrijffout.
        assert_eq!(normalize("live at the BBC"), "Live at the BBC");
        assert_eq!(normalize("DJ shadow"), "DJ Shadow");
        assert_eq!(normalize("R.E.M. unplugged"), "R.E.M. Unplugged");
        assert_eq!(normalize("AC/DC live"), "AC/DC Live");
        assert_eq!(normalize("symphony no. II"), "Symphony No. II");
    }

    #[test]
    fn shouting_is_not_an_abbreviation() {
        // Vijf letters of meer in kapitalen is geen afkorting; precies waar
        // deze actie voor bestaat.
        assert_eq!(normalize("BEATLES forever"), "Beatles Forever");
    }

    #[test]
    fn a_name_that_carries_its_own_capitals_is_left_alone() {
        // AC #6: namen met een hoofdletter verderop mogen niet platgeslagen
        // worden.
        assert_eq!(normalize("paul McCartney"), "Paul McCartney");
        assert_eq!(normalize("iPhone sessies"), "iPhone Sessies");
        assert_eq!(normalize("d'Angelo live"), "d'Angelo Live");
        assert_eq!(normalize("van der Graaf"), "Van der Graaf");
    }

    #[test]
    fn punctuation_does_not_hide_a_word() {
        assert_eq!(normalize("(live in parijs)"), "(Live in Parijs)");
        assert_eq!(normalize("zomer, winter"), "Zomer, Winter");
    }

    #[test]
    fn stray_spaces_disappear() {
        assert_eq!(normalize("  stilte   in  d  "), "Stilte in D");
        assert_eq!(normalize("   "), "");
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn accents_survive() {
        assert_eq!(normalize("café DE nuit"), "Café de Nuit");
        assert_eq!(normalize("ÉTUDE nr 3"), "Étude Nr 3");
    }

    #[test]
    fn a_number_is_not_a_word_to_capitalise() {
        assert_eq!(normalize("track 3 & 4"), "Track 3 & 4");
    }
}
