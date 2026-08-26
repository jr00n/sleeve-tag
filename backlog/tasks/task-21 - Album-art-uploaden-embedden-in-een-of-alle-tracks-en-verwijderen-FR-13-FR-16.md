---
id: TASK-21
title: >-
  Album art uploaden, embedden in een of alle tracks, en verwijderen (FR-13,
  FR-16)
status: To Do
assignee: []
created_date: '2026-08-26 22:26'
labels: []
milestone: m-4
dependencies:
  - TASK-13
  - TASK-19
  - TASK-20
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De belangrijkste art-actie: vanaf tablet of telefoon een hoes uploaden en die in één track of in alle geselecteerde tracks van het album embedden, plus het kunnen verwijderen van bestaande art.

De art wordt weggeschreven als front cover (APIC type 3 voor MP3, METADATA_BLOCK_PICTURE type 3 voor FLAC) via de bestaande tags-module en de atomische schrijfhelper, zodat dezelfde integriteitsgaranties gelden als voor tekstuele tags. Verwerking van de afbeelding zelf (validatie en verkleinen) gebeurt door de beeldverwerkingslaag.

Net als bij batch-tagbewerking geldt: een fout bij één bestand blokkeert de overige bestanden niet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een JPEG of PNG kan vanuit de browser worden geupload en in het geopende bestand worden geembed als front cover
- [ ] #2 Dezelfde geuploade art kan in een keer in alle geselecteerde tracks van een album worden geembed
- [ ] #3 Bestaande embedded art kan uit een bestand of uit alle geselecteerde bestanden verwijderd worden
- [ ] #4 Na embedden of verwijderen toont de app de opnieuw ingelezen situatie ter bevestiging
- [ ] #5 Bij het embedden in meerdere bestanden wordt per bestand gerapporteerd of het gelukt is; een fout blokkeert de rest niet
- [ ] #6 De overige tags van de bewerkte bestanden blijven onveranderd
- [ ] #7 Integratietests dekken embedden in een MP3- en een FLAC-fixture, embedden in meerdere bestanden, en verwijderen
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
