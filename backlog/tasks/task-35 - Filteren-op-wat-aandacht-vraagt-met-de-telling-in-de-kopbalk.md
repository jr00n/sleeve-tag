---
id: TASK-35
title: 'Filteren op wat aandacht vraagt, met de telling in de kopbalk'
status: To Do
assignee: []
created_date: '2026-08-30 07:04'
updated_date: '2026-08-30 07:05'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) zet naast het zoekveld een knop "Needs attention" met een telling erachter. Eén klik laat alleen de bestanden zien waar iets aan mankeert; nog een klik zet de lijst terug.

Sleeve signaleert al per bestand wat er ontbreekt of afwijkt (FR-4) en toont die labels in de lijst, maar er is geen manier om erop te filteren. In een map met honderd bestanden waar er drie een tracknummer missen, moet je die drie nu zelf zoeken.

De telling hoort bij de map die je bekijkt en zegt hoeveel bestanden daar ten minste één signalering hebben. Het filter werkt samen met het zoekveld dat er al is: samen versmallen ze de lijst, ze vervangen elkaar niet.

De signalering blijft constateren en niets meer: dit filter verandert daar niets aan en stelt geen correcties voor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 De mapweergave laat zien hoeveel bestanden in deze map ten minste één signalering hebben.
- [ ] #2 Met één klik toont de lijst alleen die bestanden, en met nog een klik weer alles; de knop laat zien welke van de twee aan staat.
- [ ] #3 Het filter en het zoekveld werken samen: staat er ook een zoekterm, dan blijft over wat aan allebei voldoet.
- [ ] #4 De gekozen stand overleeft het verversen van de pagina en staat in de URL, zodat een gefilterde lijst te delen en te bookmarken is.
- [ ] #5 Zonder JavaScript werkt het filter ook: het is dan een gewone link of knop die de pagina opnieuw laadt.
- [ ] #6 Een map waarin niets aandacht vraagt, zegt dat met zoveel woorden in plaats van een lege lijst te tonen.
- [ ] #7 Het filteren is met tests gedekt, inclusief de combinatie met een zoekterm en een map waarin alles in orde is.
- [ ] #8 README is bijgewerkt.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
