---
id: TASK-36
title: >-
  Een balk die laat zien hoeveel bestanden een wijziging krijgen, en waar je
  vandaan opslaat
status: Done
assignee: []
created_date: '2026-08-30 07:04'
updated_date: '2026-08-30 12:31'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) laat onderaan het scherm een balk staan zodra er iets openstaat: "3 files have staged changes", met daarnaast Verwerpen, Voorbeeld en Opslaan. De balk blijft in beeld terwijl je door de lijst scrollt.

In de albumweergave van Sleeve is nu niet te zien hoeveel bestanden er werkelijk iets krijgen. Je vult gedeelde velden en overrides in, en pas in de voorbeeldweergave blijkt hoeveel bestanden daadwerkelijk veranderen — soms nul, omdat er al stond wat je intikte. Die uitkomst hoort al zichtbaar te zijn terwijl je bezig bent, en de weg naar het voorbeeld en het opslaan hoort niet onderaan een lange tabel te liggen.

Wat de balk zegt, volgt uit wat er in de bestanden staat en uit wat er is ingevuld; het is dezelfde uitkomst die de voorbeeldweergave laat zien, alleen geteld. Er verandert niets aan wanneer er geschreven wordt: de voorbeeldweergave blijft de enige route daarheen, en de balk is een ingang, geen nieuwe route.

Buiten scope: wijzigingen bewaren die niet in het formulier staan, of ze laten overleven tussen mappen of sessies. De balk beschrijft wat er nú in het formulier staat.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 De albumweergave laat doorlopend zien hoeveel bestanden een wijziging zouden krijgen, en dat aantal klopt met wat de voorbeeldweergave daarna toont.
- [x] #2 Staat er niets open, dan zegt de balk dat en zijn opslaan en voorbeeld niet aan te klikken.
- [x] #3 Vanuit de balk zijn de ingevulde velden in één klik leeg te maken, en is de voorbeeldweergave in één klik te bereiken.
- [x] #4 De balk blijft in beeld terwijl je door de tabel scrollt, en dekt op een telefoon niet de regel af waar je mee bezig bent.
- [x] #5 Er wordt niets geschreven vanuit de balk zelf: de voorbeeldweergave blijft de enige stap die naar het schrijven leidt.
- [x] #6 Zonder JavaScript blijft de albumweergave werken zoals ze deed; de telling komt dan mee met de pagina die de server teruggeeft.
- [x] #7 De telling is met tests gedekt, inclusief het geval waarin een ingevulde waarde gelijk is aan wat er al in de bestanden staat en er dus niets verandert.
- [x] #8 README en, waar de regels veranderen, CLAUDE.md zijn bijgewerkt.
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
Gebouwd in worktree `agent-ab690d86c1d2d86c2`, commit e252ce5, gemerged op main.

**Eén berekening voor telling en voorbeeld.** De diff-per-bestand zat verweven
in `preview()` en is losgetrokken tot een private `fn diffs(listing, form) ->
Vec<FileDiff>`. `preview()` schrijft die verschillen uit, `album()` telt de
bestanden die werkelijk veranderen. `AlbumPage::changed_files` kwam eerder uit
`intents(...).len()` en telde bestanden met een *plan* in plaats van bestanden
met een *wijziging* — een waarde die al in het bestand stond telde dus mee. Dat
was de eigenlijke fout achter deze taak; balk en voorbeeld kunnen nu per
constructie niet meer uiteenlopen.

**Nieuw op `AlbumPage`:** `is_pending()` (selectie plus invoer) en
`pending_summary()` (de zin in de balk, inclusief "Geen enkel bestand krijgt een
wijziging: wat er is ingevuld, staat er al.").

**Bewuste keuze.** "Er staat iets open" is *selectie + invoer*, niet
*changed_files > 0*. De voorbeeldknop op de telling afknijpen zou de enige route
afsluiten om een hoes te hangen aan bestanden waarvan de tags al kloppen — de
hoes hoort bij de voorbeeldweergave. Bij een telling van nul zegt de balk dat
dus met zoveel woorden, maar blijft het voorbeeld bereikbaar. Vastgelegd in het
doc-comment op `is_pending`.

**Weergave.** De balk vervangt de oude knop onderaan `albumform.html`:
statusregel plus "Invoer leegmaken" en "Voorbeeld en opslaan" (die laatste
`disabled` zolang er niets openstaat). "Terug naar de map" blijft eronder. De
balk verschijnt alleen als de map bewerkbare bestanden heeft. In `app.css`
`.balk`, `.balk__telling` en `.balk__knoppen` op uitsluitend Nocturne-tokens;
`position: sticky` met `bottom: var(--space-4)` in plaats van `fixed`, zodat de
balk aan het eind van de pagina op zijn eigen plek landt en niets afdekt. Een
`max-width: 30rem`-query stapelt hem op een telefoon, en `scroll-padding-bottom`
plus `scroll-margin-bottom` houden de actieve regel eronder vandaan.

**Tests.** Vier unit-tests in `batch::` (niets open; tellen; een waarde die er al
staat geeft nul en loopt naar 2 zodra er een tweede veld bij komt; en
`the_bar_and_the_preview_never_disagree`, die zeven formulierbodies langsloopt en
`page.changed_files == preview.changing()` afdwingt) plus vier integratietests in
`tests/album.rs` over HTTP, alle tegen een tempdir met fixtures.

**Kwaliteitspoort na de merge met task-35 op main:** `cargo fmt --check` groen,
`cargo clippy --all-targets -- -D warnings` groen, `cargo test` 439 tests groen.

README: nieuwe subsectie "De balk: hoeveel bestanden een wijziging krijgen" vóór
de voorbeeldsectie, plus een noot in de tabel met hulpacties. CLAUDE.md kreeg één
architectuurregel erbij: "Wat er zou veranderen, wordt op één plek uitgerekend"
(`batch::diffs`; de balk is een ingang, geen route).
<!-- SECTION:NOTES:END -->
