---
id: TASK-27
title: MVP-acceptatie valideren op de UGREEN NAS met de echte bibliotheek
status: To Do
assignee: []
created_date: '2026-08-26 22:26'
labels: []
milestone: m-5
dependencies:
  - TASK-18
  - TASK-21
  - TASK-22
  - TASK-24
  - TASK-26
documentation:
  - PRD.md
priority: high
type: task
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De acceptatiecriteria uit PRD §10 gaan over gedrag dat alleen op de echte NAS met de echte share te bewijzen valt: permissies, Navidrome die de wijziging oppikt, en het gedrag bij een afgebroken schrijfactie. Deze taak is de afsluitende validatie van het MVP.

Werk met een kopie van een album als testonderwerp waar dat kan; de crash-test (container afbreken tijdens een schrijfactie) moet expliciet op een kopie gebeuren, nooit op een uniek bestand.

Bevindingen die uit deze validatie komen worden als aparte taken vastgelegd, niet stilzwijgend hier opgelost.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 De container start op de UGREEN met `docker compose up -d` en de UI is bereikbaar via http://<nas>:<port> en via Tailscale
- [ ] #2 Een MP3-album en een FLAC-album zijn vanaf een tablet volledig gecorrigeerd: velden, tracknummers en hoes
- [ ] #3 De bewerkte bestanden hebben na afloop dezelfde eigenaar en permissies als daarvoor
- [ ] #4 Na de reguliere scan van Navidrome toont Navidrome de nieuwe metadata en hoes, zonder handmatige actie in Sleeve
- [ ] #5 Het bewust afbreken van de container tijdens een schrijfactie op een testkopie laat geen beschadigd of half geschreven bestand achter
- [ ] #6 Het geheugengebruik van het proces op de NAS blijft onder 30 MB tijdens normaal gebruik
- [ ] #7 De uitkomsten van elke controle zijn vastgelegd in de taaknotities, en afwijkingen zijn als aparte taken aangemaakt
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
