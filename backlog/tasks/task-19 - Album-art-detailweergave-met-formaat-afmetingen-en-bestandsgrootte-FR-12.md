---
id: TASK-19
title: 'Album art detailweergave met formaat, afmetingen en bestandsgrootte (FR-12)'
status: Done
assignee: []
created_date: '2026-08-26 22:25'
updated_date: '2026-08-28 13:12'
labels: []
milestone: m-4
dependencies:
  - TASK-9
  - TASK-14
documentation:
  - PRD.md
modified_files:
  - src/cover.rs
  - src/checks.rs
  - src/browse.rs
  - src/edit.rs
  - src/tags/mod.rs
  - src/web/mod.rs
  - src/main.rs
  - src/testfixtures.rs
  - templates/cover.html
  - templates/edit.html
  - templates/listing.html
  - static/app.css
  - tests/art.rs
  - tests/architecture.rs
  - tests/fixtures/genereer-fixtures.sh
  - README.md
  - CLAUDE.md
priority: medium
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Voordat de gebruiker een hoes vervangt, wil hij zien wat er nu in zit en of dat goed genoeg is. Een lage-resolutie of enorme JPEG is met het blote oog niet te onderscheiden in een thumbnail, dus de detailweergave toont de art groot met de technische eigenschappen erbij.

Deze weergave is het startpunt van alle art-acties in fase 4: vervangen, verkleinen, als cover.jpg wegschrijven en verwijderen.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 De huidige embedded front cover van een bestand is groot te bekijken
- [x] #2 Bij de afbeelding worden formaat (JPEG/PNG), afmetingen in pixels en bestandsgrootte getoond
- [x] #3 Voor een bestand zonder embedded art toont de weergave dat expliciet, met de mogelijkheid om art toe te voegen
- [x] #4 Wanneer de tracks in een map verschillende art hebben, is dat zichtbaar
- [x] #5 Een integratietest controleert de getoonde eigenschappen voor een fixture met embedded art
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 cargo fmt --check slaagt
- [x] #2 cargo clippy -- -D warnings slaagt
- [x] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [x] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [x] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Nieuwe module `cover::` met het weergavemodel: formaat uit het MIME-type,
afmetingen, en de omvang leesbaar opgemaakt (duizendtallen met een komma, zoals
de Finder). Ze opent geen bestanden en raakt geen pixels aan; in gaat de
`ArtInfo` die `tags::read` al meelevert, dus de pagina kost niet meer dan het
openen van het bestand.

Route `/hoes/{*path}`, bereikbaar vanaf de bewerkpagina (de hoes zelf en een
tekstlink). Een bestand zonder hoes levert bewust geen 404 maar dezelfde pagina
met de mededeling.

AC #4 zit in de signalering en niet op deze pagina: `checks::Entry` draagt nu de
`ArtInfo` in plaats van een `bool`, en `FolderIssue::DifferentArt` telt de
verschillende hoezen in een map. De maplijst toont folder issues al, dus de
melding verschijnt daar vanzelf. De vingerafdruk is (type, breedte, hoogte,
bytes); bestanden zonder hoes tellen niet mee, die hebben hun eigen melding.

`TrackSummary.has_art: bool` is `art: Option<ArtInfo>` geworden, met `has_art()`
als methode — één bron van waarheid.

Twee dingen meegenomen die hier thuishoorden: `tags::describe_art` las de
afmetingen zelf met de image-crate, in strijd met de regel dat pixelbewerking
via `art::` loopt; dat gaat nu langs `art::dimensions`, en een tweede
architectuurtest bewaakt het.

Nieuwe fixture `tagged-with-other-art.mp3` (500×500 PNG in plaats van 300×300
JPEG), zodat AC #4 end-to-end te testen is; het genereerscript is bijgewerkt.

AC #3 is half: de weergave toont expliciet dat er geen hoes is, maar er art
kunnen toevoegen is FR-13 en hoort bij task-21. De plek en de tekst staan er
klaar voor.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: Claude
created: 2026-08-28 13:12
---
AC #3 is nu volledig: task-21 heeft het uploadformulier op deze pagina gezet, dus een bestand zonder hoes toont niet alleen dát er geen is maar biedt ook de manier om er een toe te voegen.
---
<!-- COMMENTS:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
De hoesweergave van één bestand (FR-12): `/hoes/<pad>` toont de embedded hoes
zo groot als het scherm toelaat, met formaat, afmetingen en bestandsgrootte, en
met de kanttekening als hij niet vierkant is. Een bestand zonder hoes krijgt
dezelfde pagina met de mededeling dat er niets in zit.

De maplijst signaleert voortaan ook dat de tracks in een map verschillende
hoezen hebben, vergeleken op type, afmetingen en omvang.

**AC #3 is half geleverd**: het expliciet tonen dat er geen hoes is, is klaar;
art kunnen toevoegen is FR-13 en hoort bij task-21, waar het uploaden en
embedden in zijn geheel zit. De pagina houdt de plek ervoor vrij.

Nieuwe module `cover::` met 4 unit-tests, 3 nieuwe unit-tests in `checks::` en
6 nieuwe integratietests, plus een fixture met een afwijkende hoes. Meegenomen:
`tags::` las de afmetingen van een hoes zelf met de image-crate en gaat nu via
`art::`; een tweede architectuurtest bewaakt die regel. `cargo fmt --check`,
`cargo clippy -- -D warnings` en `cargo test` (228 + 11 + overige) zijn groen.
Commit 9547f96.
<!-- SECTION:FINAL_SUMMARY:END -->
