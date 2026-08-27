---
id: TASK-8
title: Mapbrowser met bestandslijst en zoeken binnen de map (FR-1 t/m FR-3)
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:23'
updated_date: '2026-08-27 20:46'
labels: []
milestone: m-1
dependencies:
  - TASK-3
  - TASK-6
  - TASK-7
documentation:
  - PRD.md
modified_files:
  - src/main.rs
  - src/browse.rs
  - src/fs.rs
  - src/web/mod.rs
  - templates/directory.html
  - templates/listing.html
  - templates/index.html
  - static/app.css
  - tests/browse.rs
  - tests/common/mod.rs
  - Cargo.toml
  - Cargo.lock
  - README.md
  - CLAUDE.md
priority: high
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De gebruiker moet vanaf tablet of telefoon door de muziekshare kunnen bladeren en per map zien welke tracks er staan met hun belangrijkste tags. Dit is de hoofdnavigatie van de app en het startpunt van elke bewerksessie.

Per map: submappen en de MP3/FLAC-bestanden met tracknummer, titel, artiest, album, duur en formaat, plus ruimte voor de art-thumbnail (aparte taak). Zoeken/filteren gebeurt binnen de huidige map op bestandsnaam of titel. De bibliotheek is typisch `Artiest/Album/track.ext` maar niet gegarandeerd consistent, dus de weergave mag niets over de mapstructuur aannemen.

Aanname voor de standaardsortering (open punt in PRD §12): sorteren op tracknummer uit de tags, met bestandsnaam als terugval wanneer een tracknummer ontbreekt.

Prestatie-eis: een map met 30 tracks laadt in minder dan een seconde op de NAS. Tags worden lazy en per map gelezen; er is bewust geen bibliotheek-index in het MVP.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Een mappagina toont submappen en bewerkbare audiobestanden van de opgevraagde map, startend bij MUSIC_ROOT
- [x] #2 Navigatie boven MUSIC_ROOT is niet mogelijk en er is een broodkruimelpad om terug te navigeren
- [x] #3 Per bestand worden tracknummer, titel, artiest, album, duur en formaat getoond
- [x] #4 Bestanden zijn standaard gesorteerd op tracknummer met bestandsnaam als terugval
- [x] #5 Zoeken/filteren binnen de huidige map werkt op bestandsnaam en op titel
- [x] #6 De lijst is bruikbaar op een telefoonscherm
- [x] #7 Een map met 30 tracks rendert in minder dan een seconde op de NAS
- [x] #8 Een integratietest laadt een testmap met fixtures en controleert de getoonde velden, de sortering en het filter
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

De mapbrowser wordt de startpagina: `/` toont de root van `MUSIC_ROOT`, `/map/{*pad}` een submap.

### 1. `src/fs.rs` — mapinhoud opsommen (padlaag)
Nieuw op `Library`:
- `list_directory(&self, relative: &str) -> Result<DirectoryContents, PathError>` — resolvet het pad, eist dat het een map is, leest de entries en geeft submappen + kandidaat-audiobestanden terug (`DirEntry { path, name }`).
- Verborgen items (naam begint met `.`) worden overgeslagen; bestanden alleen op extensie voorgeselecteerd (`has_editable_extension`), want het echte formaatoordeel volgt gratis uit het tag-lezen.
- Elke child wordt via `resolve` opgelost, zodat een symlink die de bibliotheek uit wijst ook in een lijst niet opduikt.
- `breadcrumbs(&self, relative: &Path) -> Vec<(naam, relatief pad)>` blijft hier niet; die zit in de browse-laag omdat het puur weergave is.

### 2. `src/browse.rs` — het weergavemodel van een map
Combineert `fs::` (paden) en `tags::` (tags); roept `lofty` nergens aan.
- `Listing { path, crumbs, folders, tracks, query }`.
- `TrackSummary { name, path, track, title, artist, album, duration, format, has_art }` — `duration` als `mm:ss` (of `u:mm:ss`).
- Een bestand waarvan `tags::read` faalt, is geen bewerkbaar bestand en verdwijnt uit de lijst (dat is precies de formaatcontrole uit `is_editable`, maar met één open in plaats van twee).
- Sortering: tracknummer oplopend, bestanden zonder tracknummer daarachter, binnen gelijke sleutel op bestandsnaam (hoofdletterongevoelig). Dit is de aanname uit de taakbeschrijving voor PRD §12.
- Filter: hoofdletterongevoelige substring op bestandsnaam **of** titel, toegepast na het lezen.

### 3. `src/web/mod.rs` — routes en templates
- `GET /` en `GET /map/{*path}`, beide met `?q=` als filter.
- Het lezen van dertig bestanden is blokkerende I/O; de handler doet dat in `tokio::task::spawn_blocking` zodat de runtime niet vastloopt.
- Bij een `HX-Request`-header wordt alleen het lijstfragment gerenderd, anders de hele pagina. Zonder JavaScript werkt het zoekveld als gewone GET-form — dezelfde URL, dezelfde output.
- Templates: `templates/directory.html` (extends `base.html`) met `{% include "_tracks.html" %}`, plus `templates/tracks.html` voor het HTMX-fragment. Links worden met de `urlencode`-filter opgebouwd.
- `templates/index.html` verdwijnt als losse pagina; de root is nu de mapweergave.

### 4. Stijl (`static/app.css`)
Lijstrijen in plaats van een tabel: titel + tracknummer op de eerste regel, artiest/album eronder, formaat en duur rechts. Blijft leesbaar op een telefoon zonder horizontaal scrollen. Ruimte links in de rij voor de thumbnail uit task-9.

### 5. Tests
- Unit in `browse`: sortering (met en zonder tracknummer), filter op naam en op titel, duurformattering, niet-audio wordt overgeslagen.
- Unit in `fs`: `list_directory` op de root, submappen, weigeren van een pad buiten de root, verborgen bestanden.
- Unit in `web`: `/` rendert, `/map/...` toont de velden, `..` in de URL geeft geen 200.
- Integratie `tests/browse.rs`: echte binary met een gevulde tempdir-root; controleert getoonde velden, sortering en `?q=`. Hiervoor krijgt `tests/common/mod.rs` een `Server::start_in(root, extra)`; `start` delegeert ernaartoe.
- Prestatie: een map met 30 kopieën van een fixture rendert compleet; de eis "< 1 s op de NAS" wordt daar gemeten (task-27), lokaal alleen als rooktest.

### 6. Documentatie
`CLAUDE.md` en `README` bijwerken waar ze de mapbrowser als "volgende fase" noemen; PRD §12 open punt over sortering beantwoorden in de taaknotities.

### Afwijkingen van het plan

- De templates heten `templates/directory.html` (hele pagina) en `templates/listing.html` (het fragment dat HTMX ophaalt); `directory.html` neemt het fragment op met `{% include "listing.html" %}`. In het plan stonden nog de werknamen `_tracks.html`/`tracks.html`.
- URL's worden in `browse::url_for` opgebouwd met `percent-encoding` in plaats van met de askama-filter `urlencode`. Zo blijft de wortel-URL (`/` in plaats van `/map/`) op één plek geregeld en houden de templates geen logica.
- Het filter werkt ook op de namen van submappen. FR-3 noemt alleen bestanden, maar 'zoeken binnen de huidige map' met de submappen ongefilterd laten staan leest als een fout.
- `fs::list_directory` slaat verborgen items over (`.DS_Store` en soortgelijke rommel op een NAS).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Sortering (open punt PRD §12) beantwoord.** Tracknummer uit de tags bepaalt de volgorde; bestanden zonder tracknummer staan achteraan, onderling op bestandsnaam (hoofdletterongevoelig). Vastgelegd in `browse::sort_tracks` en in de README-sectie 'Mapbrowser'.

**Formaatcontrole gebeurt nu door het tag-lezen zelf.** `fs::is_editable` opent een bestand om te bepalen of het echt een MP3 of FLAC is. Voor een maplijst zou dat elk bestand twee keer openen (één keer voor het oordeel, één keer voor de tags). `browse::summarize` laat daarom een bestand vallen zodra `tags::read` faalt — hetzelfde oordeel, de helft van de I/O. Bewaakt door `browse::tests::skips_a_file_that_only_looks_like_audio`.

**`list_directory` controleert elke entry opnieuw tegen de root.** Zonder die stap zou een symlink die de bibliotheek uit wijst wél in de lijst staan en pas bij het aanklikken geweigerd worden. Getest met `fs::tests::listing_hides_a_symlink_pointing_outside`.

**URL-opbouw in Rust, niet in de template.** `percent-encoding` is als directe dependency toegevoegd (stond al in `Cargo.lock` via axum/askama). De askama-filter `urlencode` had ook gekund, maar dan moest elke template de lege-pad-uitzondering voor de wortel-URL zelf afhandelen. Nu levert `browse` kant-en-klare URL's en blijven de templates zonder logica. Getest met mapnamen als `Sigur Rós/( )`.

**HTMX-fragment via de `HX-Request`-header.** Eén route bedient beide gevallen: met de header alleen `templates/listing.html`, anders de hele pagina. Zonder JavaScript is het zoekveld een gewone GET-form naar dezelfde URL en komt hetzelfde resultaat terug. Handmatig in Chrome gecontroleerd: tijdens het typen wordt alleen de lijst vervangen, de URL wordt met `hx-push-url` bijgewerkt en de terugknop herstelt de ongefilterde lijst.

**Prestatie.** Dertig tracks in één map: unit-test `browse::tests::a_directory_with_thirty_tracks_stays_quick` en integratietest `a_directory_with_thirty_tracks_renders_quickly` bewaken de grens van één seconde; lokaal blijft de hele HTTP-ronde ruim onder 100 ms. De maatgevende meting op de NAS zelf hoort bij TASK-27.

**Terzijde opgeruimd:** README en CLAUDE.md verwezen nog naar `testfixtures::kopieer_naar_tempdir(...)`; die functie heet sinds de Engelse hernoeming `copy_to_tempdir`.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Mapbrowser met bestandslijst en zoeken binnen de map

De hoofdnavigatie van Sleeve. De startpagina toont voortaan de wortel van `MUSIC_ROOT`; elke map eronder heeft een eigen URL onder `/map/`. Per map staan de submappen en de bewerkbare audiobestanden met tracknummer, titel, artiest, album, duur en formaat, en een zoekveld dat binnen die map filtert.

### Wat er is toegevoegd

- **`src/browse.rs` (nieuw)** — het weergavemodel van één map. Brengt `fs::` (paden) en `tags::` (tags) samen tot een `Listing` met broodkruimels, submappen en tracks, en regelt sortering, filtering, duurformattering en URL-opbouw. Handlers bevatten daardoor zelf geen presentatielogica.
- **`fs::Library::list_directory`** — somt een map op en controleert elke gevonden entry opnieuw tegen `MUSIC_ROOT`, zodat een symlink die de bibliotheek uit wijst niet eens in de lijst verschijnt. Verborgen bestanden worden overgeslagen.
- **`web::`** — routes `GET /` en `GET /map/{*path}`, beide met `?q=`. Het tag-lezen draait in `spawn_blocking`. Bij een `HX-Request`-header komt alleen het lijstfragment terug, anders de hele pagina.
- **Templates en stijl** — `templates/directory.html` en `templates/listing.html`; `templates/index.html` is vervallen. De tracklijst is opgebouwd uit rijen in plaats van een tabel, zodat er op een telefoon niets horizontaal hoeft te scrollen.

### Beslissingen

- **Sortering (open punt PRD §12):** tracknummer uit de tags, met de bestandsnaam als terugval; bestanden zonder nummer staan achteraan.
- **Bewerkbaarheid** wordt bepaald door het tag-lezen zelf: faalt `tags::read`, dan is het geen MP3 of FLAC en verdwijnt het bestand uit de lijst. Dat is hetzelfde oordeel als `fs::is_editable`, maar zonder elk bestand twee keer te openen — wat op een NAS met dertig tracks per map het verschil maakt.
- **Zonder JavaScript werkt alles**: het zoekveld is een gewone GET-form naar dezelfde URL.
- **`percent-encoding`** toegevoegd als directe dependency (stond al in `Cargo.lock`) om URL's in Rust op te bouwen in plaats van in de templates.

### Tests

93 tests groen (73 unit, 1 architectuur, 9 mapbrowser-integratie, 6 configuratie, 4 server); `cargo fmt --check` en `cargo clippy -- -D warnings` schoon.

- Unit in `browse`: velden uit het tagmodel, sortering, filter op naam én op titel, ontbrekende tags, hoes-detectie, een JPEG die zich als MP3 voordoet, duurformattering, URL-codering.
- Unit in `fs`: opsommen van root en submap, verborgen en niet-ondersteunde bestanden, een symlink naar buiten, een pad buiten de bibliotheek.
- Integratie `tests/browse.rs`: de echte binary met een gevulde tempdir-bibliotheek — getoonde velden, sortering, filter op naam en titel, HTMX-fragment, geweigerde traversal, 404 op een dode link, dertig tracks binnen een seconde, en de controle dat het absolute NAS-pad nergens op de pagina staat. `tests/common/mod.rs` kreeg daarvoor `Server::start_in` en `get_with_headers`.

Handmatig in Chrome gecontroleerd: live filteren tijdens het typen, `hx-push-url`, de terugknop, en de lijst bij een inhoudsbreedte van 360 px zonder horizontale overloop.

### Vervolg

De thumbnail van de embedded hoes is nog een gestreept vlak; het weergavemodel geeft al `has_art` door (TASK-9). De prestatiemeting op de NAS zelf hoort bij TASK-27.
<!-- SECTION:FINAL_SUMMARY:END -->
