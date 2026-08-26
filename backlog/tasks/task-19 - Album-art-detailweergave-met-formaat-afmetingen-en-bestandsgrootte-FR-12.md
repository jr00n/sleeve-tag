---
id: TASK-19
title: 'Album art detailweergave met formaat, afmetingen en bestandsgrootte (FR-12)'
status: To Do
assignee: []
created_date: '2026-08-26 22:25'
labels: []
milestone: m-4
dependencies:
  - TASK-9
  - TASK-14
documentation:
  - PRD.md
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
- [ ] #1 De huidige embedded front cover van een bestand is groot te bekijken
- [ ] #2 Bij de afbeelding worden formaat (JPEG/PNG), afmetingen in pixels en bestandsgrootte getoond
- [ ] #3 Voor een bestand zonder embedded art toont de weergave dat expliciet, met de mogelijkheid om art toe te voegen
- [ ] #4 Wanneer de tracks in een map verschillende art hebben, is dat zichtbaar
- [ ] #5 Een integratietest controleert de getoonde eigenschappen voor een fixture met embedded art
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
