---
id: TASK-30
title: Een hoes kunnen neerslepen op de hoespagina
status: To Do
assignee: []
created_date: '2026-08-28 20:21'
updated_date: '2026-08-28 20:22'
labels: []
milestone: m-6
dependencies: []
priority: medium
type: enhancement
ordinal: 24700
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Een hoes uploaden gaat nu via `<input type="file">` op de hoespagina: bladeren, in een bestandskiezer zoeken, openen. Wie een afbeelding al in een venster ernaast heeft staan — een browsertab met de hoes, de Finder, een map met scans — wil hem er gewoon op kunnen slepen.

Het gaat om een toevoeging op het bestaande formulier (`templates/cover.html`), niet om een nieuwe route: het slepen vult hetzelfde bestandsveld dat er al is. De keuzevakjes eromheen (`mapbestand`, `overschrijf`) en de knoppen "Alleen dit bestand" / "alle tracks" blijven bepalen wat er met de afbeelding gebeurt. Er wordt dus nog steeds niets geschreven zonder dat de gebruiker een knop indrukt.

De server hoeft niets nieuws te kunnen: een neergesleept bestand komt via `DataTransfer` in dezelfde `<input type="file">` terecht en gaat als hetzelfde multipart-formulier de deur uit. `art::prepare` valideert al op de bytes zelf, dus een gesleept bestand krijgt precies dezelfde behandeling als een gekozen bestand — ook wanneer het geen JPEG of PNG blijkt.

Sluit aan bij de bezig-weergave uit TASK-28: ook hier hoort zichtbaar te zijn dat er gewerkt wordt zodra er wordt ingediend.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een afbeelding op het uploadvak slepen vult het bestaande bestandsveld, zodat het formulier verder ongewijzigd werkt
- [ ] #2 Tijdens het slepen laat het vak zien dat het bestand daar losgelaten kan worden
- [ ] #3 Na het neerzetten is te zien welk bestand gekozen is, met naam en een voorbeeldweergave
- [ ] #4 Slepen voegt niets toe zonder klik: het uploaden gebeurt nog steeds pas bij Alleen dit bestand of alle tracks
- [ ] #5 Iets anders dan een afbeelding neerzetten geeft meteen een melding in plaats van een mislukte upload
- [ ] #6 Meerdere bestanden tegelijk neerzetten neemt er niet stilzwijgend een van; het zegt dat er een hoes tegelijk gaat
- [ ] #7 Zonder JavaScript blijft het bestandsveld gewoon werken; slepen is een toevoeging en geen voorwaarde
- [ ] #8 Op een aanraakscherm verandert er niets ten opzichte van nu
- [ ] #9 Een bestand ergens anders op de pagina laten vallen opent het niet in het browservenster
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
