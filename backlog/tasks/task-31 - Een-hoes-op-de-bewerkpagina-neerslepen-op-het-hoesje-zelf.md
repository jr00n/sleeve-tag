---
id: TASK-31
title: 'Een hoes op de bewerkpagina neerslepen, op het hoesje zelf'
status: In Progress
assignee:
  - claude
created_date: '2026-08-28 21:01'
updated_date: '2026-08-28 21:02'
labels: []
milestone: m-6
dependencies:
  - TASK-30
priority: medium
type: enhancement
ordinal: 24800
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Een hoes vervangen kost nu drie stappen: vanaf het bewerkformulier doorklikken naar de hoespagina, daar de afbeelding kiezen, en terug. Terwijl het hoesje op de bewerkpagina al staat — precies waar je die nieuwe afbeelding naartoe zou willen slepen.

Dit bouwt voort op TASK-30: het neerzetvak, de controle op type en omvang en de voorbeeldweergave staan al in `static/app.js`. Wat erbij komt is een tweede plek waar dat vak dienst doet, namelijk het hoesje in `templates/edit.html`.

Er komt geen nieuwe route bij. De bewerkpagina krijgt een klein formulier dat naar dezelfde `/hoes/{pad}` post die de hoespagina al gebruikt, met `actie=embed-dit`. Dat formulier staat verborgen tot er werkelijk een afbeelding klaarstaat, en het staat náást het tagformulier en niet erin: geneste formulieren bestaan niet in HTML, en de tags horen niet mee te liften op een hoesactie.

Het slepen mag niets schrijven. Neerzetten laat zien wát er klaarstaat; wat ermee gebeurt beslist de gebruiker met een knop, net als overal elders in Sleeve. Wie meer wil dan "in dit ene bestand" — de hele map, of ook als `cover.jpg` ernaast — houdt de hoespagina, en die blijft één klik weg.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een afbeelding op het hoesje van de bewerkpagina slepen zet hem klaar, met een voorbeeldweergave en de bestandsnaam
- [ ] #2 Tijdens het slepen laat het hoesje zien dat het bestand daar losgelaten kan worden
- [ ] #3 Er wordt niets geschreven zonder klik: neerzetten zet de afbeelding klaar, een knop bevestigt
- [ ] #4 Na het embedden komt de gebruiker op de hoespagina uit, met het rapport van wat er gebeurd is
- [ ] #5 De ingevulde tagvelden gaan niet mee met de hoesactie, en een hoesactie slaat geen tags op
- [ ] #6 Ook een bestand zonder hoes is een doel om iets op te slepen
- [ ] #7 Te groot of geen JPEG/PNG geeft dezelfde melding als op de hoespagina, vóór er iets verstuurd wordt
- [ ] #8 De hoespagina blijft bereikbaar voor wat daar meer kan: alle tracks, en de losse cover.jpg
- [ ] #9 Zonder JavaScript verandert er niets aan de bewerkpagina
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Aanpak

Hergebruik, geen nieuwbouw. `app.js` kent sinds TASK-30 al een neerzetvak dat
een bestandsveld vult, type en omvang controleert en een voorbeeld toont. Het
hoesje op de bewerkpagina wordt zo'n vak; er komt alleen bij dat het formulier
eromheen verborgen blijft tot er iets klaarstaat.

1. `edit::EditPage` krijgt `max_upload_mb`, want de controle op omvang gebeurt
   in de browser (zie TASK-30: een te grote upload wordt afgekapt terwijl de
   browser nog verstuurt, en dan komt de uitleg van de server nooit aan).
2. `templates/edit.html`: het hoesje komt in een `<form>` naar `page.cover_url`
   met `enctype="multipart/form-data"`, een verborgen bestandsveld, en een knop
   `actie=embed-dit` die pas verschijnt als er een afbeelding klaarstaat. Dat
   formulier staat vóór het tagformulier, niet erin — geneste formulieren
   bestaan niet, en een hoesactie hoort geen tags mee te sturen.
3. `app.js`: generaliseer wat er al is. Wordt een bestand geaccepteerd, dan
   worden de elementen met `data-neerzetvak-klaar` zichtbaar; wordt het
   geweigerd, dan verdwijnen ze weer. Zo bepaalt de template wat er verschijnt
   en hoeft het script niets van hoezen of tags te weten.
4. De knop krijgt `data-bezig` uit TASK-28: embedden in een groot bestand duurt
   net zo lang als elke andere schrijfactie.

## Waarom niet meteen uploaden bij het neerzetten

Dat zou een schrijfactie zijn die niemand heeft bevestigd, en het zou de keuze
"dit bestand of de hele map" stilzwijgend voor de gebruiker maken. Overal in
Sleeve geldt: een hulpactie zet iets klaar, een knop voert het uit.

## Wat hier niet komt

De losse `cover.jpg` en "alle tracks". Die vragen om keuzes (overschrijven?
hoeveel bestanden?) die op de hoespagina thuishoren, en die pagina blijft één
klik weg. Deze snelkoppeling dekt het gewone geval: één hoes, dit ene bestand.
<!-- SECTION:PLAN:END -->
