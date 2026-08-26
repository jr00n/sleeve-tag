---
id: TASK-15
title: Albumweergave met selectie en gedeelde velden in een keer zetten (FR-8)
status: To Do
assignee: []
created_date: '2026-08-26 22:24'
labels: []
milestone: m-3
dependencies:
  - TASK-14
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Bestand voor bestand corrigeren is te traag voor een heel album. De gebruiker wil meerdere bestanden in een map selecteren (of alles) en de velden die het album deelt in één keer zetten: albumartiest, album, jaar, genre en disc.

Dit is de basis van fase 3; per-bestand overrides, hulpacties en de diff-preview bouwen hierop voort. Het daadwerkelijk wegschrijven gebeurt in de diff-preview-taak, zodat er nooit zonder voorbeeld geschreven wordt.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In de mapweergave zijn afzonderlijke bestanden te selecteren en is er een 'alles selecteren'-actie
- [ ] #2 De selectie blijft behouden tijdens het invullen van de gedeelde velden
- [ ] #3 Voor de gedeelde velden albumartiest, album, jaar, genre en disc kan in een keer een waarde voor de hele selectie worden opgegeven
- [ ] #4 Een gedeeld veld dat leeg wordt gelaten blijft ongemoeid; er is een expliciete manier om een veld voor de hele selectie te wissen
- [ ] #5 Wanneer de geselecteerde bestanden voor een gedeeld veld verschillende waarden hebben, is dat zichtbaar in de invoer
- [ ] #6 De weergave werkt op een telefoonscherm, waarbij de tabel horizontaal mag scrollen
- [ ] #7 Integratietests dekken selectie, gedeelde-veldinvoer en het onderscheid tussen 'leeg laten' en 'wissen'
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
