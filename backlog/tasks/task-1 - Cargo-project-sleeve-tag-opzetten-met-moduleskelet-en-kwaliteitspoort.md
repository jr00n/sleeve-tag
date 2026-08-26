---
id: TASK-1
title: Cargo-project sleeve-tag opzetten met moduleskelet en kwaliteitspoort
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:22'
updated_date: '2026-08-26 22:36'
labels: []
milestone: m-0
dependencies: []
documentation:
  - PRD.md
priority: high
type: task
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Sleeve is een web-based tag editor voor MP3/FLAC die als een enkele Docker-container op een UGREEN NAS draait. Deze taak legt het fundament: een Rust-project waarin alle volgende fasen kunnen landen, met een vaste modulegrens die voorkomt dat tag-I/O door de hele codebase lekt.

Technische naam is `sleeve-tag` (crate en binary), weergavenaam in de UI is "Sleeve". Rust stable, edition 2024. Vaste architectuurregel uit het PRD: alle bestandsmutaties en alle `lofty`-aanroepen lopen uitsluitend via een eigen module `tags::`; nergens anders in de code wordt `lofty` direct aangeroepen.

Deze taak levert alleen het skelet en de projectconventies op, geen functionaliteit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cargo-project `sleeve-tag` (edition 2024) bouwt met `cargo build` en levert binary `sleeve-tag`
- [x] #2 De in het PRD genoemde dependencies zijn opgenomen: lofty, axum, tokio, tower-http, askama, image, serde, tracing, tracing-subscriber, een config-crate (clap of envy), anyhow en thiserror
- [x] #3 Er is een moduleskelet met minimaal `config`, `tags`, `fs` (padafhandeling) en `web`, elk met een korte doc-comment die de verantwoordelijkheid beschrijft
- [x] #4 De architectuurregel 'lofty alleen binnen tags::' is vastgelegd in CLAUDE.md
- [x] #5 `cargo fmt --check`, `cargo clippy -- -D warnings` en `cargo test` slagen op een schone checkout
- [x] #6 README beschrijft hoe het project lokaal gebouwd en gedraaid wordt
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 cargo fmt --check slaagt
- [x] #2 cargo clippy -- -D warnings slaagt
- [x] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [x] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [x] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Uitgangssituatie (onderzocht 2026-08-27)

De repo bevat alleen PRD.md, CLAUDE.md (met de backlog-instructieblok) en backlog/. Er is nog geen Cargo-project.
Op de machine ontbrak de Rust-toolchain volledig (geen rustc/cargo/rustup); ffmpeg is wel aanwezig (nodig voor task-4).
Besluit met de eigenaar: rustup wordt via de officiele installer geinstalleerd, stable als default.

## Stappen

1. **Toolchain**: rustup + stable installeren; `rust-toolchain.toml` vastleggen met channel `stable` en components
   `rustfmt` en `clippy`, zodat de kwaliteitspoort op elke machine identiek draait.
2. **Cargo-project**: `cargo init` met crate- en binarynaam `sleeve-tag`, edition 2024.
3. **Dependencies** toevoegen via `cargo add` (laat cargo de actuele versies resolven): lofty, axum, tokio (rt-multi-thread,
   macros), tower-http (fs, trace), askama, image, serde (derive), tracing, tracing-subscriber (env-filter), clap (derive, env),
   anyhow, thiserror. Keuze clap boven envy: het PRD noemt beide, clap geeft met `#[arg(env = ...)]` meteen goede
   foutmeldingen per variabele, wat task-2 als acceptatiecriterium heeft.
4. **Moduleskelet** in src/, elk met doc-comment die de verantwoordelijkheid vastlegt:
   - `config` — configuratie uit omgevingsvariabelen (ingevuld in task-2)
   - `fs` — canonicalisatie en containment binnen MUSIC_ROOT; de enige plek die gebruikerspaden naar filesystem-paden vertaalt (task-6)
   - `tags` — genormaliseerd tagmodel en de enige plek waar lofty wordt aangeroepen (task-7/13)
   - `web` — axum-router, handlers en askama-templates (task-3 e.v.)
   Modules zijn in deze taak bewust leeg op de doc-comments en een minimale placeholder na; functionaliteit komt per fase-taak.
   Modulenaam `fs` blijft zoals in het acceptatiecriterium; binnen die module wordt `std::fs::` altijd volledig gekwalificeerd
   geschreven om verwarring met de crate-eigen module te voorkomen.
5. **main.rs**: minimale binary die tracing initialiseert en afsluit. De echte server komt in task-3, zodat deze taak geen
   functionaliteit vooruitloopt.
6. **CLAUDE.md**: sectie met de projectconventies toevoegen buiten de BACKLOG.MD-markers (die worden door de tooling beheerd):
   architectuurregel "lofty uitsluitend binnen `tags::`", modulegrenzen, kwaliteitspoort, en de regel dat tests nooit tegen de
   echte bibliotheek draaien.
7. **README.md**: wat Sleeve is, lokaal bouwen en draaien met MUSIC_ROOT, en de kwaliteitspoort. De volledige
   installatie-/deploymentdocumentatie is task-26; hier alleen wat een ontwikkelaar nu nodig heeft.
8. **.gitignore**: target/, .env, en macOS-rommel.
9. **Verificatie**: `cargo build`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` alle groen.

## Risico's / aandachtspunten

- Edition 2024 vereist een recente stable toolchain; wordt door de verse rustup-installatie gedekt.
- Dependencies worden in deze taak toegevoegd maar nog niet gebruikt. Dat is expliciet acceptatiecriterium #2 en levert geen
  clippy-waarschuwing op, zolang de lint `unused_crate_dependencies` niet wordt aangezet.
- Nog geen tests met inhoud in deze fase; `cargo test` slaagt met nul tests. Fixtures en testhelpers zijn task-4.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Toolchain ontbrak volledig op de machine. Na overleg met de eigenaar rustup via de officiele installer gedraaid; resultaat rustc/cargo 1.98.0 stable. `rust-toolchain.toml` legt channel + rustfmt/clippy vast zodat CI en NAS-builds dezelfde toolchain gebruiken.

Config-crate: clap gekozen boven envy. Het PRD noemt beide; clap geeft met `#[arg(env = ...)]` per variabele een foutmelding die de naam noemt, wat acceptatiecriterium #3 van de configuratietaak vraagt.

image-crate toegevoegd met `default-features = false, features = ["jpeg", "png"]`. De app ondersteunt alleen die twee formaten, en de niet-meegecompileerde decoders schelen fors in de binary — relevant voor de eis van een image onder 30 MB.

De architectuurregel 'lofty alleen binnen tags::' is niet alleen gedocumenteerd maar afgedwongen via `tests/architecture.rs`, die de bronbestanden buiten src/tags/ scant. De guard is negatief geverifieerd: met een tijdelijke `use lofty::...` in src/web/mod.rs faalde de test, na herstel slaagt hij weer.

Modulenaam `fs` aangehouden zoals in het acceptatiecriterium. In die module wordt `std::fs::` altijd volledig gekwalificeerd geschreven om verwarring met de crate-eigen module te voorkomen; dat staat als conventie in de doc-comment.

Nog niet gecommit — de eigenaar heeft daar niet om gevraagd. De volledige werkstructuur staat untracked klaar.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Wat er is gebouwd

Het fundament van `sleeve-tag`: een Cargo-project (edition 2024) met een moduleskelet waarin de latere fasen kunnen landen, plus de projectconventies en een werkende kwaliteitspoort. Bewust geen functionaliteit — de binary zet logging op en sluit af.

## Wijzigingen

- **Cargo-project** `sleeve-tag` (crate + binary), edition 2024, met alle dependencies uit het PRD: lofty 0.25, axum 0.8, tokio 1.53, tower-http 0.7, askama 0.16, image 0.25 (alleen jpeg/png), serde, tracing(+subscriber), clap, anyhow, thiserror.
- **Moduleskelet** `config`, `fs`, `tags`, `web`, elk met een doc-comment die de verantwoordelijkheid en de bijbehorende vervolgtaak benoemt.
- **`rust-toolchain.toml`** met channel stable en de componenten rustfmt/clippy, zodat de kwaliteitspoort overal identiek draait.
- **CLAUDE.md** uitgebreid met de werkafspraken: tag-I/O uitsluitend via `tags::`, padvertaling uitsluitend via `fs::`, atomisch schrijven, niets ongevraagd wijzigen, en de regel dat tests nooit tegen de echte bibliotheek draaien.
- **README.md** met ontwikkelinstructies op macOS, de kwaliteitspoort en de modulestructuur.
- **`.gitignore`** voor target/, .env en macOS-rommel.

## Tests

4 tests, alle groen:
- 3 unit-tests op `log_directive` (terugval op `info` bij ontbrekende of lege `LOG_LEVEL`, respecteren van een ingestelde waarde).
- `tests/architecture.rs` dwingt af dat `lofty` nergens buiten `src/tags/` voorkomt. Negatief geverifieerd: met een tijdelijke lofty-verwijzing in `src/web/mod.rs` faalt de test, na herstel slaagt hij.

`cargo build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` en `cargo test` zijn alle groen. De binary draait en logt zijn start.

## Aandachtspunten

- De dependencies zijn toegevoegd maar nog niet gebruikt; dat is expliciet acceptatiecriterium #2 en levert geen clippy-waarschuwing op zolang `unused_crate_dependencies` uit blijft.
- Rust is tijdens deze taak op de machine geinstalleerd (rustup, stable 1.98.0). Nieuwe shells hebben `~/.cargo/bin` in PATH; in een bestaande shell is `. "$HOME/.cargo/env"` nodig.
- Er is nog niet gecommit.
<!-- SECTION:FINAL_SUMMARY:END -->
