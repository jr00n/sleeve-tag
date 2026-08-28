---
id: TASK-31
title: 'Een hoes op de bewerkpagina neerslepen, op het hoesje zelf'
status: Done
assignee:
  - claude
created_date: '2026-08-28 21:01'
updated_date: '2026-08-28 21:35'
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
- [x] #1 Een afbeelding op het hoesje van de bewerkpagina slepen zet hem klaar, met een voorbeeldweergave en de bestandsnaam
- [x] #2 Tijdens het slepen laat het hoesje zien dat het bestand daar losgelaten kan worden
- [x] #3 Er wordt niets geschreven zonder klik: neerzetten zet de afbeelding klaar, een knop bevestigt
- [x] #4 Na het embedden blijft de gebruiker op de bewerkpagina: het hoesje ververst zich en de uitkomst komt eronder te staan (herzien tijdens de uitvoering; de oorspronkelijke opzet ging naar de hoespagina en vaagde niet-opgeslagen tagvelden weg)
- [x] #5 De ingevulde tagvelden gaan niet mee met de hoesactie, en een hoesactie slaat geen tags op
- [x] #6 Ook een bestand zonder hoes is een doel om iets op te slepen
- [x] #7 Te groot of geen JPEG/PNG geeft dezelfde melding als op de hoespagina, vóór er iets verstuurd wordt
- [x] #8 De hoespagina blijft bereikbaar voor wat daar meer kan: alle tracks, en de losse cover.jpg
- [x] #9 Zonder JavaScript verandert er niets aan de bewerkpagina: dan post het formulier gewoon en volgt de hoespagina met het rapport
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 cargo fmt --check slaagt
- [x] #2 cargo clippy -- -D warnings slaagt
- [x] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [x] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [x] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Bevestigd door de eigenaar op 2026-08-28: het hoesje ververst zich ter plekke en de ingevulde tagvelden blijven staan.

Twee dingen kwamen uit dit testen. Ten eerste landde je na het embedden op de hoespagina, wat niet-opgeslagen tagvelden wegvaagde; het formulier gaat nu op de achtergrond (commit 9d460d9). Ten tweede stond de knop 'In dit bestand zetten' altijd in beeld: `hidden` is niet meer dan `display: none` uit de standaardstijl, en mijn eigen `display: flex` won ervan. Opgelost met één regel bovenaan app.css plus een test die vastlegt dat hij er staat (commit 5fdf6d3).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Het hoesje op de bewerkpagina is een sleepdoel geworden. Een afbeelding erop zetten laat een miniatuur zien met één knop eronder; die knop schrijft.

**Geen tweede route.** Het formulier rond het hoesje post naar dezelfde `/hoes/{pad}` die de hoespagina gebruikt, met `actie=embed-dit`. Het staat náást het tagformulier en niet erin: geneste formulieren bestaan niet in HTML, en een hoesactie hoort geen tags mee te sturen. Een test bewaakt die volgorde.

**Herzien tijdens de uitvoering.** De eerste opzet liet je na het embedden op de hoespagina uitkomen, met het volledige rapport. Bij het proberen bleek dat een gevolg te hebben dat ik niet had voorzien: tagvelden die je had ingevuld maar nog niet opgeslagen, gingen bij die navigatie verloren. Het formulier gaat nu op de achtergrond — het hoesje ververst zich, de uitkomst komt eronder te staan, en je invoer blijft staan. Zonder JavaScript post het gewoon en volgt alsnog de hoespagina.

De uitkomst wordt uit het antwoord gelezen en niet uit de HTTP-status: Sleeve rendert bij een mislukt bestand nog steeds een pagina met status 200, en dan hoort er geen "gelukt" te verschijnen.

**Twee fouten uit het testen.** Een te grote afbeelding leverde een dode pagina op (opgelost met een controle in de browser, TASK-30). En de knop stond altijd in beeld: `hidden` is niet meer dan `display: none` uit de standaardstijl van de browser, en mijn eigen `display: flex` won ervan — verholpen met één regel bovenaan `app.css`, plus een test die vastlegt dat hij er staat.

**Bevestigd door de eigenaar** op 2026-08-28: slepen werkt, het hoesje ververst zich ter plekke, en de tagvelden blijven staan.
<!-- SECTION:FINAL_SUMMARY:END -->
