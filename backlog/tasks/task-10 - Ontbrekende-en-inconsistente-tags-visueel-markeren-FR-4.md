---
id: TASK-10
title: Ontbrekende en inconsistente tags visueel markeren (FR-4)
status: To Do
assignee: []
created_date: '2026-08-26 22:23'
labels: []
milestone: m-1
dependencies:
  - TASK-8
documentation:
  - PRD.md
priority: medium
type: feature
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het doel van de app is snel kunnen corrigeren. Daarvoor moet de maplijst zelf aanwijzen waar iets mis is, zonder dat de gebruiker elk bestand hoeft te openen.

Te signaleren gevallen: ontbrekende kernvelden (titel, artiest, album), ontbrekende album art, en waarden die binnen dezelfde map onderling afwijken terwijl ze gelijk horen te zijn (bijvoorbeeld meerdere albumtitels of albumartiesten in één map), plus ontbrekende of dubbele tracknummers.

De markering is puur informatief; de app past nooit ongevraagd iets aan.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Bestanden met ontbrekende kernvelden of ontbrekende album art zijn in de maplijst visueel gemarkeerd
- [ ] #2 Waarden die binnen een map onderling afwijken (album, albumartiest, jaar) worden als inconsistentie gemarkeerd op mapniveau
- [ ] #3 Ontbrekende en dubbele tracknummers binnen een map worden gesignaleerd
- [ ] #4 Bij elke markering is zichtbaar wat er precies aan de hand is (bijv. via tooltip of tekstlabel)
- [ ] #5 De markering wijzigt niets aan de bestanden
- [ ] #6 Unit-tests dekken de detectielogica met testmappen die consistent, deels inconsistent en volledig leeg getagd zijn
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
