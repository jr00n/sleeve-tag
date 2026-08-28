---
id: TASK-22
title: Album art ook als cover.jpg in de albummap wegschrijven (FR-14)
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:26'
updated_date: '2026-08-28 13:31'
labels: []
milestone: m-4
dependencies:
  - TASK-20
  - TASK-21
documentation:
  - PRD.md
modified_files:
  - src/art.rs
  - src/atomic.rs
  - src/fs.rs
  - src/cover.rs
  - src/web/mod.rs
  - templates/cover.html
  - tests/art.rs
  - README.md
  - CLAUDE.md
priority: medium
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Navidrome en vrijwel alle spelers pakken een `cover.jpg` in de albummap op, ook wanneer embedded art ontbreekt of afwijkt. De gebruiker moet daarom bij het instellen van een hoes kunnen kiezen om die ook als bestand in de map te zetten.

Dit is de enige plek waar de app een nieuw bestand in de bibliotheek aanmaakt in plaats van een bestaand bestand te wijzigen; het overschrijven van een bestaande cover.jpg moet dus bewust gebeuren en dezelfde eigendoms- en permissieregels volgen als de rest van de share.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bij het instellen van album art is er een optie om de afbeelding ook als cover.jpg in de albummap te schrijven
- [x] #2 Een bestaande cover.jpg wordt alleen overschreven na expliciete bevestiging door de gebruiker
- [x] #3 De weggeschreven cover.jpg krijgt dezelfde eigenaar, groep en permissies als de overige bestanden in de map
- [x] #4 Het schrijven verloopt atomisch, zodat een afgebroken actie geen half bestand achterlaat
- [x] #5 Een fout bij het schrijven van cover.jpg wordt gemeld maar maakt een geslaagd embedden niet ongedaan
- [x] #6 Een integratietest schrijft cover.jpg in een testmap en controleert inhoud en overschrijfgedrag
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

**Materiële keuze vooraf:** het mapbestand heet altijd `cover.jpg` en is altijd
JPEG. Een geüploade PNG blijft ongewijzigd embedden (dat is de master), maar de
losse kopie voor de spelers wordt naar JPEG gecodeerd. Zo is er één
voorspelbare naam, één overschrijfvraag, en geen bestand waarvan de extensie
liegt.

### 1. `art::as_jpeg(data, quality) -> Result<Vec<u8>, ArtError>`
Levert de bytes ongewijzigd terug als het al JPEG is; anders decoderen en als
JPEG encoderen. Pixelwerk blijft zo in `art::` (architectuurregel).

### 2. `atomic::place(path, contents, model, overwrite) -> Result<Placement, PlaceError>`
Nieuw in `atomic.rs`, want `replace` gaat uit van een bestaand bestand.
- `Placement::{Created, Replaced, Unchanged}` — identieke inhoud raakt het
  bestand niet aan.
- `Overwrite::{Refuse, Allow}`; `Refuse` op een bestaand bestand geeft
  `PlaceError::Exists` en schrijft niets.
- Schrijft naar `TempFile::beside(path)`, neemt eigenaar/groep/rechten over van
  `model` (het audiobestand uit dezelfde map) via het bestaande
  `inherit_metadata`, en hernoemt pas daarna. Faalt het overnemen van eigenaar,
  dan gaat het schrijven niet door.

### 3. Web-laag (`src/web/mod.rs`, `src/cover.rs`, `templates/cover.html`)
- `CoverForm` krijgt `mapbestand` (vinkje "ook als cover.jpg in de map") en
  `overschrijf` (tweede vinkje, alleen zichtbaar als er al een cover.jpg staat).
  De bevestiging moet vóór het versturen gebeuren: na een POST is de
  bestandsinvoer leeg en is een tweede ronde onmogelijk.
- `CoverPage` krijgt `folder_cover: Option<FolderCover>` (naam + grootte van de
  bestaande cover.jpg) zodat de pagina de waarschuwing en het bevestigingsvinkje
  kan tonen.
- Ná het embedden wordt de cover.jpg geschreven; de uitkomst komt als extra
  `SaveResult` in hetzelfde rapport. Een fout of een geweigerde overschrijving
  maakt het geslaagde embedden dus niet ongedaan (AC #5).
- Model voor eigenaar/rechten is het audiobestand waar de pagina over gaat.

### 4. Tests
- Unit in `atomic.rs`: aanmaken, weigeren bij bestaand, overschrijven na
  toestemming, ongewijzigd bij identieke inhoud, rechten overgenomen, geen
  tijdelijk bestand blijft achter.
- Unit in `art.rs`: `as_jpeg` laat JPEG met rust en zet PNG om.
- Integratie in `tests/art.rs`: upload met vinkje schrijft `Album/cover.jpg`
  met de juiste bytes; tweede upload zonder bevestiging laat het bestand staan
  en meldt dat; met bevestiging wordt het wél vervangen.

### 5. Documentatie
`CLAUDE.md` krijgt de regel over `atomic::place` en de cover.jpg; `README.md`
de beschrijving van de optie op de hoespagina.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Uitvoering

- `art::as_jpeg` toegevoegd: JPEG-bytes komen ongewijzigd terug, een PNG wordt
  omgezet. Pixelwerk blijft daarmee binnen `art::` (bewaakt door
  `tests/architecture.rs`).
- `atomic::place` toegevoegd naast `replace`, met `Overwrite::{Refuse,Allow}` en
  `Placement::{Created,Replaced,Unchanged}`. Hergebruikt `TempFile` en
  `inherit_metadata`, dus dezelfde atomische volgorde en dezelfde eigendomsregel
  als elke andere schrijfactie.
- `fs::Library::sibling` toegevoegd: een naam naast een bestand in dezelfde map,
  zonder canonicalisatie (het doel bestaat nog niet) en met weigering van alles
  wat een pad is in plaats van een naam. Padvertaling blijft zo in `fs::`.
- Web-laag: vinkjes `mapbestand` en `overschrijf` op de hoespagina, en
  `write_folder_cover` die ná het embedden draait en een eigen regel aan het
  bestaande `SaveReport` toevoegt.
- `cover::FOLDER_COVER` en `cover::FolderCover` beschrijven wat er al in de map
  staat; de pagina toont dat met de omvang erbij.

## Afwijking van de oorspronkelijke opzet

Geen. De enige materiële keuze (altijd `cover.jpg`, altijd JPEG) stond vooraf in
het plan en is zo uitgevoerd.

## Verificatie

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` en `cargo test`
zijn groen: 259 unit-tests en 87 integratietests, waaronder vier nieuwe in
`tests/art.rs` die het mapbestand controleren.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Album art kan nu ook als losse `cover.jpg` in de albummap gezet worden (FR-14).

**Wat de gebruiker ziet.** Onder het uploadveld op de hoespagina staat een
vinkje "ook als cover.jpg in de albummap zetten". Staat er al zo'n bestand, dan
zegt de pagina dat met de omvang erbij en verschijnt er een tweede vinkje om het
te vervangen. Zonder dat tweede vinkje blijft het bestaande bestand staan en
meldt het rapport waarom. Die bevestiging moet vóór het versturen gegeven
worden: na een POST is de bestandsinvoer van de browser leeg, dus een
"weet je het zeker?"-scherm achteraf zou betekenen dat de afbeelding opnieuw
gekozen moet worden.

**Eén vaste naam, één vast formaat.** Wat de map in gaat heet altijd
`cover.jpg` en is altijd JPEG; een geüploade PNG wordt daarvoor door
`art::as_jpeg` gehaald, terwijl het embedded origineel ongewijzigd PNG blijft.
Een JPEG gaat ongewijzigd de map in. Zo is er één overschrijfvraag en geen
bestand waarvan de extensie liegt.

**Schrijfroute.** Nieuw is `atomic::place`, de tegenhanger van
`atomic::replace`: tijdelijk bestand in dezelfde map, eigenaar, groep en rechten
overnemen van de track ernaast, en pas dan hernoemen. Een afgebroken actie laat
nooit een half bestand achter, identieke inhoud raakt het bestand niet aan, en
zonder expliciete toestemming gaat er niets over een bestaand bestand heen. Het
pad komt van `fs::Library::sibling`, zodat padvertaling in `fs::` blijft.

**Volgorde.** De `cover.jpg` wordt ná het embedden geschreven en krijgt een
eigen regel in hetzelfde rapport. Mislukt het, dan blijft de hoes die al in de
tracks staat gewoon staan.

Nieuwe tests: zes in `atomic::tests` (aanmaken, weigeren, vervangen,
ongewijzigd, rechten, ontbrekend voorbeeld), drie in `art::tests` (`as_jpeg`),
twee in `fs::tests` (`sibling`) en vier in `tests/art.rs` die over HTTP het
bestand in een testmap controleren.
<!-- SECTION:FINAL_SUMMARY:END -->
