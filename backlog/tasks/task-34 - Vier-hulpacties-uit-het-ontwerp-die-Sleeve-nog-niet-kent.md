---
id: TASK-34
title: Vier hulpacties uit het ontwerp die Sleeve nog niet kent
status: Done
assignee: []
created_date: '2026-08-30 07:04'
updated_date: '2026-08-30 07:30'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
modified_files:
  - src/batch.rs
  - src/naming.rs
  - src/main.rs
  - templates/albumform.html
  - tests/album.rs
  - README.md
  - CLAUDE.md
type: feature
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) toont zeven hulpacties bij de albumweergave. Sleeve heeft er drie: hernummeren, artiest → albumartiest en hoofdletters normaliseren. Vier ontbreken, en alle vier gaan over sets die uit meer dan één schijf bestaan of over bestanden die helemaal geen titel hebben — precies de gevallen waarin met de hand invullen het meeste werk is:

- **Hernummeren binnen elke disc** — nu telt hernummeren door over de hele selectie, waardoor de tweede schijf bij 13 begint in plaats van bij 1.
- **Deze schijf nummer N geven** — een selectie in één keer op hetzelfde discnummer zetten. Wat N is, volgt uit wat er al ligt: staat de selectie al op één disc, dan die, en anders de eerstvolgende die nog niet in gebruik is.
- **Disctotalen invullen** — het aantal schijven dat de map bevat in het veld "aantal discs" van alle bestanden zetten. Zonder dat totaal weten spelers niet dat een set compleet is.
- **Titel uit de bestandsnaam** — voor bestanden zonder titel. De bestandsnaam is dan de enige plek waar de titel nog staat.

Deze taak gaat alleen over die vier acties. Ze horen zich te gedragen als de drie die er al zijn: een voorstel in het formulier zetten en verder niets — geen bestand gaat open, geen tag wordt geschreven, en "Invoer leegmaken" draait alles in één klik terug.

Dit is een aanvulling op de bestaande albumweergave (FR-10) en geen herziening ervan.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Hernummeren binnen elke disc geeft de bestanden per discnummer een eigen reeks vanaf 1; bestanden zonder discnummer vormen samen één reeks.
- [x] #2 Een selectie is met één klik op hetzelfde discnummer te zetten, en de knop laat vooraf zien welk nummer dat wordt.
- [x] #3 De disctotalen zijn met één klik voor alle bestanden in de map in te vullen, met het aantal schijven dat de map werkelijk bevat.
- [x] #4 Voor een bestand zonder titel is de titel uit de bestandsnaam af te leiden en als voorstel in te vullen; een bestand dat al een titel heeft, wordt niet overschreven.
- [x] #5 Elke actie vult alleen invoervelden: er gaat geen bestand open en er wordt niets geschreven, en "Invoer leegmaken" draait het voorstel in één klik terug.
- [x] #6 Een voorstel dat gelijk is aan wat er al staat, wordt niet ingevuld.
- [x] #7 De vier acties zijn met tests gedekt, inclusief de randgevallen: bestanden zonder discnummer, een selectie die over meerdere schijven loopt, en een bestandsnaam waar geen titel uit te halen valt.
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
De vier acties zijn `Action`-varianten in `batch::` en gedragen zich als de drie bestaande: `Form::applied` zet een voorstel in het formulier en verder niets.

- **Hernummeren per schijf** (`hernummer-disc`): een teller per `tags.disc`, met `None` als eigen groep. Een bestand dat al op het voorgestelde nummer staat, krijgt geen voorstel.
- **Deze schijf nummer N geven** (`disc`): `batch::disc_suggestion` is publiek omdat het opschrift van de knop het nummer vooraf toont (`AlbumPage::disc_suggestion`). Precies één discnummer in de selectie → dat nummer; anders de eerstvolgende die in de map nog vrij is.
- **Disctotalen invullen** (`disctotaal`): vraagt om een gedeeld veld dat er nog niet was, dus `SharedField::DiscTotal` is erbij gekomen (arrays 5 → 6, `set_field` kent `disc_total`). Dit is de enige hulpactie die ook de selectie aanraakt — `resolve_selection` vinkt de hele map aan, want het aantal schijven hoort in élk bestand van de set.
- **Titel uit bestandsnaam** (`titelnaam`): de tekstlogica staat in de nieuwe module `naming::`, in dezelfde geest als `casing::` — geen tags, geen bestanden, in en uit gaat tekst. Extensie eraf, underscores naar spaties, leidend tracknummer (maximaal drie cijfers, gevolgd door een scheidingsteken) weg. Alleen voor bestanden zonder titel.

Gedekt door unit-tests in `batch::` en `naming::` (inclusief de randgevallen: geen discnummer, een selectie over meerdere schijven, een naam waar niets uit te halen valt) en twee integratietests in `tests/album.rs`; de nieuwe acties staan ook in de bestaande "er wordt niets geschreven"-lus.
<!-- SECTION:NOTES:END -->
