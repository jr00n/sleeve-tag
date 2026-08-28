---
id: TASK-18
title: Diff-preview en batch-opslaan met foutrapportage per bestand (FR-11)
status: Done
assignee: []
created_date: '2026-08-26 22:25'
updated_date: '2026-08-28 11:30'
labels: []
milestone: m-3
dependencies:
  - TASK-13
  - TASK-15
  - TASK-16
  - TASK-17
documentation:
  - PRD.md
modified_files:
  - src/batch.rs
  - src/web/mod.rs
  - templates/albumpreview.html
  - templates/albumpreviewform.html
  - templates/albumform.html
  - static/app.css
  - tests/album.rs
  - README.md
  - CLAUDE.md
priority: high
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Een batch-actie raakt in één klap een heel album. De gebruiker moet daarom vooraf precies zien welk bestand welke wijziging krijgt, inclusief velden die verwijderd worden, en achteraf per bestand zien of het gelukt is.

Regels uit het PRD: opslaan gebeurt bestand-voor-bestand; een fout bij één bestand blokkeert de rest niet en wordt per bestand gerapporteerd. Lege invoer betekent 'veld verwijderen' en dat moet in de preview expliciet als verwijdering zichtbaar zijn.

Dit is de enige route waarlangs batch-wijzigingen worden weggeschreven.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Voor het opslaan toont een preview per bestand welke velden wijzigen, met oude en nieuwe waarde
- [x] #2 Velden die verwijderd worden zijn in de preview expliciet als verwijdering gemarkeerd
- [x] #3 Bestanden zonder wijzigingen worden als zodanig getoond en worden niet aangeraakt bij het opslaan
- [x] #4 Opslaan verwerkt de bestanden een voor een; een fout bij een bestand stopt de verwerking van de overige bestanden niet
- [x] #5 Na afloop toont een resultaatoverzicht per bestand of het gelukt is, met de foutreden bij mislukking
- [x] #6 De gebruiker kan de batch annuleren vanuit de preview zonder dat er iets geschreven is
- [x] #7 Een integratietest voert een batch uit op fixture-kopieen waarvan een bestand niet schrijfbaar is, en controleert dat de overige bestanden correct zijn bijgewerkt en de fout gerapporteerd is
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
De voorbeeldweergave is een eigen stand van dezelfde POST-route:
`actie=voorbeeld` levert `albumpreview.html` (of het fragment voor HTMX),
`actie=opslaan` schrijft, `actie=terug` brengt het formulier terug. De
opslaanknop staat uitsluitend op de voorbeeldpagina, dus schrijven zonder
voorbeeld gebeurt niet.

De formulierstaat gaat als verborgen velden mee (`Preview::hidden`), en dat is
de staat ná een eventuele hulpactie. Daardoor kan er niets anders opgeslagen
worden dan wat het voorbeeld toonde: de velden zijn er niet te bewerken, en de
knop "actie" zelf gaat niet mee.

`batch::` blijft voorstellen. `FileIntent::wanted` past het plan toe op een
tagmodel (pure logica, met dezelfde getalcontrole als het bewerkformulier),
`changes_between` levert de veldwijzigingen die zowel het voorbeeld als het
resultaatoverzicht gebruikt — vooraf beloven en achteraf melden komen zo van
dezelfde berekening. De veldlijst wordt afgeleid uit `RowField` en
`SharedField`, zodat er geen tweede lijst ontstaat.

De schrijflus staat bij de handler (`save_batch`/`save_one` in `web::`), in één
`spawn_blocking`. Per bestand: pad via `fs::Library::resolve`, opnieuw inlezen,
plan toepassen, en alleen schrijven als er werkelijk iets verandert. Een fout
wordt tot een zin teruggebracht en gaat het rapport in; de lus loopt door.
Fouten in de invoer zelf worden vóór de lus afgevangen: dan komt het voorbeeld
terug met wat eraan mankeert.

Na afloop wordt de map opnieuw ingelezen en het formulier leeggemaakt
(`Form::without_input`, de selectie blijft): de getoonde waarden komen dan uit
de bestanden, en dezelfde wijziging wordt niet nog eens voorgesteld.

AC #7 werkt met een fixture-kopie op 0o444. Omdat `atomic::replace` de
permissies meekopieert naar het tijdelijke bestand, faalt het schrijven daar en
blijft het origineel byte voor byte heel. Draait het testproces als root, dan
gelden bestandsrechten niet; de test merkt dat op en slaat zichzelf over in
plaats van iets anders te bewijzen dan ze belooft.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
De voorbeeldweergave vóór het opslaan, en het opslaan zelf (FR-11).

Per bestand welke velden veranderen, met de oude waarde doorgestreept en de
nieuwe ernaast; verdwijnende velden staan er expliciet als verwijdering bij, en
bestanden waar niets mee gebeurt staan er ook in. De hele formulierstaat gaat
verborgen mee, dus er kan niets anders opgeslagen worden dan wat er te zien is;
annuleren brengt het formulier terug zonder dat er iets geschreven is.

Opslaan gaat bestand voor bestand, met een verse leesronde vlak voor elke
schrijfactie. Een fout bij één bestand stopt de rest niet en komt per bestand in
het resultaatoverzicht; klopt de invoer zelf niet, dan wordt er helemaal niets
geschreven. Na afloop toont de tabel wat er werkelijk in de bestanden staat.

10 nieuwe unit-tests in `batch::` en 6 nieuwe integratietests, waaronder een
batch op een map met een niet-schrijfbaar bestand: de rest wordt bijgewerkt, dat
ene blijft byte voor byte heel, en de reden staat erbij. `cargo fmt --check`,
`cargo clippy -- -D warnings` en `cargo test` (221 + 25 + overige) zijn groen.
Commit 4b68bf8.
<!-- SECTION:FINAL_SUMMARY:END -->
