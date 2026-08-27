---
id: TASK-11
title: 'Geavanceerde weergave met alle ruwe tags, alleen-lezen (FR-7)'
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:23'
updated_date: '2026-08-27 21:22'
labels: []
milestone: m-1
dependencies:
  - TASK-7
  - TASK-8
documentation:
  - PRD.md
modified_files:
  - src/tags/mod.rs
  - src/browse.rs
  - src/web/mod.rs
  - templates/rawtags.html
  - templates/listing.html
  - static/app.css
  - tests/rawtags.rs
  - README.md
priority: low
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Naast het genormaliseerde model wil de beheerder kunnen zien wat er werkelijk in een bestand staat: alle aanwezige ID3-frames of Vorbis-comments, inclusief velden die de app niet modelleert. Dit is diagnostisch en helpt te begrijpen waarom een mediaserver iets anders toont dan verwacht.

In het MVP is deze weergave uitdrukkelijk alleen-lezen; bewerken van ruwe frames is geen doel.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Per bestand is een 'geavanceerd'-weergave op te vragen die alle aanwezige ruwe tags als sleutel-waardelijst toont
- [x] #2 Voor MP3 worden ID3-frames getoond, voor FLAC Vorbis-comments, telkens met de originele sleutelnaam
- [x] #3 Binaire velden zoals embedded art worden samengevat (type en grootte) in plaats van als ruwe data getoond
- [x] #4 De weergave biedt geen enkele manier om ruwe tags te wijzigen
- [x] #5 Een integratietest controleert de weergave voor een MP3- en een FLAC-fixture met volledige tags
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

Het zware werk staat er al: `tags::read_raw_tags` uit task-7 levert de ruwe sleutel-waardeparen met hun oorspronkelijke naam en vat binaire waarden samen. Wat ontbreekt is een pagina, een route en de weg ernaartoe.

### 1. `src/tags/mod.rs` — het soort tag erbij
`read_raw_tags` geeft nu een kale `Vec<RawTag>`. Dat wordt:

```rust
pub struct RawTags {
    pub format: Format,
    /// Het soort tag zoals het in het bestand staat; `None` als er geen tag is.
    pub kind: Option<String>,
    pub items: Vec<RawTag>,
}
```

Zonder die informatie kan de pagina niet zeggen dat ze ID3v2-frames toont en geen Vorbis-comments — precies het onderscheid dat AC #2 vraagt. Formaat en tagsoort komen uit dezelfde `open()`, dus dit kost geen extra bestandsopening. De twee bestaande tests in `tags::` gaan mee.

### 2. `src/web/mod.rs` — de route
- `GET /tags/{*path}` rendert `templates/rawtags.html`.
- Pad via `Library::resolve`, daarna `tags::read_raw_tags`; een niet-audiobestand levert 415 op, een onbestaand pad 404, een pad buiten de bibliotheek 403. Dat loopt door de bestaande `WebError`-vertaling.
- Lezen is blokkerend en gaat in `spawn_blocking`.

### 3. `src/browse.rs` — de weg ernaartoe
- `TrackSummary` krijgt `raw_url`, net als `art_url` in Rust opgebouwd en gecodeerd.
- Een publieke helper voor de broodkruimels van de map waarin een bestand staat, zodat de detailpagina terug kan navigeren.

### 4. Templates en stijl
- `templates/rawtags.html`: broodkruimelpad naar de map, de bestandsnaam als kop, een regel met formaat en tagsoort, en een tabel met sleutel en waarde. Sleutels in een monospace-lettertype; de tabel krijgt een eigen scrollcontainer zodat de pagina zelf niet horizontaal scrollt.
- Nadrukkelijk alleen-lezen (AC #4): geen formulier, geen invoerveld, geen knop. De tekst zegt dat er ook bij.
- In de maplijst een discrete link "ruwe tags" per regel. Zodra het bewerkformulier er is (task-14) verhuist die ingang daarheen; tot dan is dit de enige weg erheen.

### 5. Tests
- Unit in `tags`: de tagsoort per fixture — ID3v2 voor de getagde MP3, Vorbis-comments voor de FLAC, ID3v1 voor de fixture zonder ID3v2, en `None` voor een bestand zonder tags.
- Unit in `web`: de pagina rendert voor MP3 en FLAC, toont `TIT2` respectievelijk `TITLE`, vat de hoes samen, en bevat geen enkel bewerkbaar element (AC #4, als assertie op `<form`, `<input`, `<textarea` en `<button`).
- Integratie `tests/tags.rs` (AC #5): over HTTP tegen de echte binary, voor een MP3- en een FLAC-fixture met volledige tags, plus het geval zonder tags en de geweigerde paden.

### 6. Documentatie
README: de route in de sectie over de mapbrowser, met de nadruk op alleen-lezen. CLAUDE.md hoeft niets nieuws: de regel dat alle tag-I/O via `tags::` loopt dekt dit al.

### Afwijkingen van het plan

- Extra op de pagina: een regel die uitlegt dat een samengesteld frame als `TRCK` als twee regels verschijnt. Niet voorzien, maar zonder die uitleg is de weergave op dat punt misleidend.
- Extra in de README: de fixture-tabel beweerde dat `untagged.flac` geen enkele tag heeft. Dat bleek onjuist (`ENCODER=ffmpeg`) en is gecorrigeerd.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Een samengesteld ID3v2-frame verschijnt als twee regels.** In het bestand staat één frame `TRCK = "3/12"`; lofty leest dat als nummer én totaal, en beide delen worden op de weergave weer `TRCK`. Hetzelfde bij `TPOS`. Dat is misleidend zonder uitleg — het lijkt alsof het bestand twee TRCK-frames heeft. Echte frame-getrouwheid zou de ID3v2-specifieke API van lofty vereisen en daarmee een tweede, formaatafhankelijk leespad; voor een diagnostische pagina weegt dat niet op. De pagina zegt daarom zelf wat er gebeurt, en de README ook.

**De fixture `untagged.flac` bevat wél een tag: `ENCODER=ffmpeg`.** Dat schrijft ffmpeg ook met `-map_metadata -1`. Mijn eerste test verwachtte een leeg tagblok en faalde terecht. Het is bovendien een mooie illustratie van waar FR-7 voor bestaat: het genormaliseerde model toont niets (ENCODER is niet gemodelleerd), de ruwe weergave toont het veld. De test controleert nu precies dat verschil, en de fixture-tabel in de README is gecorrigeerd — die beweerde 'geen enkele tag'.

**Een MP3 zonder tags en een FLAC zonder tags zijn verschillende gevallen.** De MP3 heeft geen tagblok, de FLAC heeft er één dat leeg is (of alleen ENCODER bevat). De pagina maakt dat onderscheid ook: 'Dit bestand bevat geen tagblok' tegenover 'Het tagblok van dit bestand is leeg'. Voor een diagnostische weergave zijn dat twee verschillende diagnoses.

**`read_raw_tags` geeft nu `RawTags` in plaats van `Vec<RawTag>`**, met formaat en tagsoort erbij. Zonder de tagsoort kan de pagina niet zeggen dat ze ID3v2-frames toont en geen Vorbis-comments, en dat onderscheid is juist wat AC #2 vraagt. Beide komen uit dezelfde `open()`, dus het kost geen extra bestandsopening.

**AC #4 is als assertie vastgelegd, niet als belofte.** Zowel de unit- als de integratietest controleren dat de pagina geen `<form`, `<input`, `<textarea`, `<button` of `<select` bevat — voor alle vier de fixturevarianten. In Chrome bevestigd: `document.querySelectorAll('form, input, button, textarea, select').length` is 0.

**Ingang vanuit de maplijst.** Elke regel krijgt een discrete link 'ruwe tags'. Zodra het bewerkformulier er is (task-14) hoort die ingang daarheen te verhuizen — FR-7 beschrijft de geavanceerde weergave als onderdeel van de bestandspagina, niet van de lijst. Tot dan is dit de enige weg erheen.

**Brede waarden scrollen binnen de tabel**, niet met de pagina mee: `.tabelrand` heeft `overflow-x: auto`. In Chrome gecontroleerd bij een inhoudsbreedte van 360 px — de pagina scrollt niet horizontaal.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Geavanceerde weergave met alle ruwe tags, alleen-lezen (FR-7)

`/tags/<pad>` toont per bestand alles wat er werkelijk in staat, inclusief velden die het genormaliseerde model niet kent. Diagnostisch bedoeld: het verklaart waarom een mediaserver iets anders toont dan verwacht.

### Wat er is toegevoegd

- **`tags::read_raw_tags` geeft nu `RawTags`** — formaat, tagsoort (`ID3v2`, `Vorbis-comments`, `ID3v1`) en de sleutel-waardeparen. Zonder de tagsoort kan de pagina het onderscheid uit AC #2 niet maken; beide komen uit dezelfde bestandsopening.
- **`GET /tags/{*path}`** — rendert `templates/rawtags.html`. Pad via `Library::resolve`, lezen in `spawn_blocking`; niet-audio geeft 415, onbestaand 404, buiten de bibliotheek 403.
- **De pagina** — broodkruimelpad terug naar de map, formaat en tagsoort, en een tabel met sleutel en waarde. Sleutels in monospace, brede waarden scrollen binnen de tabel zodat de pagina zelf niet horizontaal scrollt.
- **Een discrete link "ruwe tags"** per regel in de maplijst, als ingang.

### Twee dingen die eerlijkheid vroegen

- **`TRCK` verschijnt als twee regels.** In het bestand staat één frame `3/12`; dat wordt als nummer én totaal gelezen, en beide delen krijgen weer de sleutel `TRCK`. Zonder uitleg lijkt het alsof het bestand twee frames heeft. Frame-getrouwheid zou een tweede, formaatafhankelijk leespad vereisen — voor een diagnostische pagina weegt dat niet op, dus de pagina zegt zelf wat er gebeurt.
- **De fixture `untagged.flac` draagt wél een tag**: `ENCODER=ffmpeg`, die ffmpeg ook met `-map_metadata -1` schrijft. Mijn test verwachtte een leeg tagblok en faalde terecht. Het illustreert precies waar FR-7 voor bestaat: het model toont niets, de ruwe weergave toont het veld. De fixture-tabel in de README beweerde "geen enkele tag" en is gecorrigeerd.

Bijkomend: een MP3 zonder tagblok en een FLAC met een leeg tagblok zijn verschillende diagnoses, en de pagina zegt ze ook verschillend.

### Alleen-lezen als assertie

AC #4 is geen belofte maar een test: unit én integratie controleren voor alle vier de fixturevarianten dat de pagina geen `<form`, `<input`, `<textarea`, `<button` of `<select` bevat. In Chrome bevestigd — nul bedienbare elementen op de pagina.

### Tests

148 tests groen (113 unit, 1 architectuur, 5 art, 12 mapbrowser, 7 ruwe tags, 6 configuratie, 4 server); `cargo fmt --check` en `cargo clippy -- -D warnings` schoon.

- Unit in `tags`: de tagsoort per fixture (ID3v2, Vorbis-comments, ID3v1), en het verschil tussen "geen tagblok" en "een blok met alleen ENCODER erin".
- Unit in `web`: MP3 toont `TIT2`/`TPE1`, FLAC toont `TITLE`/`ARTIST` en géén ID3-frames, de hoes wordt samengevat, en de geweigerde paden.
- Integratie `tests/rawtags.rs` (AC #5): dezelfde controles over HTTP tegen de echte binary, plus de weg terug naar de map en het bestand zonder tagblok.

### Vervolg

De ingang zit nu in de maplijst. FR-7 beschrijft de geavanceerde weergave als onderdeel van de bestandspagina, dus zodra het bewerkformulier er is (task-14) hoort die link daarheen te verhuizen.
<!-- SECTION:FINAL_SUMMARY:END -->
