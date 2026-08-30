---
id: TASK-38
title: 'De bestandslijst per schijf groeperen, met een kop en een knop per groep'
status: Done
assignee: []
created_date: '2026-08-30 07:13'
updated_date: '2026-08-30 12:54'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) breekt de bestandslijst op in groepen per schijf: "Disc 1", "Disc 2", en apart "No disc number" voor wat er buiten valt. Elke kop noemt hoeveel bestanden de groep telt en hoeveel daarvan aandacht vragen, en heeft een knop om die hele schijf in één keer te selecteren.

Sleeve toont nu één doorlopende lijst. Bij een set van meerdere schijven is daardoor niet te zien waar de ene ophoudt en de volgende begint, en is een hele schijf selecteren handwerk. Juist bij die sets gaat het vaakst iets mis met de nummering.

Deze taak gaat over het tonen en selecteren. Het invullen van disc- en tracknummers hoort bij de hulpacties (task-34).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 De bestandslijst groepeert op discnummer, met per groep een kop en hoeveel bestanden erin zitten.
- [x] #2 Bestanden zonder discnummer vormen een eigen groep die als laatste staat en als zodanig benoemd is.
- [x] #3 De kop van een groep zegt hoeveel bestanden daarin aandacht vragen, of niets wanneer dat er geen zijn.
- [x] #4 In de albumweergave is een hele groep met één klik te selecteren, zonder de rest van de selectie aan te tasten wanneer dat niet de bedoeling is.
- [x] #5 Een map waarin geen enkel bestand een discnummer heeft, ziet er niet anders uit dan nu: één lijst zonder overbodige kop.
- [x] #6 De volgorde binnen een groep blijft de sortering die er al was.
- [x] #7 De groepering is met tests gedekt, inclusief een map met twee schijven, een map zonder discnummers, en een map waarin sommige bestanden er wel en andere er geen hebben.
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Gebouwd in een eigen worktree, commit 07a61e5, gemerged op main (7a4af6b).

**Groeperen in het weergavemodel.** `browse::DiscGroup { disc, start, count,
attention }` met `label()` ("Schijf 1" / "Zonder discnummer"), `key()`,
`count_label()`, `attention_label()` (`None` als er niets aandacht vraagt) en
`describe()`. `disc_groups(&[TrackSummary])` vult een nieuw veld
`Listing::groups`, ná het filteren: de kop telt wat er op het scherm staat,
terwijl de mapsignalering over de hele map blijft gaan. `Listing::is_grouped()`
is alleen waar als érgens een discnummer staat, zodat een map zonder
discnummers er precies uitziet als voorheen (AC #5), en
`Listing::group_starting_at(index)` hangt de kop aan de juiste regel.

**Sortering.** `sort_tracks` sorteert nu disc-major (disc → tracknummer → naam).
Binnen een schijf verandert er niets (AC #6), en zonder discnummers is de
uitkomst identiek aan voorheen.

**Selecteren per groep.** `batch::Action::ToggleGroup(Option<u32>)`, geparsed uit
een knopwaarde `schijf:1` / `schijf:` (een onleesbare waarde doet niets). Staat
de groep volledig aan, dan gaat hij eraf, anders erbij; bestanden buiten de
groep blijven ongemoeid (AC #4). Het is geen hulpactie: de invoervelden blijven
met rust. `GroupHeading` bouwt de kop uit de `DiscGroup` plus de selectie, en het
knopopschrift zegt wat een klik doet ("Schijf 1 selecteren" / "uitvinken").

**Weergave.** In `albumform.html` één `<tr class="batchtabel__groep">` met
`colspan="99"`, bewust ruim zodat de kop blijft kloppen naast de kolommen van
task-39. In `listing.html` een insertie in de trackslus. CSS-blok onderaan
`app.css`, alleen bestaande tokens.

**Tests.** Zeven in `browse::` (twee schijven, disc vóór tracknummer, groep
zonder discnummer achteraan, telling plus "vraagt aandacht", map zonder
discnummers, deels wel/geen discnummer, groepen volgen het filter), negen in
`batch::` (koppen, selecteren en uitvinken zonder de rest te raken,
knopopschrift, groep zonder discnummer, knop vult geen velden, onleesbare
waarde, aandachtstelling), drie integratietests in `tests/album.rs` — waaronder
een set van twee schijven, want geen fixture heeft disc 2: die wordt via de app
zelf in een tempdir weggeschreven — en twee in `tests/browse.rs`.

**Bij de merge opgelost:** de worktree vertrok van 5e43409 en kende task-35, -36
en -39 dus niet. De agent voegde een eigen `TrackSummary::needs_attention` toe
die sinds task-35 al bestond (doc-comments samengevoegd, één implementatie
behouden); conflicten in `app.css`, `tests/album.rs` en `tests/browse.rs` waren
zuivere toevoegingen aan het eind en zijn uit de merge-stages gereconstrueerd;
in `batch.rs` stonden `RowInput` (task-39) en `GroupHeading` op dezelfde plek,
beide behouden; en één test gebruikte nog `Row::track_input`, dat task-39 heeft
vervangen door `Row::input(RowField::Track).value`.

**Kwaliteitspoort na de merge:** fmt, clippy `--all-targets` en 486 tests groen.
<!-- SECTION:NOTES:END -->
