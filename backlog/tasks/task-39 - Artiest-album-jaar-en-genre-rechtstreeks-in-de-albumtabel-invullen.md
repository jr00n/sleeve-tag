---
id: TASK-39
title: 'Artiest, album, jaar en genre rechtstreeks in de albumtabel invullen'
status: To Do
assignee: []
created_date: '2026-08-30 07:13'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) laat elke kolom in de bestandstabel bewerken: titel, artiest, album, jaar en genre, plus track- en discnummer.

Sleeve kent nu twee wegen: gedeelde velden voor de hele selectie, en per bestand alleen titel en tracknummer. Een compilatie waarin elke track een andere artiest heeft, is daardoor alleen bestand voor bestand te doen — terwijl de tabel er al staat.

De regel die er al is, blijft gelden: wat per bestand wordt ingetikt, is een override en wint van wat de gedeelde velden voor datzelfde bestand zouden doen.

Buiten scope: nieuwe velden die Sleeve nog niet kent, en het bewerken van ruwe frames.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Artiest, album, jaar en genre zijn per bestand in de tabel zelf in te tikken, net als titel en tracknummer nu.
- [ ] #2 Wat er per bestand wordt ingetikt, wint van wat de gedeelde velden voor datzelfde bestand zouden doen.
- [ ] #3 Leeg laten betekent in de tabel hetzelfde als nu bij titel en tracknummer: het bestand houdt wat het heeft; wissen blijft een aparte, expliciete keuze.
- [ ] #4 Een fout in één rij houdt alleen die rij tegen en niet de rest van de batch, en wordt bij die rij gemeld.
- [ ] #5 De tabel blijft op een telefoon bruikbaar: hij scrollt binnen zijn eigen rand en de pagina zelf scrollt niet horizontaal mee.
- [ ] #6 Wat er per bestand wordt ingetikt, komt terug in de voorbeeldweergave en wordt daar per veld getoond zoals de andere wijzigingen.
- [ ] #7 De overrides per veld zijn met tests gedekt, inclusief het samenspel met een gedeeld veld dat hetzelfde veld raakt.
- [ ] #8 README en, waar de regels veranderen, CLAUDE.md zijn bijgewerkt.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
