---
id: TASK-22
title: Album art ook als cover.jpg in de albummap wegschrijven (FR-14)
status: To Do
assignee: []
created_date: '2026-08-26 22:26'
labels: []
milestone: m-4
dependencies:
  - TASK-20
  - TASK-21
documentation:
  - PRD.md
priority: medium
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Navidrome en vrijwel alle spelers pakken een `cover.jpg` in de albummap op, ook wanneer embedded art ontbreekt of afwijkt. De gebruiker moet daarom bij het instellen van een hoes kunnen kiezen om die ook als bestand in de map te zetten.

Dit is de enige plek waar de app een nieuw bestand in de bibliotheek aanmaakt in plaats van een bestaand bestand te wijzigen; het overschrijven van een bestaande cover.jpg moet dus bewust gebeuren en dezelfde eigendoms- en permissieregels volgen als de rest van de share.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Bij het instellen van album art is er een optie om de afbeelding ook als cover.jpg in de albummap te schrijven
- [ ] #2 Een bestaande cover.jpg wordt alleen overschreven na expliciete bevestiging door de gebruiker
- [ ] #3 De weggeschreven cover.jpg krijgt dezelfde eigenaar, groep en permissies als de overige bestanden in de map
- [ ] #4 Het schrijven verloopt atomisch, zodat een afgebroken actie geen half bestand achterlaat
- [ ] #5 Een fout bij het schrijven van cover.jpg wordt gemeld maar maakt een geslaagd embedden niet ongedaan
- [ ] #6 Een integratietest schrijft cover.jpg in een testmap en controleert inhoud en overschrijfgedrag
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
