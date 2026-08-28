---
id: TASK-17
title: >-
  Hulpacties voor batch-bewerking: hernummeren, artiest kopieren, hoofdletters
  normaliseren (FR-10)
status: Done
assignee: []
created_date: '2026-08-26 22:25'
updated_date: '2026-08-28 06:27'
labels: []
milestone: m-3
dependencies:
  - TASK-15
documentation:
  - PRD.md
modified_files:
  - src/casing.rs
  - src/batch.rs
  - src/main.rs
  - templates/album.html
  - templates/albumform.html
  - static/app.css
  - tests/album.rs
  - README.md
  - CLAUDE.md
priority: medium
type: feature
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Drie terugkerende correcties die met de hand te veel werk zijn:

1. Tracknummers automatisch nummeren op basis van de huidige sortering in de tabel.
2. Artiest naar albumartiest kopieren voor de selectie.
3. Hoofdlettergebruik normaliseren (optioneel) met een preview vooraf.

Deze acties vullen alleen de invoervelden van de batch-tabel; ze schrijven zelf niets weg. Het wegschrijven gebeurt pas via de diff-preview, zodat de gebruiker altijd ziet wat er gaat gebeuren.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Een actie nummert de geselecteerde bestanden opeenvolgend volgens de huidige sortering in de tabel
- [x] #2 Een actie kopieert per bestand de artiest naar de albumartiest voor de hele selectie
- [x] #3 Een actie normaliseert het hoofdlettergebruik van tekstvelden en toont het resultaat als voorstel voordat het wordt toegepast
- [x] #4 Elke hulpactie vult uitsluitend de invoervelden; er wordt niets naar bestanden geschreven
- [x] #5 Een hulpactie is ongedaan te maken door de invoer terug te zetten voordat er opgeslagen wordt
- [x] #6 Unit-tests dekken de transformaties, inclusief randgevallen zoals namen met voorzetsels en bestaande afkortingen bij hoofdletternormalisatie
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
Vier knoppen naast "Alles/Niets selecteren", als gewone submitwaarden van
`actie`: `hernummer`, `artiest`, `hoofdletters` en `herstel`. `Form::applied`
levert het formulier ná de actie plus een zin over wat er gebeurd is; `album` en
`intents` gaan allebei door die functie, zodat de pagina en het plan van
hetzelfde ingevulde formulier komen. De actie is idempotent, dus tweemaal
toepassen verandert niets.

Voor AC #2 is `RowField::AlbumArtist` toegevoegd: de artiesten van een selectie
hoeven niet gelijk te zijn, dus "per bestand" vraagt om een override per rij.
Albumartiest staat daarmee zowel bij de gedeelde velden als in de tabel — en
daarmee is de precedentieregel uit task-16 (rij wint van gedeeld veld) niet
langer hypothetisch, maar afgedekt door `a_row_wins_from_the_shared_album_artist`.

Nieuwe module `casing::` voor het hoofdlettergebruik: kent geen tags en geen
bestanden, in en uit gaat tekst. Regels: kleine woorden (NL en EN) blijven klein
middenin maar niet aan de randen; hoogstens vier letters in kapitalen is een
afkorting en blijft staan (DJ, BBC, R.E.M., AC/DC); een hoofdletter verderop in
een woord betekent dat het woord zijn eigen vorm draagt (McCartney, iPhone,
d'Angelo); vijf letters of meer in kapitalen is geschreeuw en wordt omgezet.

Normaliseren werkt over wat er al in het veld staat wanneer de gebruiker zelf
iets heeft ingetikt, en anders over wat er in het bestand staat. Gedeelde velden
(album, genre) krijgen alleen een voorstel wanneer de hele selectie er dezelfde
waarde heeft; één veld kan geen twee voorstellen bevatten.

AC #5 is de knop "Invoer leegmaken": die zet het formulier terug op leeg maar
laat de selectie staan.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Drie hulpacties voor de albumweergave, plus een knop om ze terug te draaien
(FR-10). Ze vullen uitsluitend de invoervelden — geen bestand gaat open, er
wordt niets geschreven — zodat opslaan een keuze van de gebruiker blijft.

- Hernummeren volgt de volgorde van de tabel, niet die van de bestaande
  tracknummers.
- "Artiest → albumartiest" werkt per bestand; daarvoor is albumartiest nu ook
  een override per rij, die van het gedeelde veld wint. Een bestand zonder
  artiest wordt overgeslagen in plaats van leeggemaakt.
- Hoofdletters normaliseren zet een voorstel in de velden; wat al klopt krijgt
  er geen.

Nieuwe module `casing::` met 11 unit-tests voor de transformatie inclusief
afkortingen en namen met voorzetsels, 12 nieuwe unit-tests in `batch::` en 4
nieuwe integratietests. `cargo fmt --check`, `cargo clippy -- -D warnings` en
`cargo test` (211 + 19 + overige) zijn groen. Commit 6dcaf79.
<!-- SECTION:FINAL_SUMMARY:END -->
