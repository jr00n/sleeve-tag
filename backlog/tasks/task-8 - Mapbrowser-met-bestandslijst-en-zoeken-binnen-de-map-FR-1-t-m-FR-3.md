---
id: TASK-8
title: Mapbrowser met bestandslijst en zoeken binnen de map (FR-1 t/m FR-3)
status: To Do
assignee: []
created_date: '2026-08-26 22:23'
labels: []
milestone: m-1
dependencies:
  - TASK-3
  - TASK-6
  - TASK-7
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De gebruiker moet vanaf tablet of telefoon door de muziekshare kunnen bladeren en per map zien welke tracks er staan met hun belangrijkste tags. Dit is de hoofdnavigatie van de app en het startpunt van elke bewerksessie.

Per map: submappen en de MP3/FLAC-bestanden met tracknummer, titel, artiest, album, duur en formaat, plus ruimte voor de art-thumbnail (aparte taak). Zoeken/filteren gebeurt binnen de huidige map op bestandsnaam of titel. De bibliotheek is typisch `Artiest/Album/track.ext` maar niet gegarandeerd consistent, dus de weergave mag niets over de mapstructuur aannemen.

Aanname voor de standaardsortering (open punt in PRD §12): sorteren op tracknummer uit de tags, met bestandsnaam als terugval wanneer een tracknummer ontbreekt.

Prestatie-eis: een map met 30 tracks laadt in minder dan een seconde op de NAS. Tags worden lazy en per map gelezen; er is bewust geen bibliotheek-index in het MVP.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een mappagina toont submappen en bewerkbare audiobestanden van de opgevraagde map, startend bij MUSIC_ROOT
- [ ] #2 Navigatie boven MUSIC_ROOT is niet mogelijk en er is een broodkruimelpad om terug te navigeren
- [ ] #3 Per bestand worden tracknummer, titel, artiest, album, duur en formaat getoond
- [ ] #4 Bestanden zijn standaard gesorteerd op tracknummer met bestandsnaam als terugval
- [ ] #5 Zoeken/filteren binnen de huidige map werkt op bestandsnaam en op titel
- [ ] #6 De lijst is bruikbaar op een telefoonscherm
- [ ] #7 Een map met 30 tracks rendert in minder dan een seconde op de NAS
- [ ] #8 Een integratietest laadt een testmap met fixtures en controleert de getoonde velden, de sortering en het filter
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
