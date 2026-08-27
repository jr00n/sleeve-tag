---
id: TASK-9
title: Thumbnails van embedded album art in de maplijst
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:23'
updated_date: '2026-08-27 21:02'
labels: []
milestone: m-1
dependencies:
  - TASK-7
  - TASK-8
documentation:
  - PRD.md
modified_files:
  - src/main.rs
  - src/art.rs
  - src/browse.rs
  - src/web/mod.rs
  - templates/listing.html
  - static/app.css
  - tests/art.rs
  - tests/common/mod.rs
  - README.md
  - CLAUDE.md
priority: medium
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In de maplijst hoort per bestand een kleine weergave van de embedded hoes, zodat direct zichtbaar is welke tracks art missen of afwijkende art hebben. De afbeeldingen komen uit de bestanden zelf; er is geen cache-laag in het MVP.

Aandachtspunt is de prestatie-eis van FR-2 in combinatie met §8.5: een map met 30 tracks moet binnen een seconde laden, dus thumbnails mogen het renderen van de pagina niet blokkeren.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Er is een endpoint dat de embedded front cover van een bestand als afbeelding serveert, met correcte content-type header
- [x] #2 In de maplijst wordt per bestand een thumbnail getoond, en een duidelijke placeholder wanneer er geen art is
- [x] #3 Het laden van thumbnails blokkeert het renderen van de maplijst niet
- [x] #4 Een verzoek om art van een bestand zonder art geeft een nette 404 in plaats van een fout
- [x] #5 Een integratietest controleert het endpoint voor een fixture met en zonder embedded art
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

### 1. `src/art.rs` (nieuw) — afbeeldingsbewerking
De plek waar de `image`-crate thuishoort, los van `tags::` (dat alleen de bytes uit het bestand haalt).

- `thumbnail(data: &[u8], max: u32) -> Result<Vec<u8>, ArtError>` — decodeert, schaalt met behoud van de beeldverhouding naar hoogstens `max` px per as, en encodeert als JPEG.
- JPEG omdat een thumbnail geen transparantie nodig heeft en het formaat de kleinste bytes oplevert; het origineel blijft ongemoeid in het bestand.
- Task-20 (verkleinen en hercoderen bij het schrijven) kan hier straks op verder.

### 2. `src/web/mod.rs` — het endpoint
- `GET /art/{*path}` geeft de embedded front cover **ongewijzigd** terug, met het MIME-type zoals het in het bestand staat. Dat is wat AC #1 vraagt en wat de detailweergave (task-19) straks nodig heeft.
- `GET /art/{*path}?size=thumb` geeft een verkleinde JPEG (max 160 px, genoeg voor het 40px-vakje op een scherm met hoge pixeldichtheid). De maplijst gebruikt deze variant: dertig hoezen van elk een halve megabyte over Tailscale naar een telefoon sturen is geen thumbnail.
- Pad via `fs::Library::resolve`, daarna `tags::read_front_cover`. Bewust niet via `resolve_editable_file`: dat opent het bestand een extra keer, en bij dertig thumbnails per pagina telt dat op.
- Geen art in het bestand → 404 met een leesbare melding (AC #4). Een pad buiten de bibliotheek → 403, onbestaand → 404, geen audio → 415; die vertaling staat al in `WebError`.
- Lezen én verkleinen is blokkerend werk en gaat in `spawn_blocking`.
- `Cache-Control: no-cache`: er is bewust geen cache-laag in het MVP, en na een latere schrijfactie mag de browser geen oude hoes tonen.

### 3. `src/browse.rs` — de URL in het weergavemodel
`TrackSummary` krijgt `art_url: String` (de `?size=thumb`-variant), naast het bestaande `has_art`. De template kiest tussen afbeelding en placeholder; de URL wordt net als de andere in Rust opgebouwd en gecodeerd.

### 4. `templates/listing.html` + `static/app.css`
- `<img>` met `loading="lazy"`, `decoding="async"` en vaste `width`/`height`, zodat de lijst gerenderd is voordat er ook maar één hoes binnen is (AC #3) en er geen layout-verschuiving optreedt.
- De bestaande gestreepte placeholder blijft voor bestanden zonder hoes, met een `title` zodat de betekenis ook bij twijfel duidelijk is.

### 5. Tests
- Unit in `art`: verkleint een grote afbeelding, laat een kleine met rust, JPEG eruit ongeacht PNG erin, en een foutmelding op onzin-bytes.
- Unit in `web`: 200 + `image/jpeg` voor een fixture mét hoes, 404 voor een fixture zonder, 415 voor een niet-audiobestand, 403 voor een pad buiten de bibliotheek.
- Unit in `browse`: `art_url` wijst naar de thumbnail-variant van het juiste pad.
- Integratie `tests/art.rs`: het endpoint over HTTP tegen de echte binary, met en zonder embedded art, en de thumbnail kleiner dan het origineel (AC #5).

### 6. Documentatie
README: `art` in de moduletabel, en in de sectie 'Mapbrowser' beschrijven hoe de hoezen geladen worden. CLAUDE.md: de regel dat afbeeldingsbewerking via `art::` loopt, naast de bestaande regels voor `tags::` en `fs::`.

### Afwijkingen van het plan

- `art::thumbnail` schaalt alleen omlaag; het plan ging ervan uit dat de crate dat zelf al deed.
- Naast `thumbnail` heeft `art::` ook `dimensions` gekregen — nodig om in de tests te controleren wat er teruggegeven wordt, en straks voor de detailweergave (task-19).
- De constante `THUMBNAIL_SIZE_PARAM` staat in `browse::` (waar de URL's worden opgebouwd) en wordt door `web::` geïmporteerd, zodat de URL en het endpoint niet uit elkaar kunnen lopen.
- Extra, niet gepland: de poortrace in `tests/common/mod.rs` moest gedicht worden. Met een tweede integratiebinary viel de suite er af en toe op om.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**`DynamicImage::thumbnail` vergroot ook.** Mijn aanname dat de functie alleen omlaag schaalt bleek fout; een hoes van 100 px werd opgeblazen naar 160 px. Dat kost bytes en levert geen scherpte op, dus `art::thumbnail` schaalt nu alleen wanneer de afbeelding groér is dan de grens. Gevonden door de test `does_not_enlarge_a_smaller_image`, die eerst faalde.

**Een afgekapte JPEG decodeert gewoon.** De test die daar een fout van verwachtte, was fout: de JPEG-decoder vult ontbrekende data aan en levert alsnog pixels, de PNG-decoder weigert. De test controleert nu wat er werkelijk toe doet — geen paniek, en wat eruit komt is een leesbare afbeelding.

**Bytes zijn geen bruikbare maatstaf met deze fixtures.** De ingecheckte hoes is een egaal blauw vlak van 300×300 dat in 1288 bytes past; hercoderen naar 160×160 levert 1328 bytes op, dus gróter. De garantie op HTTP-niveau is daarom de afmeting. Dat verkleinen wél bytes scheelt, wordt in `art::tests` getest met een gegenereerde afbeelding van 1200×1200 met detail in elke pixel: die krimpt meer dan tienvoudig.

**Poortrace in de testharnas gedicht.** Met een tweede integratiebinary (`tests/art.rs`) naast `tests/browse.rs` werd de als 'theoretisch' beschreven race tussen `free_port()` en het binden door de server echt: één op de twaalf volledige testruns viel om op een leesfout. `Server::start_in` probeert nu tot vijf poorten, en `wait_until_listening` controleert vóór elke verbindingspoging of het eigen proces nog leeft — anders zou een test tegen de server van een ándere test kunnen praten, met een vreemde bibliotheek. Twaalf volledige runs daarna zonder uitval.

**Geen dubbele bestandsopening per thumbnail.** Het endpoint gaat via `Library::resolve` plus `tags::read_front_cover`, niet via `resolve_editable_file`: dat laatste opent het bestand een extra keer alleen om het formaat vast te stellen. Bij dertig thumbnails per pagina is dat dertig overbodige opens. Dat het geen audio is, blijkt vanzelf uit het lezen en wordt een 415.

**Een hoes die niet te verkleinen is, wordt onverkleind geserveerd** in plaats van een fout te worden. Een gebroken plaatje in de lijst zegt de gebruiker niets; het origineel laat zien wat er in het bestand zit. De mislukking komt wel in het log.

**AC #3 objectief gemeten** in Chrome via de Resource Timing API: het document was na 18 ms compleet, de twee hoesverzoeken begonnen pas na 3578 ms, en er werden precies twee verzoeken gedaan — alleen voor de bestanden die ook echt een hoes hebben.

**Geheugengrens bij het decoderen** (`Limits::max_alloc`, 256 MB). Een afbeelding van 20000×20000 past in een paar honderd kilobyte gecomprimeerde data maar vraagt bij het uitpakken meer dan een gigabyte; op de NAS is dat het verschil tussen een foutmelding en een gestorven container.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Thumbnails van embedded album art in de maplijst

De maplijst toont nu per bestand de embedded hoes, en een duidelijke placeholder waar die ontbreekt.

### Wat er is toegevoegd

- **`src/art.rs` (nieuw)** — de enige module die pixels aanraakt. `thumbnail()` decodeert, schaalt omlaag met behoud van de beeldverhouding en encodeert als JPEG; `dimensions()` leest afmetingen uit de header zonder te decoderen. Een geheugengrens van 256 MB voorkomt dat een opgeblazen afbeelding de container omlegt. Task-20 (verkleinen bij het schrijven) kan hierop verder.
- **`GET /art/{*path}`** — de hoes ongewijzigd, met het MIME-type uit het bestand. Met `?size=thumb` een JPEG van hoogstens 160 px. Beide met `Cache-Control: no-cache`, want er is bewust geen cache-laag en na een latere schrijfactie mag de browser geen oude hoes tonen.
- **`browse::TrackSummary.art_url`** — de thumbnail-URL, net als de andere URL's in Rust opgebouwd en gecodeerd.
- **Template en stijl** — `<img loading="lazy" decoding="async">` met vaste afmetingen; bestanden zonder hoes krijgen de gestreepte placeholder met een `aria-label` en doen geen verzoek dat toch een 404 zou opleveren.

### Beslissingen

- **Verkleinen bij het verzoek.** Een vakje van 40 px vullen met een hoes van een halve megabyte, dertig keer per pagina, maakt de app onbruikbaar op een telefoon. De CPU-kosten van het schalen wegen daar ruim tegenop; zonder cache-laag is dat de prijs van het MVP.
- **Eén bestandsopening per hoes.** Het endpoint gaat via `Library::resolve` en niet via `resolve_editable_file`, dat het bestand alleen voor de formaatcontrole een tweede keer zou openen.
- **Een onverkleinbare hoes wordt onverkleind geserveerd**, met een waarschuwing in het log. Een gebroken plaatje zegt de gebruiker niets.

### Tests

114 tests groen (89 unit, 1 architectuur, 5 art-integratie, 9 mapbrowser-integratie, 6 configuratie, 4 server); `cargo fmt --check` en `cargo clippy -- -D warnings` schoon.

- Unit in `art`: schalen omlaag, niet omhoog, beeldverhouding, altijd JPEG (ook uit PNG), de bytewinst gemeten met een gegenereerde afbeelding van 1200×1200 met detail in elke pixel, en onzin- en afgekapte invoer zonder paniek.
- Unit in `web`: content-type en afmetingen met en zonder `?size=thumb`, `Cache-Control`, 404 zonder hoes, 415 op niet-audio, 403 op traversal, en dat de lijst de thumbnail-variant opvraagt.
- Integratie `tests/art.rs`: het endpoint over HTTP tegen de echte binary, voor MP3 en FLAC, met en zonder embedded art. De afmetingen worden uit de JPEG/PNG-header gelezen, omdat een integratietest `art::` niet kan aanroepen.

In Chrome gemeten dat de hoezen het renderen niet ophouden: het document was na 18 ms compleet, de hoesverzoeken begonnen pas na 3578 ms, en er gingen precies twee verzoeken uit — alleen voor de bestanden die een hoes hebben.

### Terzijde meegenomen

`tests/common/mod.rs`: de poortrace tussen `free_port()` en het binden door de server werd echt zodra er twee integratiebinaries naast elkaar draaien (één uitval in twaalf volledige runs). `Server::start_in` probeert nu tot vijf poorten en controleert vóór elke verbindingspoging of het eigen serverproces nog leeft, zodat een test nooit tegen de server van een andere test praat.

### Vervolg

De hoes wordt bij elk verzoek opnieuw uitgelezen en verkleind. Zolang de bibliotheek op de NAS staat en de app één gebruiker heeft is dat prima, maar een `ETag` op basis van pad, grootte en wijzigingstijd zou herbezoeken gratis maken. Dat is een bewuste openstaande keuze, geen omissie.
<!-- SECTION:FINAL_SUMMARY:END -->
