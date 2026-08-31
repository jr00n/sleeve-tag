---
id: TASK-44
title: 'Kijken en bewerken op één scherm: de tag-editor naast de bestandslijst'
status: Done
assignee:
  - Claude
created_date: '2026-08-30 21:02'
updated_date: '2026-08-30 22:08'
labels: []
dependencies:
  - TASK-43
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
modified_files:
  - src/batch.rs
  - templates/albumform.html
  - templates/album.html
  - static/app.css
  - tests/album.rs
  - README.md
  - CLAUDE.md
type: enhancement
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Dit is de grootste afwijking tussen het ontwerp en Sleeve, en het raakt aan een keuze die eerst gemaakt moet worden.

**Wat het ontwerp toont.** In Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) is een map openen één scherm met twee panelen naast elkaar. Links een paneel van hoogstens 420 pixels met drie onderdelen onder elkaar: de hoes (vierkant, met "Replace…", "Embed in N files" en het vinkje "Also write cover.jpg into the folder"), daaronder de gedeelde velden (titel, artiest, albumartiest, album, dan tracknummer/disc/discs/jaar in vier kolommen naast elkaar, dan genre), en daaronder de hulpacties onder een eigen kopje. Rechts, drie keer zo breed, de bestandentabel met daarboven de mapnaam, "3 of 10 selected" en "3 discs". Onderin zweeft een balk met de stand van de wijzigingen.

**Wat Sleeve doet.** Dezelfde onderdelen bestaan allemaal, maar ze staan verdeeld over twee pagina's en onder elkaar in plaats van naast elkaar. `/map` is alleen kijken en heeft een knop "Meerdere bestanden bewerken" naar `/album`. Daar staat eerst een alinea uitleg, dan de telling met "Alles/Niets selecteren", dan een blok van acht hulpactieknoppen, dan pas de tabel, met het hoespaneel ernaast (dat stuk volgt het ontwerp al), en de gedeelde velden staan als `<fieldset>` ver ónder de tabel. Op een scherm van 1440 bij 900 begint de tabel pas op tweederde van de hoogte en zijn de gedeelde velden niet in beeld.

**De keuze die eerst gemaakt moet worden.** Het ontwerp kent het onderscheid tussen "een map bekijken" en "een selectie bewerken" niet: dat is één scherm. Sleeve heeft er twee pagina's van gemaakt, en dat zit ook in de architectuurregels: de albumweergave stelt alleen voor, en de voorbeeldweergave is de enige route die schrijft. Die laatste regel staat hier niet ter discussie — het gaat om de vraag of `/map` en `/album` samenvallen tot één scherm, of dat de tag-editor alleen op de albumweergave naast de lijst komt te staan. Dat is een productbeslissing en hoort aan de gebruiker voorgelegd te worden voordat er code wordt geschreven.

Wat er in beide gevallen verandert: de gedeelde velden en de hulpacties verhuizen naar een paneel naast de lijst, samen met het hoespaneel dat daar al staat, en de lijst krijgt de ruimte die het ontwerp hem geeft.

Buiten scope: de voorbeeldweergave vervangen door een modaal venster. Dat toont het ontwerp wel, maar de aparte pagina is de vastgelegde route naar het schrijven en die regel blijft staan.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 De keuze of `/map` en `/album` één scherm worden, is aan de gebruiker voorgelegd en het antwoord staat in de taak vastgelegd voordat er code wordt geschreven.
- [x] #2 De gedeelde velden, de hulpacties en de hoes staan samen in één paneel naast de bestandslijst, in de volgorde die het ontwerp aanhoudt.
- [x] #3 Tracknummer, disc, discs en jaar staan in dat paneel naast elkaar en niet elk op een eigen regel.
- [x] #4 De bestandslijst krijgt het grootste deel van de breedte; op een smal scherm valt het paneel onder de lijst zonder dat de lijst wordt weggedrukt.
- [x] #5 Boven de lijst staat de mapnaam met de telling van de selectie en het aantal schijven, met de knoppen om alles of niets te selecteren ernaast.
- [x] #6 Wat de gedeelde velden doen verandert niet: leeg laten betekent ongemoeid laten, wissen blijft een eigen vinkje, en een waarde per rij wint van het gedeelde veld.
- [x] #7 De hulpacties vullen nog steeds alleen invoervelden en schrijven niets.
- [x] #8 Opslaan blijft uitsluitend via de voorbeeldweergave lopen; geen enkele knop in dit scherm schrijft rechtstreeks.
- [x] #9 Zonder JavaScript blijft het scherm bruikbaar: de vinkjes, de velden en de knoppen doen het werk zoals ze dat nu zonder script doen.
- [x] #10 De indeling is met tests gedekt voor zover dat zonder browser kan, en wat alleen in de browser te zien is staat als zodanig beschreven.
- [x] #11 README en CLAUDE.md zijn bijgewerkt, inclusief de architectuurregels die door de gekozen indeling veranderen.
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
## De beslissing bij AC #1 (voorgelegd en beantwoord)

Voorgelegd zijn drie opties: (a) alleen `/album` herindelen, (b) `/map` en
`/album` samenvoegen tot één scherm, (c) samenvoegen op basis van de inhoud van
de map (map met tracks opent meteen in het bewerkscherm).

**Antwoord van de gebruiker: (a) — alleen `/album` herindelen.** `/map` blijft
kijken: de tabel over de volle breedte, het zoekveld en het aandachtsfilter in
de kopbalk, de submappen, en de knop "Meerdere bestanden bewerken" naar
`/album`. De tag-editor komt naast de lijst te staan op de albumweergave.

Wat daarmee blijft staan: de architectuurregel dat de albumweergave alleen
voorstelt, dat de voorbeeldweergave de enige route naar het schrijven is, en dat
de mapweergave geen selectie kent (en dus geen vinkjeskolom — de afwijking die
TASK-43 vastlegde blijft dus zoals hij is).

Blijvende afwijking van het ontwerp: het ontwerp kent één scherm per map, Sleeve
houdt er twee.

## Plan

1. **`src/batch.rs`**
   - `SharedField::ALL` op de volgorde van het ontwerp zetten: albumartiest,
     album, jaar, discnummer, aantal discs, genre. Zo staan de drie korte velden
     naast elkaar en sluit genre de rij af.
   - `SharedField::is_compact()` erbij: welke velden smal genoeg zijn om naast
     elkaar te staan (jaar, discnummer, aantal discs). Dat is iets anders dan
     `is_numeric()` — het jaar is in het tagmodel tekst.
   - `SharedInput.compact` erbij en `AlbumPage::field_rows()`: dezelfde platte
     lijst `fields` blijft bestaan (de gevolgweergave en de tests gebruiken
     hem), met daarnaast een gegroepeerde weergave voor de indeling.
   - `AlbumPage.discs` erbij plus `selection_summary()`: de telling van de
     selectie met het aantal schijven erachter, voor de kop boven de lijst.

2. **`templates/albumform.html`** — de indeling uit het ontwerp:
   - Links `aside.editor` (hoogstens 26rem ≈ 420px): hoes → gedeelde velden →
     hulpacties onder een eigen kopje, met de gevolgweergave eronder.
   - Rechts de kop boven de lijst (mapnaam, selectietelling, aantal schijven,
     alles/niets) en daaronder de tabel.
   - De tabel staat eerst in de DOM en gaat op een breed scherm met
     `grid-column` naar rechts; op een smal scherm valt het paneel er vanzelf
     onder zonder dat de tabel wordt weggedrukt.
   - De balk onderaan blijft over de volle breedte plakken.

3. **`templates/album.html`** — de alinea uitleg verhuist naar het paneel, bij
   de velden waar hij over gaat; de mapnaam verhuist naar de kop boven de lijst.

4. **`static/app.css`** — `.albumlayout` wordt de indeling van het hele scherm
   in plaats van alleen tabel + hoes; `.editor`, `.lijstkop` en `.gedeeld__rij`
   erbij. Alleen tokens, geen kleurwaarden.

5. **Tests** — `tests/album.rs` en de unit-tests in `src/batch.rs`: de volgorde
   en groepering van de gedeelde velden, de kop boven de lijst, en dat het
   paneel de drie onderdelen in de volgorde van het ontwerp bevat. Wat alleen in
   de browser te zien is (de kolombreedte, het inklappen op een smal scherm)
   staat als zodanig in de notities.

6. **README en CLAUDE.md** — de architectuurregel over het hoespaneel wordt de
   regel over het hele bewerkpaneel.

## Afwijking bij AC #3

Het ontwerp zet tracknummer, disc, discs en jaar naast elkaar in het paneel.
Sleeve kent geen gedeeld tracknummer: dat verschilt per bestand en wordt in de
tabel zelf ingetikt (FR-9, en de architectuurregel dat een gedeeld veld over de
hele selectie gaat). Een gedeeld tracknummer zou alle geselecteerde bestanden
hetzelfde nummer geven, en daar is de hulpactie "Hernummeren" al voor. Naast
elkaar komen daarom jaar, discnummer en aantal discs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Wat er in de browser is nagelopen** (Chrome, `MUSIC_ROOT` op een scratch-map met 7 kopieën van de fixtures, dus nooit de echte bibliotheek):

- **1900 × 1200** — paneel 416px, lijst 1420px, `documentElement.scrollWidth == clientWidth` (geen horizontale paginascroll). Kop boven de lijst met naam, "7 van 7 bestanden geselecteerd · 1 schijf" en de twee knoppen rechts.
- **1285 breed** — paneel 416, lijst 820. Nog steeds naast elkaar.
- **820 breed** — paneel valt onder de lijst (`top` 711 tegen 119), beide over de volle breedte, en de tabel scrolt binnen zijn eigen rand. AC #4 gehaald.
- **Hulpactie "Titel uit bestandsnaam"** aangeklikt: de melding verschijnt in het paneel, de balk springt naar "1 bestand krijgt een wijziging", er wordt niets geschreven.
- `/map/Album` is onveranderd; TASK-43 blijft staan zoals hij was.

**Twee dingen die tijdens het werk boven kwamen:**

1. **De pagina scrolde horizontaal mee met de tabel.** Op een smal scherm rekte de tabel de impliciete gridkolom van `.albumlayout` op tot zijn eigen breedte (1534px in een venster van 805), waardoor de héle pagina horizontaal ging scrollen in plaats van alleen de tabel binnen `.tabelrand`. Dit bestond al vóór deze taak, maar AC #4 vraagt er letterlijk naar. Opgelost met `grid-template-columns: minmax(0, 1fr)` op de basisregel en `min-width: 0` op de lijstkolom. Niet met een test af te dekken zonder browser; de meting staat hierboven.

2. **De drie korte velden liepen scheef.** In een kolom van een derde paneel breekt "Discnummer · verschillend" over twee regels en "Discnummer uit de selectie wissen" over drie, waardoor de invoervelden van de drie buren op verschillende hoogtes stonden. Opgelost met `grid-template-rows: subgrid` op `.gedeeld__veld`, zodat opschrift, invoer, "Nu: …" en het vinkje op dezelfde regels liggen; en het vinkje van een kort veld heet zichtbaar "Wissen" met de hele zin als `aria-label`. Een browser zonder `subgrid` zet de onderdelen gewoon onder elkaar — niet uitgelijnd, wel compleet.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Kijken en bewerken op één scherm: de tag-editor naast de bestandslijst

De albumweergave (`/album/<pad>`) is opnieuw ingedeeld naar het ontwerp: links een
bewerkpaneel van 420 pixels met de hoes, de gedeelde velden en de hulpacties
onder elkaar; rechts de bestandslijst met daarboven de mapnaam, de stand van de
selectie en het aantal schijven, met "Alles/Niets selecteren" ernaast.

### De beslissing bij AC #1

Voorgelegd zijn drie opties: alleen `/album` herindelen, `/map` en `/album`
samenvoegen, of samenvoegen op basis van de inhoud van de map. **De gebruiker
koos: alleen `/album` herindelen.** `/map` blijft kijken — de tabel over de volle
breedte, zoeken en het aandachtsfilter in de kopbalk, geen selectie en dus geen
vinkjeskolom. De blijvende afwijking van het ontwerp is daarmee vastgelegd: het
ontwerp kent één scherm per map, Sleeve houdt er twee.

### Wat er verandert

- **`src/batch.rs`** — `SharedField::ALL` staat op de volgorde van het ontwerp
  (albumartiest, album, jaar, discnummer, aantal discs, genre). Nieuw:
  `SharedField::is_compact`, `SharedInput.compact`, `AlbumPage::field_rows` (de
  velden gegroepeerd tot de regels waarop ze staan), `AlbumPage.discs` en
  `AlbumPage::selection_summary`. De platte `fields`-lijst blijft bestaan, zodat
  de gevolgweergave en de bestaande tests niet hoeven te veranderen.
- **`templates/albumform.html`** — twee kolommen binnen hetzelfde formulier. De
  lijst staat eerst in de DOM en het paneel erna; op een breed scherm zet het
  grid het paneel links. De alinea uitleg staat nu bij de velden waar hij over
  gaat, en de hulpacties staan onder die velden in plaats van boven de tabel.
- **`templates/album.html`** — de mapnaam en de uitleg zijn naar het formulier
  verhuisd; de pagina houdt de kruimels en neemt de volle breedte.
- **`static/app.css`** — `.albumlayout` is de indeling van het hele scherm
  geworden; `.editor`, `.lijstkop` en `.gedeeld__rij` zijn nieuw. Alleen tokens,
  geen kleurwaarden.

### Twee fouten die onderweg zijn opgelost

- De pagina scrolde horizontaal mee met de tabel op een smal scherm (bestond al
  vóór deze taak, maar AC #4 vraagt er letterlijk naar).
- De drie korte velden stonden op verschillende hoogtes doordat hun opschriften
  verschillend breken; nu uitgelijnd met `subgrid`.

Beide staan met meetwaarden in de notities.

### Afwijking bij AC #3

Het ontwerp zet tracknummer, disc, discs én jaar naast elkaar. Sleeve kent geen
gedeeld tracknummer — dat verschilt per bestand en wordt in de tabel zelf
ingetikt; een gedeeld tracknummer zou elk geselecteerd bestand hetzelfde nummer
geven, en daar is "Hernummeren" al voor. Naast elkaar staan daarom jaar,
discnummer en aantal discs.

### Wat níét is veranderd

Leeg laten betekent nog steeds ongemoeid laten, wissen blijft een eigen vinkje,
een waarde per rij wint van het gedeelde veld, de hulpacties vullen alleen
invoervelden, en de voorbeeldweergave blijft de enige route die schrijft. Zonder
JavaScript zijn het gewone vinkjes en submitknoppen, precies als voorheen.

### Tests

Drie nieuwe integratietests in `tests/album.rs`
(`the_editor_stands_beside_the_list_in_one_panel`,
`the_short_fields_share_one_row`,
`the_list_carries_its_own_heading_with_the_count`) en vier nieuwe unit-tests in
`src/batch.rs` over de groepering, de telling en de lege map. Wat alleen in een
browser te zien is — de kolombreedtes en het inklappen op een smal scherm —
staat als meting in de notities.

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` en `cargo test`
(15 suites, alles groen) slagen.
<!-- SECTION:FINAL_SUMMARY:END -->
