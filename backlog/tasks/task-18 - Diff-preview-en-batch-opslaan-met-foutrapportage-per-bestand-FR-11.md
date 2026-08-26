---
id: TASK-18
title: Diff-preview en batch-opslaan met foutrapportage per bestand (FR-11)
status: To Do
assignee: []
created_date: '2026-08-26 22:25'
labels: []
milestone: m-3
dependencies:
  - TASK-13
  - TASK-15
  - TASK-16
  - TASK-17
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Een batch-actie raakt in één klap een heel album. De gebruiker moet daarom vooraf precies zien welk bestand welke wijziging krijgt, inclusief velden die verwijderd worden, en achteraf per bestand zien of het gelukt is.

Regels uit het PRD: opslaan gebeurt bestand-voor-bestand; een fout bij één bestand blokkeert de rest niet en wordt per bestand gerapporteerd. Lege invoer betekent 'veld verwijderen' en dat moet in de preview expliciet als verwijdering zichtbaar zijn.

Dit is de enige route waarlangs batch-wijzigingen worden weggeschreven.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Voor het opslaan toont een preview per bestand welke velden wijzigen, met oude en nieuwe waarde
- [ ] #2 Velden die verwijderd worden zijn in de preview expliciet als verwijdering gemarkeerd
- [ ] #3 Bestanden zonder wijzigingen worden als zodanig getoond en worden niet aangeraakt bij het opslaan
- [ ] #4 Opslaan verwerkt de bestanden een voor een; een fout bij een bestand stopt de verwerking van de overige bestanden niet
- [ ] #5 Na afloop toont een resultaatoverzicht per bestand of het gelukt is, met de foutreden bij mislukking
- [ ] #6 De gebruiker kan de batch annuleren vanuit de preview zonder dat er iets geschreven is
- [ ] #7 Een integratietest voert een batch uit op fixture-kopieen waarvan een bestand niet schrijfbaar is, en controleert dat de overige bestanden correct zijn bijgewerkt en de fout gerapporteerd is
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
