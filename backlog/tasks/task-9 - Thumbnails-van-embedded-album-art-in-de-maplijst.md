---
id: TASK-9
title: Thumbnails van embedded album art in de maplijst
status: To Do
assignee: []
created_date: '2026-08-26 22:23'
labels: []
milestone: m-1
dependencies:
  - TASK-7
  - TASK-8
documentation:
  - PRD.md
priority: medium
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In de maplijst hoort per bestand een kleine weergave van de embedded hoes, zodat direct zichtbaar is welke tracks art missen of afwijkende art hebben. De afbeeldingen komen uit de bestanden zelf; er is geen cache-laag in het MVP.

Aandachtspunt is de prestatie-eis van FR-2 in combinatie met §8.5: een map met 30 tracks moet binnen een seconde laden, dus thumbnails mogen het renderen van de pagina niet blokkeren.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Er is een endpoint dat de embedded front cover van een bestand als afbeelding serveert, met correcte content-type header
- [ ] #2 In de maplijst wordt per bestand een thumbnail getoond, en een duidelijke placeholder wanneer er geen art is
- [ ] #3 Het laden van thumbnails blokkeert het renderen van de maplijst niet
- [ ] #4 Een verzoek om art van een bestand zonder art geeft een nette 404 in plaats van een fout
- [ ] #5 Een integratietest controleert het endpoint voor een fixture met en zonder embedded art
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
