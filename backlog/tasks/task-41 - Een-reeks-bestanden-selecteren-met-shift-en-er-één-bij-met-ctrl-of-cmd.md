---
id: TASK-41
title: 'Een reeks bestanden selecteren met shift, en er één bij met ctrl of cmd'
status: Done
assignee:
  - claude
created_date: '2026-08-30 07:13'
updated_date: '2026-08-30 14:05'
labels: []
dependencies:
  - TASK-35
  - TASK-38
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
modified_files:
  - static/app.js
  - static/app.css
  - templates/albumform.html
  - tests/selectie.rs
  - README.md
  - CLAUDE.md
type: enhancement
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) laat een selectie maken zoals in een bestandsbeheerder: klikken op een regel selecteert die, shift-klikken selecteert alles ertussen, en ctrl- of cmd-klikken haalt er één bij of weg.

Sleeve heeft alleen vinkjes. Twintig tracks van een schijf aanvinken is daarmee twintig klikken, terwijl het er twee zouden kunnen zijn.

De vinkjes blijven: ze zijn de weg voor wie geen muis gebruikt, en ze zijn wat er overblijft zonder JavaScript. Dit is een toevoeging bovenop wat er is, geen vervanging ervan.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Klikken op een regel selecteert dat bestand en niets anders; shift-klikken selecteert alles tussen die regel en de vorige klik.
- [x] #2 Ctrl- of cmd-klikken haalt één bestand bij de selectie of eruit, zonder de rest aan te tasten.
- [x] #3 Een reeks volgt de volgorde zoals de lijst er op dat moment uitziet, ook wanneer er gefilterd of gegroepeerd is.
- [x] #4 Klikken in een invoerveld of op een vinkje verandert de selectie niet op een manier die het intikken in de weg zit.
- [x] #5 De vinkjes blijven werken zoals ze deden en blijven de weg voor wie geen muis gebruikt; de selectie is ook met het toetsenbord te maken.
- [x] #6 Zonder JavaScript verandert er niets: de vinkjes doen dan het werk, zoals nu.
- [x] #7 Het selecteren is met tests gedekt voor zover dat zonder browser kan; wat alleen in de browser te zien is, staat als zodanig beschreven.
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

De selectie is server-state: de vinkjes `name="bestand"` posten het hele
formulier (htmx, `hx-target="#album"`, `outerHTML`). Het selecteren met de muis
verandert daar niets aan — het zet dezelfde vinkjes en laat het formulier één
keer posten. Zo blijft er precies één waarheid over wat er geselecteerd is, en
werkt alles zonder JavaScript zoals het deed.

### 1. `templates/albumform.html`
- `data-bestand="{{ row.name }}"` op `<tr class="batchtabel__rij">`. Daarmee kent
  het script de bestandsnaam van een regel zonder in het vinkje te graven, en
  kan een test vasthouden in welke volgorde de regels staan — een reeks volgt
  die volgorde, ook per schijf gegroepeerd (AC #3).
- Verder niets: geen `onclick`, geen knop die zonder script niets doet (AC #6).

### 2. `static/app.js` — nieuw blok "Een reeks selecteren"
Gedelegeerd op `document`, zodat het een htmx-swap overleeft.

- **Anker**: de bestandsnaam van de laatst aangeklikte regel, in een variabele
  die de swap overleeft. Staat die naam niet meer in de tabel (andere map), dan
  telt de klik als een gewone klik.
- **Klik op een regel** (niet op een `input`, `label`, `a` of `button`):
  - kaal → alleen dit bestand;
  - ctrl/cmd → dit bestand erbij of eraf, de rest ongemoeid;
  - shift → de reeks van anker tot hier wordt de selectie.
  Daarna één `form.requestSubmit()`; htmx post en vervangt `#album`. Levert de
  klik dezelfde selectie op als er al staat, dan gaat er geen verzoek uit.
- **Klik op een vinkje**: doet wat het deed (één bestand aan of uit, met zijn
  eigen htmx-post). Met shift erbij wordt die nieuwe stand op de hele reeks
  vanaf het anker gezet; het vinkje post zelf, dus er blijft één verzoek. Dat is
  ook de toetsenbordweg: shift+spatie op een vinkje geeft een klik met
  `shiftKey` (AC #5).
- **Shift+mousedown** op een regel: `preventDefault`, anders selecteert de
  browser de tekst ertussen.
- De tabel krijgt van het script `batchtabel--selecteerbaar`; zonder script komt
  die klasse er niet, en belooft de pagina dus niets wat ze niet waarmaakt.

### 3. `static/app.css`
Alleen onder `.batchtabel--selecteerbaar`: cursor en een lichte hover op de rij.

### 4. Tests — `tests/selectie.rs`
Wat zonder browser te toetsen is (AC #7):
- elke regel draagt `data-bestand`, in de volgorde van de lijst, ook met
  schijfgroepen ertussen;
- de vinkjes zijn ongewijzigd: `name="bestand"`, `value`, `hx-post`;
- zonder JavaScript doet een POST met twee `bestand`-waarden precies wat het
  deed;
- `app.js` bevat het blok, en de geserveerde HTML bevat de JS-only klasse niet.
Wat alleen in de browser te zien is (shift/ctrl, het slepen van het anker),
staat als zodanig in de doc-comment van dat testbestand.

### 5. Documentatie
README: een regel in de tabel onder "Wat de browser er nog bij doet" plus een
alinea over waarom het selecteren dezelfde vinkjes zet. CLAUDE.md: een regel bij
de architectuurregels dat het selecteren met de muis niets nieuws schrijft en
alleen de vinkjes zet.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Geen enkele Rust-regel veranderd. Dat was de uitkomst van de eerste vraag: waar hoort de selectie te wonen? Ze woont al op de server — de vinkjes `name="bestand"` posten het hele formulier — en het enige dat ontbrak, was een tweede manier om diezelfde vinkjes te zetten. Het script doet dus letterlijk wat een mens met de hand zou doen en laat het formulier één keer posten. Een eigen route of een eigen selectiestaat in de browser zou een tweede waarheid hebben opgeleverd die met de eerste uit de pas kan lopen.

Het verzoek gaat via `form.requestSubmit()` en niet via een eigen `fetch` of `htmx.ajax`: alleen `requestSubmit` levert een `submit`-event op, en daar hangt htmx al aan met `hx-target="#album"`. Kan een browser het niet, dan wordt het `form.submit()` — een gewone POST die de hele pagina teruggeeft. Trager, zelfde uitkomst.

Shift op een vinkje leunt op de volgorde die de browser aanhoudt: bij een klik op een checkbox is de nieuwe stand al gezet wanneer het `click`-event bij ons langskomt, en het `change`-event dat htmx laat posten volgt daarna. Wat wij in dat click-moment op de rest van de reeks zetten, reist dus mee in datzelfde ene verzoek — geen tweede post. Dat is met echte muis- en toetsenbordinvoer nagelopen, niet alleen met gescripte events.

Verificatie in een echte browser (chromium via chromedriver, buiten de repo in de scratchpad — het is geen onderdeel van de suite). Drie ronden: (1) gescripte MouseEvents over alle AC's, (2) echte muis- en toetsenbordinvoer via de WebDriver-actions voor klikken, shift-klikken en shift+spatie, (3) een sessie met JavaScript uitgezet. Alles goed. Wat daar te zien was en een `cargo test` niet kan zien: de reeks over een schijfkop heen, het anker dat bij een tweede shift-klik blijft staan, ctrl én cmd, dat er onderweg geen tekst geselecteerd raakt, dat intikken in een rijveld gewoon werkt, en dat zonder script een klik op een regel niets doet terwijl de vinkjes het werk gewoon doen.

`data-bestand` op de rij is de enige toevoeging aan de template. Het alternatief was de bestandsnaam uit het vinkje vissen, maar dan zou de volgorde die een reeks volgt nergens als zodanig staan — en juist die volgorde is wat een test zonder browser kan vastleggen.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Wat er is gebeurd

De albumtabel is te bedienen zoals een bestandsbeheerder: klikken op een regel
selecteert dat ene bestand, shift-klikken selecteert alles tussen die regel en
de vorige klik, en ctrl- of cmd-klikken haalt er één bij of weg zonder de rest
aan te tasten. Twintig tracks aanvinken is daarmee twee klikken.

Het is een toevoeging in de browser en verder niets: **geen enkele Rust-regel is
veranderd**. De selectie woonde al op de server — de vinkjes `name="bestand"`
posten het hele formulier — en het script zet precies diezelfde vinkjes en laat
het formulier één keer posten, via `form.requestSubmit()` zodat htmx het
antwoord in de pagina zet. Zo blijft er één waarheid over wat er geselecteerd
staat. Levert een klik dezelfde selectie op als er al staat, dan gaat er geen
verzoek uit.

## Wijzigingen

- **`static/app.js`** — het blok "Een reeks bestanden selecteren": een
  gedelegeerde `click`-afhandeling die een htmx-swap overleeft, een anker dat als
  bestandsnaam wordt bewaard (het element van zojuist bestaat na een swap niet
  meer) en bij shift op zijn plaats blijft, en een `mousedown`-afhandeling die
  voorkomt dat shift-klikken de tekst ertussen selecteert.
- **`templates/albumform.html`** — `data-bestand` op elke regel. De enige
  toevoeging: daaraan ziet het script om welk bestand een regel gaat, en de
  volgorde waarin de regels staan is de volgorde die een reeks volgt.
- **`static/app.css`** — een hover op de regel, uitsluitend onder
  `.batchtabel--selecteerbaar`. Die klasse zet het script; zonder JavaScript
  staat ze er niet en ziet een regel er dus ook niet uit alsof een klik iets doet.
- **`tests/selectie.rs`** — 7 tests over wat zonder browser vast te stellen is.
- **README.md / CLAUDE.md** — een regel in de tabel "Wat de browser er nog bij
  doet" plus de uitleg, en een architectuurregel dat dit alleen vinkjes zet.

## Wat de vinkjes blijven doen

Ze zijn onaangeroerd: één klik zet één bestand aan of uit, ze zijn de weg voor
wie geen muis gebruikt, en zonder JavaScript doen ze het werk in hun eentje.
Shift werkt ook op een vinkje — dan gaat de stand die het zojuist kreeg over de
hele reeks vanaf de vorige klik — en dat is meteen de toetsenbordweg, want
shift+spatie op een vinkje is een klik met shift. Een klik in een invoerveld, op
een link of op een knop laat de selectie met rust.

## Getest

`cargo fmt --check`, `cargo clippy -- -D warnings` en `cargo test` groen: 505
tests, waarvan 7 nieuw in `tests/selectie.rs` (de greep per regel, de volgorde
over de schijfkoppen heen, de onveranderde vinkjes, de weg zonder JavaScript, en
dat de pagina niets belooft wat ze zonder script niet waarmaakt).

Het gedrag zelf is nagelopen in een echte browser (chromium via chromedriver,
buiten de repo): alle AC's met gescripte events, daarna nog eens met echte muis-
en toetsenbordinvoer, en ten slotte een ronde met JavaScript uitgezet. Alles
goed. Wat alleen in een browser te zien is, staat als zodanig in de doc-comment
van `tests/selectie.rs`.

## Nog te doen

Niets binnen deze taak. Het werk staat ongecommit in de working tree, naast de
eerder afgeronde taak 40.
<!-- SECTION:FINAL_SUMMARY:END -->
