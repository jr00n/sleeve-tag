---
id: TASK-14
title: 'Bewerkformulier per bestand met opslaan en teruglezen (FR-5, FR-6)'
status: To Do
assignee: []
created_date: '2026-08-26 22:24'
labels: []
milestone: m-2
dependencies:
  - TASK-8
  - TASK-13
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De kern van de app voor de gebruiker: één bestand openen, velden corrigeren, opslaan, en direct bevestigd zien dat het gelukt is. Na opslaan toont de app de opnieuw uit het bestand ingelezen waarden, niet de zojuist ingetypte waarden, zodat zichtbaar is wat er werkelijk in het bestand staat.

Kernvelden: titel, artiest, albumartiest, album, tracknummer en totaal, discnummer en totaal, jaar, genre, componist, commentaar.

Het formulier moet op een telefoonscherm bruikbaar zijn en duidelijk maken wanneer een leeg veld betekent dat de tag verwijderd wordt.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Vanuit de maplijst is per bestand een bewerkformulier te openen met alle kernvelden gevuld met de huidige waarden
- [ ] #2 Opslaan schrijft de wijzigingen weg en toont daarna de opnieuw uit het bestand ingelezen waarden ter bevestiging
- [ ] #3 Een veld leegmaken verwijdert de tag, en de UI maakt dat vooraf duidelijk
- [ ] #4 Een mislukte schrijfactie toont een begrijpelijke foutmelding en laat de ingevulde waarden staan zodat de gebruiker het opnieuw kan proberen
- [ ] #5 Ongeldige invoer (bijv. niet-numeriek tracknummer) wordt afgevangen voordat er geschreven wordt
- [ ] #6 Het formulier is bruikbaar op een telefoonscherm
- [ ] #7 Een integratietest bewerkt een fixture-kopie via de HTTP-laag en controleert de teruggelezen waarden
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
