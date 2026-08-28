---
id: TASK-15
title: Albumweergave met selectie en gedeelde velden in een keer zetten (FR-8)
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:24'
updated_date: '2026-08-28 05:51'
labels: []
milestone: m-3
dependencies:
  - TASK-14
documentation:
  - PRD.md
modified_files:
  - src/batch.rs
  - src/browse.rs
  - src/edit.rs
  - src/main.rs
  - src/web/mod.rs
  - templates/album.html
  - templates/albumform.html
  - templates/directory.html
  - static/app.css
  - tests/album.rs
  - tests/common/mod.rs
  - README.md
  - CLAUDE.md
priority: high
type: feature
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Bestand voor bestand corrigeren is te traag voor een heel album. De gebruiker wil meerdere bestanden in een map selecteren (of alles) en de velden die het album deelt in één keer zetten: albumartiest, album, jaar, genre en disc.

Dit is de basis van fase 3; per-bestand overrides, hulpacties en de diff-preview bouwen hierop voort. Het daadwerkelijk wegschrijven gebeurt in de diff-preview-taak, zodat er nooit zonder voorbeeld geschreven wordt.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 In de mapweergave zijn afzonderlijke bestanden te selecteren en is er een 'alles selecteren'-actie
- [x] #2 De selectie blijft behouden tijdens het invullen van de gedeelde velden
- [x] #3 Voor de gedeelde velden albumartiest, album, jaar, genre en disc kan in een keer een waarde voor de hele selectie worden opgegeven
- [x] #4 Een gedeeld veld dat leeg wordt gelaten blijft ongemoeid; er is een expliciete manier om een veld voor de hele selectie te wissen
- [x] #5 Wanneer de geselecteerde bestanden voor een gedeeld veld verschillende waarden hebben, is dat zichtbaar in de invoer
- [x] #6 De weergave werkt op een telefoonscherm, waarbij de tabel horizontaal mag scrollen
- [x] #7 Integratietests dekken selectie, gedeelde-veldinvoer en het onderscheid tussen 'leeg laten' en 'wissen'
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

Een eigen albumpagina per map (`/album/{*path}`), naast de bestaande mapweergave.
De mapweergave is een leeslijst; de albumweergave is de werkbank van fase 3
waarop TASK-16 t/m TASK-18 verder bouwen. Er wordt in deze taak **niets**
geschreven: opslaan komt pas met de diff-preview (TASK-18).

### Nieuw: `src/batch.rs`
- `SharedField` — de vijf gedeelde velden uit FR-8 (albumartiest, album, jaar,
  genre, disc) met formuliernaam, label en `value_of(&Tags)`.
- `Form` — de verstuurde toestand: actie (`alles` / `niets` / bijwerken), de
  geselecteerde bestandsnamen, de vijf waarden en de vijf `wis_`-vinkjes.
  `Form::parse` leest de urlencoded body zelf, omdat `serde_urlencoded` (de
  basis onder `axum::Form` in 0.8) herhaalde sleutels niet naar een `Vec` kan
  deserialiseren.
- `Intent` — per veld het resultaat: `Unchanged`, `Set(waarde)` of `Clear`.
  Dit is het aangrijpingspunt voor de diff-preview van TASK-18.
- `Current` — wat er nú in de selectie staat: `Same`, `Different(waarden)`,
  `Empty` of `None` (niets geselecteerd). Dat is AC #5.
- `album(&Listing, &Form) -> AlbumPage` — het weergavemodel, opgebouwd uit de
  bestaande `browse::listing`.

### Invoermodel (AC #4 en #5)
Het invoerveld is **altijd leeg** bij het openen; het wordt nooit voorgevuld met
de huidige waarde. Daardoor betekent leeg altijd hetzelfde: ongemoeid laten.
Wat er nu staat, is als tekst naast het veld te zien ("Nu: …", "Nu: verschillend
(…)", "Nu: leeg"), plus in de placeholder. Wissen gebeurt met een expliciet
`wissen`-vinkje per veld. Onder het formulier staat per veld wat er bij opslaan
gebeurt; dat maakt het onderscheid leeg-laten/wissen zichtbaar en testbaar
zolang er nog niet geschreven wordt.

### Selectie (AC #1 en #2)
Vinkje per rij (`bestand=<naam>`) plus de knoppen "Alles selecteren" en "Niets
selecteren" (`actie=alles` / `actie=niets`). Bij het openen is alles
geselecteerd. Zonder JavaScript zijn het gewone submitknoppen die de pagina
opnieuw opbouwen; met HTMX post een vinkje het hele formulier naar dezelfde URL
en wordt alleen `#album` vervangen. De ingevulde waarden gaan mee in die POST en
komen er weer uit, dus de selectie én de invoer blijven staan.

### Web
- `/album` en `/album/{*path}`, `GET` en `POST`. `POST` geeft het fragment terug
  bij een `HX-Request` en anders de hele pagina, net als de maplijst.
- `browse::album_url` erbij; `Listing` krijgt `album_url`, zodat de mapweergave
  naar de albumweergave kan linken.
- `edit::parse_number` wordt `pub(crate)` zodat de discnummer-controle op één
  plek staat.

### Templates en CSS
`templates/album.html` (pagina) en `templates/albumform.html` (het fragment dat
HTMX vervangt). De tabel staat in een `overflow-x: auto`-omhulsel; op een smal
scherm scrollt hij horizontaal (AC #6).

### Tests
- Unit-tests in `batch.rs`: parsen van de body, alles/niets, `Current` in de
  vier varianten, `Intent` voor leeg/gevuld/gewist, afgekeurd discnummer.
- `tests/album.rs`: integratietest tegen de binary voor selectie,
  gedeelde-veldinvoer en het onderscheid tussen leeg laten en wissen (AC #7).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Bewijs per acceptatiecriterium.

**#1 selectie en 'alles selecteren'** — `/album/<pad>` toont een vinkje per rij (`bestand=<naam>`) plus de knoppen "Alles selecteren" en "Niets selecteren" (`actie=alles`/`actie=niets`). Getest in `tests/album.rs::the_album_view_opens_with_everything_selected`, `a_single_file_can_be_deselected` en `select_all_and_select_nothing_are_actions_of_their_own`, en in `batch.rs` door `the_all_action_wins_from_what_was_ticked` en `the_none_action_empties_the_selection`.

**#2 selectie blijft behouden** — selectie, waarden en wissen-vinkjes zitten in één formulier en gaan samen mee met elke POST. `tests/album.rs::the_selection_and_the_input_survive_each_other` en `batch.rs::typed_values_survive_a_change_of_selection`. In de browser gecontroleerd: veld invullen → wissen aanzetten → bestand terug aanvinken; de ingetikte waarde stond er daarna nog.

**#3 gedeelde velden in één keer** — de vijf velden uit FR-8 staan op de pagina en leveren per veld een `Intent`. Zichtbaar als "Album wordt “…” in N bestanden."

**#4 leeg laten versus wissen** — een leeg veld levert `Intent::Unchanged` ("blijft ongemoeid"), het wissen-vinkje `Intent::Clear` ("wordt verwijderd uit N bestanden"). `leaving_a_field_empty_is_not_the_same_as_clearing_it` in beide testlagen.

**#5 verschillende waarden zichtbaar** — `Current::Different` levert "Nu: verschillend (“A”, “B”, leeg)." onder het veld, een placeholder die het herhaalt, en een label met het achtervoegsel "· verschillend". `differing_values_are_visible_in_the_input`.

**#6 telefoonscherm** — in Chrome gemeten op een viewport van 390px: `documentElement.scrollWidth == clientWidth` (386), dus de pagina zelf scrollt niet horizontaal, terwijl de tabel dat binnen zijn eigen rand wel doet (818 tegen 352). De gedeelde velden vallen daar onder elkaar, 320px breed. De kolom met de bestandsnaam is `position: sticky` en blijft staan.

**#7 integratietests** — `tests/album.rs`, elf tests, waaronder `nothing_is_written_yet`, die de bestanden byte voor byte vergelijkt na een POST met een ingevulde waarde én een wissen-vinkje. Ook handmatig nagelopen na alle browserinteractie: de checksum van `een.mp3` was gelijk aan die van de fixture.

Kwaliteitspoort: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` en `cargo test` (232 tests) zijn groen.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
De albumweergave staat er: `/album/<pad>` toont de bestanden van één map als tabel met een vinkje per rij en de vijf velden die een album deelt. Er wordt nog niets weggeschreven; dat hoort bij de diff-preview (TASK-18).

## Wat er gebouwd is

**`src/batch.rs`** — het weergavemodel en de formulierlaag van de albumpagina. `SharedField` benoemt de vijf gedeelde velden, `Form::parse` leest de urlencoded body (nodig omdat `serde_urlencoded`, de basis onder `axum::Form`, herhaalde sleutels niet naar een `Vec` kan deserialiseren), `Intent` zegt per veld wat er zou gebeuren en `Current` wat er nú in de selectie staat. De module opent geen bestanden: ze krijgt een `browse::Listing` binnen.

**Web** — `/album` en `/album/{*path}`, GET en POST. De POST bouwt de pagina alleen opnieuw op met de nieuwe selectie; bij een `HX-Request` komt alleen het formulier terug, anders de hele pagina. `browse::album_url` en `Listing::album_url` erbij, zodat de mapweergave ernaartoe linkt.

**Templates en CSS** — `album.html` en `albumform.html`, plus opmaak voor de tabel, de vinkjes en de gedeelde velden.

## De keuze die de pagina draagt

Het invoerveld wordt nooit voorgevuld met de huidige waarde. Daardoor betekent leeg altijd hetzelfde — dit veld blijft in elk bestand zoals het is — en is wissen een aparte, expliciete keuze met een eigen vinkje. Wat er nu staat, is als tekst onder het veld te lezen ("Nu: “X” in de hele selectie", "Nu: verschillend (…)", "Nu: leeg"). Was het veld wél voorgevuld, dan zou leeg soms "ongemoeid" en soms "overschrijven met hetzelfde" betekenen, en dat verschil is bij een selectie van dertig bestanden niet meer te overzien. Onder het formulier staat per veld één zin over wat er bij opslaan gebeurt; dat is de opstap naar de diff-preview.

Het wissen-vinkje maakt het veld `readonly` en niet `disabled`: een uitgeschakeld veld wordt niet meegestuurd, en dan zou het uitzetten van het vinkje de ingetikte waarde alsnog wegnemen.

## Zonder JavaScript

De knoppen zijn gewone submitknoppen en de POST levert dan de hele pagina op. Met HTMX post een vinkje hetzelfde formulier en wordt alleen `#album` vervangen.

## Voor TASK-16 en TASK-18

`Form::intent(field)` is het aangrijpingspunt: de diff-preview vertaalt die voornemens naar een wijziging per bestand. De per-bestand overrides van TASK-16 krijgen hun eigen kolom in dezelfde tabel; de vijf gedeelde velden staan daar bewust los van.
<!-- SECTION:FINAL_SUMMARY:END -->
