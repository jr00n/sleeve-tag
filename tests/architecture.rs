//! Bewaakt twee architectuurregels uit CLAUDE.md: alle tag-I/O loopt via
//! `tags::`, en alle pixelbewerking via `art::`.
//!
//! De regel staat in de documentatie, maar documentatie vergeet je; deze test
//! niet. Zodra een handler of hulpmodule rechtstreeks naar `lofty` grijpt, faalt
//! de build in plaats van dat het bij review moet worden opgemerkt.

use std::path::{Path, PathBuf};

/// Verzamelt alle `.rs`-bestanden onder `dir`.
fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("src-map moet leesbaar zijn") {
        let path = entry.expect("map-entry moet leesbaar zijn").path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn lofty_is_used_only_inside_tags() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let tags = src.join("tags");

    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(!files.is_empty(), "er zijn geen bronbestanden gevonden");

    for file in files {
        if file.starts_with(&tags) || file == src.join("tags.rs") {
            continue;
        }

        let inhoud = std::fs::read_to_string(&file).expect("bronbestand moet leesbaar zijn");
        assert!(
            !inhoud.contains("lofty"),
            "{} verwijst naar lofty; tag-I/O hoort uitsluitend in tags::",
            file.display()
        );
    }
}

#[test]
fn the_image_crate_is_used_only_inside_art() {
    // De tegenhanger van de regel hierboven: `tags::` levert de ruwe bytes van
    // een hoes, en wat er daarna met die pixels gebeurt, gebeurt in `art::`.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let art = src.join("art.rs");

    let mut files = Vec::new();
    rust_files(&src, &mut files);

    for file in files {
        if file == art {
            continue;
        }

        let inhoud = std::fs::read_to_string(&file).expect("bronbestand moet leesbaar zijn");
        assert!(
            !inhoud.contains("image::"),
            "{} gebruikt de image-crate; pixelbewerking hoort uitsluitend in art::",
            file.display()
        );
    }
}
