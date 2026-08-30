---
id: TASK-40
title: De hoes naast de lijst in plaats van op een eigen pagina
status: Done
assignee:
  - claude
created_date: '2026-08-30 07:13'
updated_date: '2026-08-30 13:26'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) zet de hoes in een paneel naast de bestandslijst: de afbeelding, wat erover te zeggen valt ("JPEG · 1000×1000 · 284 KB", of "wisselt binnen de selectie"), een knop om hem te vervangen, een knop om hem in de selectie te zetten, en het vinkje om ook een cover.jpg in de map te schrijven.

Sleeve heeft dit allemaal, maar verspreid: de hoespagina hoort bij één bestand, en een hoes voor een selectie zit in de voorbeeldweergave van een batch. Terwijl je de tabel invult, is niet te zien welke hoes er in die bestanden zit.

De architectuurregel blijft staan: een hoes reist alleen mee in de laatste stap. Het paneel toont en kiest; wat ermee gebeurt, beslist de gebruiker in de voorbeeldweergave, en dat blijft de enige route naar het schrijven.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 De albumweergave toont de hoes van de selectie naast de lijst, met formaat, afmetingen en grootte erbij.
- [x] #2 Loopt de selectie over bestanden met verschillende hoezen, of hebben sommige er geen, dan zegt het paneel dat in plaats van er één uit te kiezen en de rest te verzwijgen.
- [x] #3 Een nieuwe hoes is vanuit het paneel te kiezen of erheen te slepen, en de knop noemt op hoeveel bestanden hij terechtkomt.
- [x] #4 De keuze om ook een cover.jpg in de map te schrijven staat bij die actie, en gaat mee naar de stap die werkelijk schrijft.
- [x] #5 Er wordt niets geschreven vanuit het paneel zelf: de afbeelding reist mee in de laatste stap, en de voorbeeldweergave blijft de enige route naar het schrijven.
- [x] #6 De bestaande hoespagina per bestand blijft werken; deze taak voegt een weg toe en haalt er geen weg.
- [x] #7 Op een smal scherm valt het paneel onder of boven de lijst en dringt het de tabel niet weg.
- [x] #8 Het paneel is met tests gedekt, inclusief een selectie met verschillende hoezen en een selectie waarin niet elk bestand er een heeft.
- [x] #9 README en, waar de regels veranderen, CLAUDE.md zijn bijgewerkt.
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

Het paneel is een weergave en een ingang, geen route naar het schrijven. De
afbeelding blijft in de laatste stap reizen (voorbeeldweergave → `actie=opslaan`).

1. **`batch::CoverPanel`** (nieuw, in `batch.rs`): krijgt de gekozen tracks uit
   de listing en beschrijft de hoes van de selectie. Opent geen bestanden en
   raakt geen pixels aan — het kijkt alleen naar de `ArtInfo` die al in de
   listing zit, en laat `cover::CoverDetails` er tekst van maken.
   Toestanden: niets geselecteerd · alle geselecteerde bestanden dezelfde hoes
   (afbeelding + "JPEG · 1000 × 1000 pixels · 284,1 kB") · geen enkel bestand
   een hoes · verschillende hoezen en/of sommige zonder → dan zegt het paneel
   dát, en kiest het er geen uit (AC #2).
   Knopopschrift noemt het aantal bestanden (AC #3).
2. **`AlbumPage`** krijgt `cover: CoverPanel`, plus `max_upload_mb` en
   `folder_cover` die de webhandler invult (zoals `describe_preview` dat voor
   het voorbeeld doet) — `batch::` leest geen omgeving en opent geen map.
3. **`templates/albumform.html`**: de tabel en het paneel komen in een
   `albumlayout`-grid te staan, binnen hetzelfde ene formulier (geneste
   formulieren bestaan niet, en zo reizen de vinkjes vanzelf mee met elke POST).
   Het paneel bevat: de hoes + de feiten, het vinkje "ook als cover.jpg in de
   albummap" (+ het overschrijfvinkje als er al een staat, AC #4), een
   sleep-/kiesvak dat `hidden` staat en pas door JS zichtbaar wordt, en een
   knop `actie=voorbeeld` die het aantal bestanden noemt.
4. **De hoes zelf reist niet mee vanuit het paneel.** Het bestandsveld in het
   paneel draagt géén `name` en wordt dus nooit verstuurd; `app.js` onthoudt het
   gekozen bestand en zet het na de htmx-wissel in het `afbeelding`-veld van de
   voorbeeldweergave. Zonder JavaScript is het paneel een knop die naar de
   voorbeeldweergave leidt, waar het bestandsveld al staat. In beide gevallen
   gaat de afbeelding precies één keer over de lijn, in de stap die schrijft
   (AC #5).
5. **`app.js`**: neerzetvakken worden ook ná een htmx-wissel aangesloten (dat
   gebeurde nog niet, waardoor het slepen in de voorbeeldweergave stil uitviel),
   en het onthouden bestand wordt daar ingezet.
6. **`Preview`** krijgt `folder_cover_wanted` / `overwrite_wanted`, zodat de
   keuze uit het paneel in de voorbeeldweergave aangevinkt terugkomt en mee gaat
   naar het opslaan (AC #4).
7. **CSS**: `albumlayout` als grid, paneel naast de lijst; onder een smal scherm
   valt het paneel boven de tabel en dringt het niets weg (AC #7). Alleen tokens.
8. **Tests** (AC #8): unit-tests in `batch.rs` voor de vier toestanden van het
   paneel, en integratietests in `tests/album.rs` — paneel met feiten, gemengde
   selectie, selectie waarin niet elk bestand een hoes heeft, en dat een POST
   vanuit het paneel geen byte schrijft.
9. **Docs** (AC #9): README en de architectuurregel over "een hoes reist alleen
   mee in de laatste stap" in CLAUDE.md aanvullen met het paneel.

De bestaande hoespagina per bestand blijft ongemoeid (AC #6).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Uitgevoerd volgens het plan; twee dingen zijn onderweg bijgesteld.

**De hoes reist nog steeds alleen in de laatste stap.** Het bestandsveld in het paneel draagt geen `name` en wordt dus door geen enkele POST meegenomen — ook niet door de htmx-post die elk vinkje doet. `app.js` onthoudt het gekozen bestand in een variabele en zet het na de wissel in het `afbeelding`-veld van de voorbeeldweergave (via `DataTransfer`). Zonder JavaScript staat het veld `hidden` en is het paneel een knop naar diezelfde voorbeeldweergave. In beide gevallen gaat de afbeelding één keer over de lijn, in het verzoek dat schrijft.

**Onderweg gerepareerd:** de neerzetvakken werden alleen bij `DOMContentLoaded` aangesloten. Na een htmx-wissel — dus in de hele voorbeeldweergave — viel het slepen daardoor stil. `sluitAan()` loopt nu ook op `htmx:load`, met `data-aangesloten` als markering tegen dubbel aansluiten.

**Verificatie.** `cargo test` (342 unit- + 47 albumintegratietests, alles groen), en de pagina met de hand opgevraagd tegen een tempdir met fixtures: het paneel toont bij drie bestanden met twee verschillende hoezen "De hoes wisselt binnen de selectie: 2 verschillende in deze 3 bestanden", zonder afbeelding, en bij een lege selectie "Er is niets geselecteerd". Wat níét met een test gedekt is, is het slepen zelf: dit project heeft geen browsertests, en dat geldt net zo voor de bestaande neerzetvakken op de hoes- en bewerkpagina. Gedekt is de markup eromheen en dat er geen byte geschreven wordt.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Wat er is gebouwd

De albumweergave toont de hoes van de selectie in een paneel naast de bestandslijst, met formaat, afmetingen en omvang erbij ("JPEG · 300 × 300 pixels · 1,3 kB"). Loopt de selectie uiteen, dan kiest het paneel er géén uit maar zegt het wat er aan de hand is: "De hoes wisselt binnen de selectie: 2 verschillende in deze 6 bestanden" of "Eén hoes in 4 van de 6 bestanden; de rest heeft er geen".

## Hoe

- **`batch::CoverPanel`** (nieuw) beschrijft de hoezen van de aangevinkte bestanden uit de `ArtInfo` die al in de listing zit; de feiten komen van `cover::CoverDetails`. Er gaat geen bestand open en er wordt geen pixel aangeraakt. `AlbumPage` draagt het paneel, plus `max_upload_mb` en `folder_cover` die de webhandler invult (`describe_album`, naar het voorbeeld van `describe_preview`).
- **Het paneel schrijft niets.** Het zit in hetzelfde ene albumformulier, dus de vinkjes reizen met elke POST mee. De knop noemt op hoeveel bestanden de hoes terechtkomt en of dat toevoegen of vervangen is, en leidt naar de voorbeeldweergave — die de enige route naar het schrijven blijft.
- **De afbeelding reist alleen in de laatste stap.** Het bestandsveld in het paneel draagt geen `name` en wordt nooit verstuurd; `app.js` onthoudt de keuze en zet hem na de htmx-wissel in het `afbeelding`-veld van de voorbeeldweergave. Zonder JavaScript staat dat veld `hidden` en is het paneel een knop naar diezelfde stap.
- **Het vinkje voor `cover.jpg`** staat bij die actie en komt in de voorbeeldweergave aangevinkt terug (`Preview::folder_cover_wanted` / `overwrite_wanted`).
- **Layout**: `albumlayout` als grid; vanaf 62rem staat het paneel naast de tabel, daaronder eronder. De tabel houdt zijn eigen scrollcontainer (`minmax(0, 1fr)`) en wordt nergens weggedrukt. Alleen tokens in de CSS.
- **Onderweg gerepareerd**: neerzetvakken werden alleen bij `DOMContentLoaded` aangesloten, waardoor het slepen in de voorbeeldweergave na een htmx-wissel stil viel.

## Tests

Acht unit-tests in `batch.rs` (gedeelde hoes, uiteenlopende hoezen, niet elk bestand een hoes, geen enkele hoes, selectie kleiner dan de map, niet-vierkant, lege selectie, en dat de `cover.jpg`-keuze naar het voorbeeld reist) en vier integratietests in `tests/album.rs` tegen de echte binary, waaronder één die byte voor byte vasthoudt dat een POST vanuit het paneel niets schrijft en ook geen losse `cover.jpg` achterlaat. `cargo fmt --check`, `cargo clippy -- -D warnings` en `cargo test` zijn groen.

## Niet gedekt

Het slepen zelf: dit project heeft geen browsertests, net zomin voor de bestaande neerzetvakken op de hoes- en bewerkpagina. De markup eromheen en het uitblijven van schrijfacties zijn wel gedekt.

## Ongemoeid

De hoespagina per bestand (`/hoes/<pad>`) is niet aangeraakt; deze taak voegt een weg toe en haalt er geen weg.
<!-- SECTION:FINAL_SUMMARY:END -->
