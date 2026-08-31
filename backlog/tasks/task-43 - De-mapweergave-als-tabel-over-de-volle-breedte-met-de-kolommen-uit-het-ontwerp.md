---
id: TASK-43
title: >-
  De mapweergave als tabel over de volle breedte, met de kolommen uit het
  ontwerp
status: Done
assignee:
  - Claude
created_date: '2026-08-30 21:01'
updated_date: '2026-08-30 21:36'
labels: []
dependencies:
  - TASK-42
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
modified_files:
  - src/browse.rs
  - templates/listing.html
  - templates/directory.html
  - templates/base.html
  - static/app.css
  - tests/browse.rs
  - README.md
type: enhancement
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp (Claude Design, project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) toont de bestanden van een map als tabel met tien kolommen: een vinkje, tracknummer, disc, een kleine hoes, titel, artiest, album, jaar, genre en lengte. Onder de titel staan de bestandsnaam en de signaleringen als kleine labels. De tabel vult de breedte van het scherm en scrollt horizontaal binnen zijn eigen omhulsel zodra hij niet meer past. Boven de tabel staat de mapnaam met de telling en het aantal schijven ernaast.

Sleeve toont diezelfde bestanden nu als een `<ul>` met gestapelde regels: hoes, tracknummer, en daarnaast titel, artiest · album, bestandsnaam en signaleringen onder elkaar, met formaat en duur rechts. Alles staat in een kolom van 960 pixels (`.inhoud { max-width: 60rem }`), gecentreerd op een breed scherm. Je kunt de bestanden dus niet per kolom vergelijken — precies waar een tabel voor is als je wilt zien welk bestand uit de pas loopt.

De albumweergave heeft al wél een tabel, maar zonder hoes- en lengtekolom, en met het discnummer als tekst.

Deze taak gaat over de mapweergave: hoe de bestanden worden getoond en hoeveel ruimte de pagina daarvoor neemt. Wat de regels doen verandert niet — de titel blijft de ingang naar het bewerkformulier, en de schijfkoppen blijven de groepen scheiden zoals ze dat nu doen. Het bewerken ín de tabel hoort bij de taak die de tag-editor naast de lijst zet en valt hier buiten.

Let op de bestaande regel dat de mapweergave niets ongevraagd opent: wat een regel toont komt uit de tags die al voor de lijst gelezen worden, niet uit een extra leesronde.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 De bestanden van een map staan in een tabel met de kolommen uit het ontwerp, in die volgorde, met de bestandsnaam en de signaleringen onder de titel.
- [x] #2 De pagina gebruikt de volle breedte in plaats van een kolom van 960 pixels, zonder dat de leesbaarheid van de tekstblokken eronder lijdt.
- [x] #3 De tabel scrollt horizontaal binnen zijn eigen omhulsel zodra hij niet past; de pagina zelf schuift nooit horizontaal mee.
- [x] #4 Op een telefoon blijft de lijst bruikbaar: wat niet past is bereikbaar, en de bestandsnaam blijft altijd zichtbaar.
- [x] #5 De schijfkoppen blijven de groepen scheiden en staan als eigen regel in de tabel, met dezelfde telling en aandachtsmelding als nu.
- [x] #6 De titel blijft de ingang naar het bewerkformulier van dat ene bestand.
- [x] #7 Er wordt geen enkel bestand extra geopend om de tabel te kunnen tonen; wat er staat komt uit dezelfde leesronde als nu.
- [x] #8 De weergave is met tests gedekt, inclusief een map zonder discnummers, een map met meerdere schijven en een bestand zonder hoes.
- [x] #9 README is bijgewerkt.
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

De maplijst wordt een tabel in `templates/listing.html` (het fragment dat HTMX
ook los terugkrijgt), de mappagina krijgt de volle breedte, en het
weergavemodel levert de drie velden die er nog niet als label uitkwamen. Er
wordt geen bestand extra geopend: alles komt uit de `Tags` die al in
`TrackSummary` zitten.

1. **`src/browse.rs`** — `TrackSummary::disc_label()`, `year_label()` en
   `genre_label()`, in de stijl van de bestaande labels: een nummer dat
   ontbreekt wordt leeg (een streepje per regel in een smalle kolom is ruis),
   tekst die ontbreekt wordt `—`. Daarnaast `Listing::summary_label()` voor de
   regel naast de mapnaam ("10 bestanden · 3 schijven"), met de bestaande
   `is_grouped()` als voorwaarde voor het schijvendeel. Unit-tests erbij.
2. **`templates/listing.html`** — de `<ul class="tracks">` wordt een `<table
   class="maptabel">` in een `.tabelrand`, hetzelfde omhulsel met
   `tabindex="0"` en `role="region"` dat de batchtabel al gebruikt. Kolommen in
   de volgorde van het ontwerp: tracknummer, disc, hoes, titel, artiest, album,
   jaar, genre, lengte — met formaat als tiende, omdat FR-2 dat expliciet eist
   en het ontwerp die kolom niet kent. De bestandsnaam en de signaleringen
   staan onder de titel in dezelfde cel. De schijfkop wordt een eigen rij
   (`<th colspan>`), net als in de batchtabel.
3. **Geen vinkjeskolom.** Het ontwerp zet er één, maar de mapweergave heeft
   geen selectie: die hoort bij `/album`, en of `/map` en `/album` samenvallen
   is precies de open vraag van TASK-44 (AC #1 daar). Een vinkje dat nergens
   heen post zou een bedieningselement zijn dat niets doet. Wordt bij TASK-44
   beslist.
4. **`templates/base.html` en `static/app.css`** — `<main>` krijgt een blok
   voor zijn breedteklasse; `directory.html` zet daar `inhoud--breed` in, zodat
   alleen de mapweergave de volle breedte neemt en het bewerkformulier, de hoes
   en de voorbeeldweergave hun leeskolom houden. Binnen die brede pagina houden
   de tekstblokken (`.signalering`, `.leeg`) wél een leesbare maximumbreedte.
   Nieuwe `.maptabel`-regels naar het model van `.batchtabel`; de
   `.track*`-regels en de `.schijf*`-regels verdwijnen met de `<ul>` mee.
   De titelkolom staat stil (`position: sticky; left: 0`) zodat de bestandsnaam
   in beeld blijft terwijl de rest horizontaal scrolt.
5. **Tests** (`tests/browse.rs`) — de kolomkoppen in de volgorde van het
   ontwerp, jaar en genre in een rij, de schijfkop als rij in de tabel bij
   meerdere schijven, geen kop in een map zonder discnummers, een bestand
   zonder hoes met de placeholder, en het omhulsel dat horizontaal scrolt.
   Voor wat alleen in de browser te zien is: een test die in de geserveerde
   `app.css` vaststelt dat de titelkolom stil blijft staan.
6. **README** — de alinea over de maplijst beschrijft de tabel, de kolommen en
   de volle breedte.

## Grenzen

- Bewerken ín de tabel valt buiten deze taak (TASK-44).
- Er wordt niets extra gelezen: `browse::` opent geen bestand meer dan nu.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Afwijking bij AC #1: geen vinkjeskolom.** Het ontwerp zet er tien kolommen neer waarvan de eerste een vinkje is; deze tabel heeft er tien zonder dat vinkje en met `Formaat` erachter. Reden: de mapweergave kent geen selectie — die hoort bij `/album` — en of `/map` en `/album` samenvallen is precies de open vraag die TASK-44 AC #1 aan de gebruiker voorlegt. Een vinkje dat nergens heen post zou een bedieningselement zijn dat niets doet, en dat is in dit project een uitgesproken keuze om niet te doen. De kolom `Formaat` is er wél, omdat FR-2 het formaat expliciet bij de belangrijkste velden noemt; hij staat achteraan zodat de volgorde van het ontwerp intact blijft.

Wat alleen in een browser te zien is — dat de tabel binnen zijn omhulsel scrolt en dat de titelkolom stil blijft staan — is als stijlregel getest (`the_table_scrolls_inside_its_own_wrapper` leest de geserveerde `app.css`), niet met een echte browser.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
De bestanden van een map staan nu in een tabel over de volle breedte, met de
kolommen uit het ontwerp. Er wordt niets extra gelezen: alle waarden komen uit
de `Tags` die al in `TrackSummary` zaten.

**`src/browse.rs`** — `TrackSummary::disc_label()`, `year_label()` en
`genre_label()` erbij, in de stijl van de bestaande labels: een ontbrekend
getal wordt leeg, een ontbrekend tekstveld `—`. Het jaar blijft tekst, zodat
een volledige datum niet sneuvelt. `Listing::summary_label()` levert de regel
naast de mapnaam ("3 bestanden · 1 schijf"); de groep zonder discnummer telt
daar niet als schijf mee, en een lege lijst levert `None` — daar staat al een
uitleg. Twee unit-tests erbij.

**`templates/listing.html`** — de `<ul class="tracks">` is een `<table
class="maptabel">` in de `.tabelrand` geworden, hetzelfde omhulsel met
`tabindex="0"` en `role="region"` dat de albumweergave al gebruikte. Kolommen:
`#`, `Disc`, `Hoes`, `Titel`, `Artiest`, `Album`, `Jaar`, `Genre`, `Lengte` en
— buiten het ontwerp om, want FR-2 eist het — `Formaat`. De bestandsnaam en de
signaleringen staan onder de titel in dezelfde cel; de titel blijft de link naar
`/bewerk/<pad>`. De schijfkop is een eigen rij (`<th colspan>`) met dezelfde
telling en aandachtsmelding als voorheen, en gebruikt dezelfde `groep__*`-klassen
als de albumweergave.

**Geen vinkjeskolom.** Bewust weggelaten; zie de notitie bij deze taak. Kort:
de mapweergave heeft geen selectie, en of `/map` en `/album` samenvallen is de
open vraag van TASK-44.

**`templates/base.html` / `directory.html` / `static/app.css`** — `<main>` heeft
een blok voor zijn breedteklasse gekregen; alleen de mapweergave zet daar
`inhoud--breed` in, zodat het bewerkformulier, de hoespagina en de
voorbeeldweergave hun leeskolom van 60rem houden. Binnen de brede pagina houden
`.signalering`, `.leeg` en `.uitleg` wél die leesbreedte. De `.track*`-regels
zijn vervangen door `.maptabel*` naar het model van `.batchtabel`; de
`.schijf*`-regels zijn opgegaan in de gedeelde `groep__*`-regels. De titelkolom
staat stil (`position: sticky; left: 0`) zodat de bestandsnaam in beeld blijft
terwijl de rest horizontaal scrolt, met de mapnaam en de telling in een nieuwe
`.mapkop`.

**Tests** (`tests/browse.rs`) — vier nieuwe: de kolomkoppen in de volgorde van
het ontwerp (binnen `<thead>` gezocht, want "Artiest" is ook een mapnaam in het
broodkruimelpad) met jaar, genre, de bestandsnaam onder de titel en de link naar
het bewerkformulier; het omhulsel dat scrolt plus de stijlregels die dat en de
stille titelkolom vastleggen; een bestand zonder hoes dat de placeholder krijgt
en geen verzoek uitlokt; en de schijfkop als rij in de tabel. De bestaande test
op een map zonder discnummers controleert nu ook dat er geen koprij staat.

**README** — een eigen paragraaf "De bestanden als tabel" met de tien kolommen,
waarom `Formaat` erbij staat en het vinkje niet, de volle breedte en het
scrollen binnen het omhulsel.

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` en `cargo test`
zijn groen (15 testbinaries, 0 failed).
<!-- SECTION:FINAL_SUMMARY:END -->
