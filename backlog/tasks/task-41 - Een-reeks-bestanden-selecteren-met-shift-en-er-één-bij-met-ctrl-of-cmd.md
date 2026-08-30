---
id: TASK-41
title: 'Een reeks bestanden selecteren met shift, en er één bij met ctrl of cmd'
status: To Do
assignee: []
created_date: '2026-08-30 07:13'
updated_date: '2026-08-30 07:13'
labels: []
dependencies:
  - TASK-35
  - TASK-38
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: enhancement
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) laat een selectie maken zoals in een bestandsbeheerder: klikken op een regel selecteert die, shift-klikken selecteert alles ertussen, en ctrl- of cmd-klikken haalt er één bij of weg.

Sleeve heeft alleen vinkjes. Twintig tracks van een schijf aanvinken is daarmee twintig klikken, terwijl het er twee zouden kunnen zijn.

De vinkjes blijven: ze zijn de weg voor wie geen muis gebruikt, en ze zijn wat er overblijft zonder JavaScript. Dit is een toevoeging bovenop wat er is, geen vervanging ervan.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Klikken op een regel selecteert dat bestand en niets anders; shift-klikken selecteert alles tussen die regel en de vorige klik.
- [ ] #2 Ctrl- of cmd-klikken haalt één bestand bij de selectie of eruit, zonder de rest aan te tasten.
- [ ] #3 Een reeks volgt de volgorde zoals de lijst er op dat moment uitziet, ook wanneer er gefilterd of gegroepeerd is.
- [ ] #4 Klikken in een invoerveld of op een vinkje verandert de selectie niet op een manier die het intikken in de weg zit.
- [ ] #5 De vinkjes blijven werken zoals ze deden en blijven de weg voor wie geen muis gebruikt; de selectie is ook met het toetsenbord te maken.
- [ ] #6 Zonder JavaScript verandert er niets: de vinkjes doen dan het werk, zoals nu.
- [ ] #7 Het selecteren is met tests gedekt voor zover dat zonder browser kan; wat alleen in de browser te zien is, staat als zodanig beschreven.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
