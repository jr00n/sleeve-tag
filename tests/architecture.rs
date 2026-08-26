//! Bewaakt de architectuurregel uit CLAUDE.md: alle tag-I/O loopt via `tags::`.
//!
//! De regel staat in de documentatie, maar documentatie vergeet je; deze test
//! niet. Zodra een handler of hulpmodule rechtstreeks naar `lofty` grijpt, faalt
//! de build in plaats van dat het bij review moet worden opgemerkt.

use std::path::{Path, PathBuf};

/// Verzamelt alle `.rs`-bestanden onder `dir`.
fn rust_bestanden(dir: &Path, gevonden: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("src-map moet leesbaar zijn") {
        let pad = entry.expect("directory-entry moet leesbaar zijn").path();
        if pad.is_dir() {
            rust_bestanden(&pad, gevonden);
        } else if pad.extension().is_some_and(|ext| ext == "rs") {
            gevonden.push(pad);
        }
    }
}

#[test]
fn lofty_wordt_uitsluitend_binnen_tags_gebruikt() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let tags = src.join("tags");

    let mut bestanden = Vec::new();
    rust_bestanden(&src, &mut bestanden);
    assert!(!bestanden.is_empty(), "er zijn geen bronbestanden gevonden");

    for bestand in bestanden {
        if bestand.starts_with(&tags) || bestand == src.join("tags.rs") {
            continue;
        }

        let inhoud = std::fs::read_to_string(&bestand).expect("bronbestand moet leesbaar zijn");
        assert!(
            !inhoud.contains("lofty"),
            "{} verwijst naar lofty; tag-I/O hoort uitsluitend in tags::",
            bestand.display()
        );
    }
}
