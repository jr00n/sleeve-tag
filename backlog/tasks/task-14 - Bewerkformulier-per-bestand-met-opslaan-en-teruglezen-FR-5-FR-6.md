---
id: TASK-14
title: 'Bewerkformulier per bestand met opslaan en teruglezen (FR-5, FR-6)'
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:24'
updated_date: '2026-08-27 22:16'
labels: []
milestone: m-2
dependencies:
  - TASK-8
  - TASK-13
documentation:
  - PRD.md
modified_files:
  - src/main.rs
  - src/edit.rs
  - src/browse.rs
  - src/web/mod.rs
  - src/atomic.rs
  - templates/edit.html
  - templates/listing.html
  - static/app.css
  - tests/edit.rs
  - tests/common/mod.rs
  - README.md
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
- [x] #1 Vanuit de maplijst is per bestand een bewerkformulier te openen met alle kernvelden gevuld met de huidige waarden
- [x] #2 Opslaan schrijft de wijzigingen weg en toont daarna de opnieuw uit het bestand ingelezen waarden ter bevestiging
- [x] #3 Een veld leegmaken verwijdert de tag, en de UI maakt dat vooraf duidelijk
- [x] #4 Een mislukte schrijfactie toont een begrijpelijke foutmelding en laat de ingevulde waarden staan zodat de gebruiker het opnieuw kan proberen
- [x] #5 Ongeldige invoer (bijv. niet-numeriek tracknummer) wordt afgevangen voordat er geschreven wordt
- [x] #6 Het formulier is bruikbaar op een telefoonscherm
- [x] #7 Een integratietest bewerkt een fixture-kopie via de HTTP-laag en controleert de teruggelezen waarden
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

### 1. `src/edit.rs` (nieuw) — formulier en paginamodel
- `Form`: alle twaalf kernvelden als `String`, want een leeg invoerveld komt als lege tekst binnen en niet als "afwezig". Dat is precies het onderscheid dat het model nodig heeft.
- `Form::from_tags` vult het formulier; `Form::to_tags` gaat de andere kant op en valideert daarbij de vier numerieke velden (AC #5). Fouten komen terug als leesbare meldingen, niet als een 400.
- `EditPage`: naam, broodkruimels, formaat, duur, hoes, de velden, en een eventuele melding (bevestiging of fout).

### 2. `src/web/mod.rs` — twee routes
- `GET /bewerk/{*path}` toont het formulier met de huidige waarden.
- `POST /bewerk/{*path}` valideert, schrijft via `tags::write`, leest daarna **opnieuw uit het bestand** en toont die waarden (AC #2). Wat er op het scherm komt is dus wat er werkelijk in het bestand staat, niet wat de gebruiker net intikte.
- Bij een validatiefout of een mislukte schrijfactie blijven de ingevulde waarden staan met een uitleg erboven (AC #4).
- `AppState` krijgt de schrijfopties uit `BACKUP_ON_WRITE`.

Bewust geen redirect na het opslaan: het herladen van een POST is hier ongevaarlijk, want `tags::write` doet niets wanneer er niets verandert. Dat scheelt een flash-mechanisme voor de bevestiging.

### 3. Navigatie opschonen
De maplijst linkt nu per regel naar de geavanceerde weergave. Die ingang verhuist naar de bestandspagina, waar FR-7 hem beschrijft; de regel in de lijst gaat naar het bewerkformulier. `TrackSummary.raw_url` wordt `edit_url`.

### 4. Formulier op een telefoon (AC #3, #6)
- Eén kolom, ruime invoervelden, `inputmode="numeric"` op de nummervelden.
- Boven het formulier één zin die zegt wat een leeg veld betekent: de tag wordt uit het bestand verwijderd. Dat is geen bijzin — het is het enige gedrag dat een gebruiker kan verrassen.
- Samenhangende velden (nummer/totaal, disc/totaal) naast elkaar wanneer er ruimte is, onder elkaar wanneer niet.

### 5. Tests
- Unit in `edit`: heen en weer tussen model en formulier, lege velden worden `None`, spaties tellen als leeg, en elk numeriek veld levert een leesbare fout bij onzin.
- Unit in `web`: het formulier toont de huidige waarden; een POST schrijft en toont de teruggelezen waarden; ongeldige invoer schrijft niets en houdt de invoer vast; een pad buiten de bibliotheek wordt geweigerd.
- Integratie (AC #7): via de echte binary een fixture-kopie bewerken met een POST, en controleren dat het antwoord de nieuwe waarden toont én dat het bestand op schijf ze werkelijk bevat.
- Een mislukte schrijfactie laat de ingevulde waarden staan.

### 6. Documentatie
README: het bewerkformulier in de sectie over de mapbrowser, met de nadruk op "wat je ziet is wat er in het bestand staat". Het `#![allow(dead_code)]` in `atomic` kan nu weg: er is eindelijk een handler die schrijft.

### Afwijkingen van het plan

- Niet gepland: de poortrace in `tests/common/mod.rs` moest voor de derde keer aangepakt worden, nu met de werkelijke oorzaak (zie de implementatienotities). De diagnostische foutmelding die daarvoor is toegevoegd blijft staan — die maakte het verschil tussen gokken en weten.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**De poortrace in de testharnas is nu pas echt opgelost, en de oorzaak was iets anders dan ik twee keer eerder dacht.** Met een vijfde integratiebinary viel ongeveer één op de drie volledige runs om. De diagnostische foutmelding die ik toevoegde wees het aan: de server logde `webserver luistert address=0.0.0.0:57389` én de test kreeg `ConnectionReset` na nul bytes.

De verklaring is BSD-gedrag: op macOS mag een socket op `127.0.0.1:P` naast een bestaande socket op `0.0.0.0:P` bestaan wanneer `SO_REUSEADDR` aanstaat, en dat zet Rust standaard aan. `free_port()` bond op de loopback en kreeg dus een poort terug die een dráaiende testserver al gebruikte. De verbinding naar `127.0.0.1:P` kwam vervolgens uit bij die net weer gesloten proef-socket in plaats van bij de server — en werd meteen gereset.

`free_port()` bindt nu op `0.0.0.0`, hetzelfde adres als de server. Twee sockets op `0.0.0.0:P` weigert de kernel, dus een poort die een andere testserver draait komt er nooit meer uit. Vijftien opeenvolgende volledige runs daarna zonder uitval.

De eerdere pogingen (task-9 en task-12) waren niet fout, maar dekten andere gaten: een kindproces dat stierf, en een verbinding met de server van een andere test. Dit was het derde gat.

**Bewust geen redirect na het opslaan.** Het gebruikelijke POST/Redirect/GET voorkomt dat een herlaadactie het formulier opnieuw verstuurt. Hier is dat niet nodig: `tags::write` raakt het bestand niet aan wanneer er niets verandert, dus dezelfde waarden een tweede keer versturen doet niets. Dat scheelt een flash-mechanisme om de bevestiging te bewaren, en de integratietest `saving_twice_leaves_the_file_alone_the_second_time` legt vast dat het klopt.

**Validatie vóór het bestand.** De numerieke velden worden in `edit::Form::to_tags` gecontroleerd, niet tijdens het schrijven. Een typefout in een tracknummer hoort geen schrijfactie te starten die halverwege afketst — ook al zou `atomic::replace` dat netjes opvangen. Alle fouten komen tegelijk terug, niet één per keer: veld voor veld corrigeren omdat de app maar één fout meldt is onnodig vervelend.

**Het trimmen zit niet in het formulier.** `Form::to_tags` maakt van lege invoer `None` en laat de rest aan `Tags::normalized` over. Die regel hoort op één plek te staan, en dat is het tagmodel — anders krijg je twee plekken die het net iets anders doen.

**De ingang naar de geavanceerde weergave is verhuisd.** Die zat sinds task-11 per regel in de maplijst, als tijdelijke oplossing. FR-7 beschrijft hem als onderdeel van de bestandspagina, en daar staat hij nu. De maplijstregel linkt naar het bewerkformulier; dat scheelt ook een tweede link per regel op een telefoon.

**`atomic` is zijn `#![allow(dead_code)]` kwijt.** Er is nu een handler die schrijft, dus de hele keten van route tot `atomic::replace` wordt bereikt.

**Handmatig in de browser en met curl gecontroleerd**: het formulier toont de waarden uit het bestand, opslaan levert een bevestiging met de teruggelezen waarden, een leeggemaakte componist verdwijnt ook volgens `ffprobe`, de embedded hoes blijft heel (1288 bytes vóór en na), en ongeldige invoer meldt beide fouten tegelijk terwijl het bestand ongewijzigd blijft. De browser-extensie viel halverwege weg; de rest is met curl afgemaakt.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Bewerkformulier per bestand met opslaan en teruglezen (FR-5, FR-6)

De kern van de app voor de gebruiker. De titel in de maplijst opent `/bewerk/<pad>`: een formulier met de twaalf kernvelden, gevuld met wat er nú in het bestand staat.

### Wat er is toegevoegd

- **`src/edit.rs` (nieuw)** — de vertaling tussen het tagmodel en de tekst in een formulier. Dat zijn twee verschillende werelden: een invoerveld kent geen `None`, alleen lege tekst, en een tracknummer is daar tekst en geen getal. `Form::to_tags` valideert de vier numerieke velden en levert leesbare meldingen op.
- **`GET`/`POST /bewerk/{*path}`** — het formulier en het opslaan. Na een geslaagde schrijfactie wordt het bestand **opnieuw ingelezen** en worden díe waarden getoond: de bevestiging komt uit het bestand en niet uit wat de gebruiker intikte, want alleen dan zegt hij iets.
- **De pagina** — hoes, formaat, duur, een link naar de ruwe tags, en boven het formulier één zin over wat een leeg veld doet. Eén kolom, ruime velden, `inputmode="numeric"` op de nummers; alleen nummer en totaal delen een regel, en ook die vallen onder elkaar zodra het krap wordt.

### Beslissingen

- **Validatie vóór het bestand.** Een typefout in een tracknummer hoort geen schrijfactie te starten die halverwege afketst. Alle fouten komen tegelijk terug, niet één per keer.
- **Geen redirect na het opslaan.** Het gebruikelijke POST/Redirect/GET is hier niet nodig: `tags::write` raakt het bestand niet aan wanneer er niets verandert, dus een herlaadactie doet niets. Dat scheelt een flash-mechanisme; een test legt vast dat het klopt.
- **Het trimmen blijft in `Tags::normalized`.** Het formulier maakt van lege invoer `None` en laat de rest aan het tagmodel over — die regel hoort op één plek te staan.
- **De ingang naar de ruwe tags is verhuisd** van de maplijst naar de bestandspagina, waar FR-7 hem beschrijft.

### Tests

199 tests groen (156 unit, 1 architectuur, 5 art, 12 mapbrowser, 8 bewerken, 6 configuratie, 7 ruwe tags, 4 server); `cargo fmt --check` en `cargo clippy -- -D warnings` schoon.

- Unit in `edit`: heen en weer tussen model en formulier, leeg wordt `None`, spaties tellen als leeg, elk numeriek veld meldt zijn eigen naam bij onzin, en meerdere fouten komen samen terug.
- Unit in `web`: het formulier toont de huidige waarden en de uitleg; een POST schrijft en toont de teruggelezen waarden; ongeldige invoer schrijft niets en houdt de invoer vast; verkeerde paden geven 403, 404 en 415.
- Integratie `tests/edit.rs` (AC #7): via de echte binary een fixture-kopie bewerken — voor MP3 én FLAC, met accenten in de titel — en `ffprobe` laten bevestigen wat er in het bestand staat. Plus: een leeggemaakt veld verdwijnt, ongeldige invoer verandert niets, twee keer versturen raakt het bestand de tweede keer niet aan, en met `BACKUP_ON_WRITE=true` verschijnt er een `.bak`.

### De poortrace, voor de derde en laatste keer

Met een vijfde integratiebinary viel ongeveer één op de drie volledige runs om. Ik heb eerst de foutmelding diagnostisch gemaakt in plaats van te gokken, en die wees het aan: de server logde dat hij op `0.0.0.0:57389` luisterde, en de test kreeg tóch `ConnectionReset` na nul bytes.

De oorzaak is BSD-gedrag: op macOS mag `127.0.0.1:P` naast een bestaande `0.0.0.0:P` bestaan wanneer `SO_REUSEADDR` aanstaat — en dat zet Rust standaard aan. `free_port()` bond op de loopback en gaf dus poorten uit die een draaiende testserver al gebruikte; de verbinding kwam vervolgens uit bij de net gesloten proef-socket. `free_port()` bindt nu op `0.0.0.0`, hetzelfde adres als de server, waarna de kernel zo'n poort simpelweg niet meer uitdeelt. Vijftien opeenvolgende volledige runs daarna zonder uitval.

De eerdere pogingen (task-9, task-12) waren niet fout maar dekten andere gaten. Dit was het derde.
<!-- SECTION:FINAL_SUMMARY:END -->
