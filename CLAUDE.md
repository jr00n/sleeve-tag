# Sleeve (`sleeve-tag`) — werkafspraken

Sleeve is een web-based tag editor voor MP3/FLAC die als één Docker-container op
een UGREEN NAS draait. `PRD.md` is leidend voor scope en eisen; dit bestand legt
de conventies vast waar code zich aan houdt.

## Architectuurregels

- **Alle tag-I/O loopt via `tags::`.** `lofty` wordt uitsluitend binnen die
  module aangeroepen; nergens anders in de codebase. De rest van de applicatie
  werkt alleen met het genormaliseerde tagmodel uit PRD §7 en weet niet of een
  bestand ID3v2-frames of Vorbis-comments bevat. Deze regel wordt afgedwongen
  door `tests/architecture.rs`.
- **Alle padvertaling loopt via `fs::`.** Een door de gebruiker aangeleverd pad
  wordt daar gecanonicaliseerd en gecontroleerd tegen `MUSIC_ROOT`; handlers
  bouwen nooit zelf een pad op.
- **Schrijven is atomisch.** Naar een tijdelijk bestand in dezelfde map,
  hervalideren door opnieuw in te lezen, en pas dan hernoemen over het origineel.
  Bij een fout blijft het origineel onaangetast.
- **Niets ongevraagd wijzigen.** Geen achtergrondjobs, geen opschoonacties, geen
  velden aanraken die de gebruiker niet zelf heeft ingevuld.

## Conventies

- Rust stable, edition 2024. De toolchain ligt vast in `rust-toolchain.toml`.
- Code en identifiers in het Engels; doc-comments en commentaar in het Nederlands,
  net als de UI.
- Configuratie komt uitsluitend uit omgevingsvariabelen. In de container is
  `MUSIC_ROOT` altijd `/music`; het host-pad van de share is de app onbekend.

## Kwaliteitspoort

`cargo fmt --check`, `cargo clippy -- -D warnings` en `cargo test` moeten groen
zijn voordat werk als afgerond geldt.

## Tests

Tests draaien **nooit** tegen de echte muziekbibliotheek. Ze kopiëren fixtures
uit `tests/fixtures/` naar een tempdir en werken daar, via
`testfixtures::kopieer_naar_tempdir(...)`. Een test die `MUSIC_ROOT` op een echt
bibliotheekpad zet, of die rechtstreeks tegen een fixture in de repo werkt, is
per definitie fout.

Integratietests delen hun procesbesturing via `tests/common/mod.rs`: de binary is
een langlopende server, dus wachten op `Command::output()` laat een test hangen.


<!-- BACKLOG.MD MCP GUIDELINES START -->
<!-- backlog.md-instructions-version: 1.50.1 -->

<CRITICAL_INSTRUCTION>

## BACKLOG WORKFLOW INSTRUCTIONS

This project uses Backlog.md MCP for all task and project management activities.

**CRITICAL GUIDANCE**

- If your client supports MCP resources, read `backlog://workflow/overview` to understand when and how to use Backlog for this project.
- If your client only supports tools or the above request fails, call `backlog.get_backlog_instructions()` to load the tool-oriented overview. Use the `instruction` selector when you need `task-creation`, `task-execution`, or `task-finalization`.

- **First time working here?** Read the overview resource IMMEDIATELY to learn the workflow
- **Already familiar?** You should have the overview cached ("## Backlog.md Overview (MCP)")
- **When to read it**: BEFORE creating tasks, or when you're unsure whether to track work

These guides cover:
- Decision framework for when to create tasks
- Search-first workflow to avoid duplicates
- Links to detailed guides for task creation, execution, and finalization
- MCP tools reference

You MUST read the overview resource to understand the complete workflow. The information is NOT summarized here.

</CRITICAL_INSTRUCTION>

<!-- BACKLOG.MD MCP GUIDELINES END -->
