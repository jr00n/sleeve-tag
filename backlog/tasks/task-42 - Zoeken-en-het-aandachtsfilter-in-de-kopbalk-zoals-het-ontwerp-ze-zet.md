---
id: TASK-42
title: 'Zoeken en het aandachtsfilter in de kopbalk, zoals het ontwerp ze zet'
status: Done
assignee:
  - Claude
created_date: '2026-08-30 21:01'
updated_date: '2026-08-30 21:12'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
modified_files:
  - templates/directory.html
  - static/app.css
  - tests/browse.rs
  - README.md
  - CLAUDE.md
type: enhancement
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In het ontwerp (Claude Design, project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) staan vier dingen in de kopbalk, naast elkaar op één regel: de naam Sleeve met het ID3-label, een zoekveld ("Filter this folder", met een vergrootglas erin, meegroeiend tot hoogstens 380px), een knop "Needs attention" met een stip en de telling, en de licht/donker-schakelaar.

Sleeve zet daar nu alleen de naam en de schakelaar. Het zoekformulier en de aandachtsknop staan als losse blokken ín de pagina, onder elkaar, elk met een eigen label en omlijsting. Op een breed scherm is de kopbalk daardoor grotendeels leeg terwijl de pagina drie blokken lang is voordat de bestanden in beeld komen.

De haak bestaat al en wordt nergens gebruikt: `base.html` heeft een `{% block kopbalk %}` dat door geen enkele template wordt gevuld. Taak 35 heet zelfs "Filteren op wat aandacht vraagt met de telling in de kopbalk", maar geen van de acceptatiecriteria daar zei waar het terecht moest komen, en het is in de pagina beland.

Dit gaat over de plaats van twee bestaande bedieningselementen, niet over wat ze doen. Zoeken en filteren blijven werken zoals ze werken: samen te gebruiken, de stand in de URL, en zonder JavaScript een gewoon formulier en een gewone link.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Het zoekveld staat in de kopbalk, met het pictogram erin en zonder los label ernaast; het groeit mee met de beschikbare ruimte tot de bovengrens uit het ontwerp.
- [x] #2 De aandachtsknop staat in de kopbalk, met de telling erin, en laat zien of het filter aan of uit staat — ook in woorden en niet alleen met kleur.
- [x] #3 Zoeken en filteren doen precies wat ze deden: ze versmallen samen, de stand staat in de URL en een gefilterde lijst blijft te delen en te bookmarken.
- [x] #4 Zonder JavaScript werken beide: het zoekveld als gewoon formulier, het filter als gewone link.
- [x] #5 Op een smal scherm blijft de kopbalk bruikbaar: de onderdelen vallen netjes onder elkaar en niets valt weg of buiten beeld.
- [x] #6 Op pagina's zonder maplijst (bewerken, hoes, voorbeeld) staat er geen zoekveld of filter in de kopbalk dat daar niets te zoeken heeft.
- [x] #7 De verplaatsing is met tests gedekt: dat de bedieningselementen op de mapweergave in de kopbalk staan en op de andere pagina's niet, en dat de bestaande tests op zoeken en filteren blijven slagen.
- [x] #8 README is bijgewerkt.
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

Puur een verplaatsing: `browse::` en `src/web/mod.rs` blijven ongewijzigd. Het
weergavemodel levert al alles wat de kopbalk nodig heeft (`listing.url`,
`query`, `has_flagged()`, `flagged_count`, `only_flagged`, `attention_url()`).

1. **`templates/directory.html`** — het zoekformulier en het aandachtsblok uit
   `{% block inhoud %}` halen en in een nieuw `{% block kopbalk %}` zetten.
   - Het formulier houdt dezelfde `hx-*`-attributen (`hx-target="#maplijst"`);
     dat werkt over de blokgrens heen omdat het doel-id globaal is. Het staat
     nu al buiten `#maplijst`, dus HTMX-gedrag verandert niet.
   - Het label wordt visueel weggenomen (dezelfde techniek als
     `.filterknop__stand`) in plaats van zichtbaar; een vergrootglas als inline
     SVG (`aria-hidden`) komt ín het veld te staan. De losse knop "Zoek"
     vervalt: één tekstveld betekent dat Enter het formulier gewoon verstuurt,
     ook zonder JavaScript. Het verborgen `aandacht`-veld blijft.
   - De aandachtsknop blijft dezelfde link met dezelfde klassen
     (`filterknop`, `--aan`, `__telling`, `__stand`) zodat de bestaande tests
     blijven slagen; de `✓` wordt een altijd aanwezige stip-slot: `✓` als het
     filter aan staat, een gevulde stip als het uit staat. De regel "Niets in
     deze map vraagt aandacht." blijft ook in de kopbalk staan, als zachte
     tekst in dezelfde plek.
   - "Meerdere bestanden bewerken" blijft in de pagina: die hoort niet bij deze
     taak en niet bij het ontwerp van de kopbalk.
2. **`static/app.css`** — `.kop` mag afbreken (`flex-wrap: wrap`);
   `margin-right: auto` gaat van `.kop__naam` af en `margin-left: auto` komt op
   `.kop__thema`, zodat de schakelaar rechts blijft terwijl het zoekveld
   ertussen meegroeit tot `380px` (`flex: 1 1 14rem; max-width: 380px`). Nieuwe
   regels voor `.kop__zoek`, het pictogram en het weggenomen label; de
   `.zoek*`-regels voor de oude, losse vorm en `.mapfilter` worden aangepast of
   verwijderd. De `.filterknop` krijgt de maat van de kopbalk.
3. **Tests** — in `tests/browse.rs` een test die vaststelt dat het zoekveld en
   de aandachtsknop op de mapweergave vóór `<main` staan (dus in de kopbalk) en
   op de bewerk-, hoes- en voorbeeldpagina niet in de kopbalk voorkomen.
   Bestaande tests op zoeken en filteren blijven ongewijzigd draaien.
4. **README** — de alinea's over het zoekveld en het aandachtsfilter benoemen
   dat beide in de kopbalk staan en alleen op een pagina met een maplijst.

## Risico's

- `tests/rawtags.rs` en `src/web/mod.rs` splitsen op `<main` om de kopbalk niet
  mee te tellen; een `<form>` in de kopbalk mag daar dus staan. Alleen
  `directory.html` vult het blok, dus die pagina's houden een lege kopbalk.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Het zoekveld en de aandachtsknop staan nu in de kopbalk, in het blok dat
`base.html` al had en dat nog door niemand werd gevuld. Er is niets aan hun
werking veranderd: `browse::` en `src/web/mod.rs` zijn niet aangeraakt.

**`templates/directory.html`** vult `{% block kopbalk %}` met het zoekformulier
en de filterlink, die allebei uit `{% block inhoud %}` zijn verdwenen. Het
formulier houdt dezelfde `hx-*`-attributen — `hx-target="#maplijst"` werkt over
de blokgrens heen, want een id geldt over het hele document, en het formulier
stond ook vóór deze taak al buiten `#maplijst`. Het zichtbare label is
weggenomen uit de weergave (wel voorleesbaar), er staat een vergrootglas als
inline SVG in het veld, en de losse knop "Zoek" is vervallen: met één tekstveld
verstuurt Enter het formulier ook zonder script. Het verborgen `aandacht`-veld
blijft, zodat zoeken het filter niet uitzet. De filterlink houdt haar klassen;
de `✓` is een slot geworden dat in beide standen gevuld is — een stip als het
filter uit staat, een vinkje als het aan staat — zodat de stand niet alleen aan
de vulling te zien is. De regel "Niets in deze map vraagt aandacht." staat op
diezelfde plek in de kopbalk.

**`static/app.css`**: `.kop` mag afbreken (`flex-wrap: wrap`), `margin-right:
auto` is van `.kop__naam` naar `margin-left: auto` op `.kop__thema` verhuisd
zodat de schakelaar rechts houdt en het zoekveld ertussen meegroeit tot `380px`
(`flex: 1 1 14rem`). De oude `.zoek*`- en `.mapfilter`-regels zijn vervangen
door `.kop__zoek*` en `.kop__schoon`; `.filterknop` heeft de maat van de
kopbalk gekregen en krimpt daar niet mee.

**Tests** (`tests/browse.rs`): `searching_and_filtering_stand_in_the_header`
stelt vast dat het veld, het pictogram en de knop met de telling vóór `<main`
staan én dat ze niet meer in de pagina eronder staan;
`a_page_without_a_listing_keeps_a_bare_header` doet hetzelfde omgekeerd voor
bewerken, de hoes, de albumweergave en de voorbeeldweergave. De bestaande tests
op zoeken en filteren zijn ongewijzigd en slagen.

**Documentatie**: README beschrijft de kopbalk, het veld zonder knop en de twee
standen van de filterknop; CLAUDE.md heeft er een regel bij die vastlegt dat wat
over de hele lijst gaat in de kopbalk hoort en dat alleen `directory.html` dat
blok vult.

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` en `cargo test`
zijn groen (alle testbinaries, 0 failed).
<!-- SECTION:FINAL_SUMMARY:END -->
