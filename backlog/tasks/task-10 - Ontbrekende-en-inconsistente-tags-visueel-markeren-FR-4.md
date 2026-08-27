---
id: TASK-10
title: Ontbrekende en inconsistente tags visueel markeren (FR-4)
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:23'
updated_date: '2026-08-27 21:11'
labels: []
milestone: m-1
dependencies:
  - TASK-8
documentation:
  - PRD.md
modified_files:
  - src/main.rs
  - src/checks.rs
  - src/browse.rs
  - src/web/mod.rs
  - templates/listing.html
  - static/app.css
  - tests/browse.rs
  - README.md
  - CLAUDE.md
priority: medium
type: feature
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het doel van de app is snel kunnen corrigeren. Daarvoor moet de maplijst zelf aanwijzen waar iets mis is, zonder dat de gebruiker elk bestand hoeft te openen.

Te signaleren gevallen: ontbrekende kernvelden (titel, artiest, album), ontbrekende album art, en waarden die binnen dezelfde map onderling afwijken terwijl ze gelijk horen te zijn (bijvoorbeeld meerdere albumtitels of albumartiesten in één map), plus ontbrekende of dubbele tracknummers.

De markering is puur informatief; de app past nooit ongevraagd iets aan.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bestanden met ontbrekende kernvelden of ontbrekende album art zijn in de maplijst visueel gemarkeerd
- [x] #2 Waarden die binnen een map onderling afwijken (album, albumartiest, jaar) worden als inconsistentie gemarkeerd op mapniveau
- [x] #3 Ontbrekende en dubbele tracknummers binnen een map worden gesignaleerd
- [x] #4 Bij elke markering is zichtbaar wat er precies aan de hand is (bijv. via tooltip of tekstlabel)
- [x] #5 De markering wijzigt niets aan de bestanden
- [x] #6 Unit-tests dekken de detectielogica met testmappen die consistent, deels inconsistent en volledig leeg getagd zijn
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

### 1. `src/checks.rs` (nieuw) — de signalering
Pure functie over het genormaliseerde tagmodel; leest geen bestanden en raakt niets aan (AC #5 volgt daar automatisch uit).

```rust
pub fn review(entries: &[Entry<'_>]) -> Review
```
- `Entry { tags: &Tags, has_art: bool }` — de feiten per bestand.
- `Review { tracks: Vec<Vec<TrackIssue>>, folder: Vec<FolderIssue> }`, waarbij `tracks` dezelfde volgorde houdt als de invoer.
- `TrackIssue`: geen titel, geen artiest, geen album, geen hoes, geen tracknummer, dubbel tracknummer. Elk met een korte Nederlandse tekst voor in de lijst.
- `FolderIssue`: afwijkende waarden voor album, albumartiest of jaar; bestanden zonder tracknummer; tracknummers die meer dan eens voorkomen. Elk met een volzin die zegt wat er aan de hand is (AC #4).
- Alleen ingevulde waarden tellen mee bij het vergelijken: een ontbrekend album is een ontbrekend veld, geen inconsistentie. Bij veel verschillende waarden worden er drie genoemd en de rest geteld, zodat één regel niet ontspoort.

### 2. `src/browse.rs` — koppelen aan het weergavemodel
- `TrackSummary` krijgt het volledige `tags: Tags` in plaats van de losse `title`/`artist`/`album`/`track`-velden. Dat is nodig omdat de signalering ook albumartiest en jaar bekijkt, en het maakt het model eerlijker: de rij draagt de gegevens die hij toont. De `*_label()`-methoden en de templates veranderen niet.
- `TrackSummary` krijgt `issues: Vec<TrackIssue>`, `Listing` krijgt `folder_issues: Vec<FolderIssue>`.
- De signalering draait over **alle** bestanden in de map, vóór het filteren. Anders zou "twee verschillende albumtitels" verdwijnen zodra je zoekt, terwijl er niets aan de map veranderd is.

### 3. Templates en stijl
- Boven de lijst een blok "Let op in deze map" met de bevindingen op mapniveau, alleen wanneer er iets te melden is.
- Per rij korte, zichtbare labels ("geen hoes", "dubbel tracknummer") in plaats van alleen een tooltip: op een telefoon is er geen hover, dus een tooltip alleen zou AC #4 niet halen.
- Een eigen waarschuwingskleur in `app.css`, voor licht en donker.

### 4. Tests
- Unit in `checks`: een volledig consistente map (geen enkele melding), een map met deels afwijkende album/albumartiest/jaar, een map met ontbrekende en dubbele tracknummers, een map met volledig ongetagde bestanden, en de randgevallen: één bestand, lege map, en het onderscheid tussen "ontbreekt" en "wijkt af".
- Unit in `browse`: de meldingen komen op de juiste rij terecht en de mapmeldingen blijven staan wanneer er gefilterd wordt.
- Unit in `web`: de mappagina toont de meldingen.
- Integratie: uitbreiding van `tests/browse.rs` met een map die opzettelijk rommelig is.

### 5. Documentatie
README: `checks` in de moduletabel en een korte uitleg bij de sectie 'Mapbrowser' over wat er gesignaleerd wordt en dat het puur informatief is. CLAUDE.md: de regel dat de signalering nooit iets wijzigt.

### Afwijkingen van het plan

- Geen. De enige inperking is de dekking van AC #2 over HTTP: twee verschillende albumtitels in één map vereist het schrijven van tags (task-13), dus die detectie wordt op unit-niveau bewezen en de weergave via een ander mapniveau-signaal.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Ontbreken en afwijken zijn verschillende dingen.** Een bestand zonder album naast bestanden mét hetzelfde album is een ontbrekend veld op dat ene bestand, geen tegenstrijdigheid in de map. Zonder dat onderscheid zou elke map met één ongetagd bestand als inconsistent gelden en zou de melding niets meer betekenen. Vastgelegd in `distinct_values` (alleen ingevulde waarden tellen mee) en in de test `a_missing_value_is_not_an_inconsistency`.

**De beoordeling loopt over de hele map, vóór het filteren.** Anders zou 'twee verschillende albumtitels' verdwijnen zodra de gebruiker zoekt, terwijl er aan de map niets veranderd is. In `browse::listing` worden daarom eerst alle bestanden ingelezen en beoordeeld, en pas daarna gefilterd en gesorteerd. Getest met `folder_issues_survive_a_filter`.

**`TrackSummary` draagt nu het volledige `Tags` in plaats van vier losse velden.** De signalering kijkt ook naar albumartiest en jaar, en het bewerkformulier (task-14) heeft straks alles nodig. De `*_label()`-methoden en de templates bleven ongewijzigd; alleen de tests die rechtstreeks `track.title` lazen, zijn `track.tags.title` geworden.

**Zichtbare labels, geen tooltip.** AC #4 noemt 'tooltip of tekstlabel', maar op een telefoon is er geen hover; een tooltip alleen zou daar niets tonen. Het zijn daarom korte chips onder de bestandsnaam, die bij een smalle kolom netjes doorlopen naar de volgende regel (gecontroleerd bij een inhoudsbreedte van 360 px).

**AC #2 is over HTTP niet volledig te testen met de huidige fixtures.** Twee verschillende albumtitels in één map vereist dat er tags geschreven worden, en dat kan pas vanaf task-13; alle ingecheckte fixtures dragen dezelfde albumtitel. De detectie is daarom op unit-niveau gedekt (`different_albums_are_reported_at_folder_level`, `album_artist_and_year_are_checked_too`) — precies wat AC #6 vraagt — en de weergave van mapmeldingen wordt over HTTP bewezen via het dubbele tracknummer, dat door dezelfde template loopt.

**AC #5 is met bytes bewezen**, niet met een redenering: `marking_leaves_the_files_untouched` maakt een vingerafdruk van de volledige inhoud van elk bestand in de map, vraagt de pagina twee keer op (met en zonder filter) en vergelijkt opnieuw.

**Toon van de meldingen.** Bewust 'Let op in deze map' en niet 'Fouten': een map met meerdere albumtitels kan een verzamelmap zijn en hoeft niet fout te zijn. De PRD zegt dat de bibliotheek typisch `Artiest/Album` is maar niet gegarandeerd consistent; de app constateert, de gebruiker beslist.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Ontbrekende en inconsistente tags visueel markeren (FR-4)

De maplijst wijst nu zelf aan waar iets mis is, zodat je niet elk bestand hoeft te openen om dat te ontdekken.

### Wat er is toegevoegd

- **`src/checks.rs` (nieuw)** — de signalering, als pure functie over het genormaliseerde tagmodel. Leest geen bestanden en schrijft niets, waarmee AC #5 een eigenschap van het ontwerp is en geen belofte.
  - `TrackIssue` per bestand: geen titel, artiest, album, hoes, tracknummer, of een dubbel tracknummer.
  - `FolderIssue` per map: afwijkende albumtitels, albumartiesten of jaartallen; hoeveel bestanden geen tracknummer hebben; welke nummers meer dan eens voorkomen. Elk met een volzin die zegt wat er aan de hand is.
- **`browse::`** — `TrackSummary` draagt nu het volledige `Tags` (de signalering kijkt ook naar albumartiest en jaar) plus `issues`; `Listing` krijgt `folder_issues`.
- **Weergave** — een blok "Let op in deze map" boven de lijst, en korte zichtbare labels onder elke bestandsnaam. Een eigen waarschuwingskleur voor licht en donker.

### Beslissingen

- **Ontbreken ≠ afwijken.** Alleen ingevulde waarden tellen mee bij het vergelijken. Zonder dat onderscheid zou elke map met één ongetagd bestand als inconsistent gelden.
- **Beoordelen vóór filteren.** Aan een map verandert niets doordat de gebruiker zoekt, dus de mapmeldingen blijven staan.
- **Zichtbare tekst, geen tooltip.** Op een telefoon is er geen hover; een tooltip alleen zou AC #4 daar niet halen.
- **Constaterende toon.** "Let op in deze map", niet "Fouten": een map met meerdere albumtitels kan een verzamelmap zijn. De app constateert, de gebruiker beslist.

### Tests

134 tests groen (106 unit, 1 architectuur, 5 art-integratie, 12 mapbrowser-integratie, 6 configuratie, 4 server); `cargo fmt --check` en `cargo clippy -- -D warnings` schoon.

- Unit in `checks` (AC #6): een volledig consistente map zonder enkele melding, een volledig ongetagde map, afwijkende album/albumartiest/jaar, ontbrekende en dubbele tracknummers, het onderscheid tussen ontbreken en afwijken, de randgevallen één bestand en lege map, en de bewoording van elke melding inclusief het inkorten van lange opsommingen.
- Unit in `browse`: meldingen landen op de juiste rij, mapmeldingen overleven een filter, een nette map meldt niets.
- Unit in `web` en integratie in `tests/browse.rs`: de meldingen belanden ook werkelijk op de pagina, en een nette map krijgt geen waarschuwing.
- `marking_leaves_the_files_untouched` vergelijkt de volledige inhoud van elk bestand vóór en na het opvragen van de pagina — AC #5 met bytes bewezen.

In Chrome gecontroleerd bij een inhoudsbreedte van 360 px: de labels lopen netjes door naar de volgende regel en de pagina scrollt niet horizontaal.

### Beperking

Twee verschillende albumtitels in één map is met de ingecheckte fixtures niet te maken — daarvoor moeten er tags geschreven worden, wat pas vanaf task-13 kan. Die detectie is op unit-niveau gedekt; de weergave van mapmeldingen wordt over HTTP bewezen via het dubbele tracknummer, dat door dezelfde template loopt.
<!-- SECTION:FINAL_SUMMARY:END -->
