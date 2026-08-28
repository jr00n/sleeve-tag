---
id: TASK-21
title: >-
  Album art uploaden, embedden in een of alle tracks, en verwijderen (FR-13,
  FR-16)
status: Done
assignee: []
created_date: '2026-08-26 22:26'
updated_date: '2026-08-28 13:12'
labels: []
milestone: m-4
dependencies:
  - TASK-13
  - TASK-19
  - TASK-20
documentation:
  - PRD.md
modified_files:
  - src/tags/mod.rs
  - src/cover.rs
  - src/web/mod.rs
  - src/art.rs
  - src/browse.rs
  - templates/cover.html
  - static/app.css
  - tests/art.rs
  - tests/common/mod.rs
  - Cargo.toml
  - README.md
  - CLAUDE.md
priority: high
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De belangrijkste art-actie: vanaf tablet of telefoon een hoes uploaden en die in één track of in alle geselecteerde tracks van het album embedden, plus het kunnen verwijderen van bestaande art.

De art wordt weggeschreven als front cover (APIC type 3 voor MP3, METADATA_BLOCK_PICTURE type 3 voor FLAC) via de bestaande tags-module en de atomische schrijfhelper, zodat dezelfde integriteitsgaranties gelden als voor tekstuele tags. Verwerking van de afbeelding zelf (validatie en verkleinen) gebeurt door de beeldverwerkingslaag.

Net als bij batch-tagbewerking geldt: een fout bij één bestand blokkeert de overige bestanden niet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Een JPEG of PNG kan vanuit de browser worden geupload en in het geopende bestand worden geembed als front cover
- [x] #2 Dezelfde geuploade art kan in een keer in alle geselecteerde tracks van een album worden geembed
- [x] #3 Bestaande embedded art kan uit een bestand of uit alle geselecteerde bestanden verwijderd worden
- [x] #4 Na embedden of verwijderen toont de app de opnieuw ingelezen situatie ter bevestiging
- [x] #5 Bij het embedden in meerdere bestanden wordt per bestand gerapporteerd of het gelukt is; een fout blokkeert de rest niet
- [x] #6 De overige tags van de bewerkte bestanden blijven onveranderd
- [x] #7 Integratietests dekken embedden in een MP3- en een FLAC-fixture, embedden in meerdere bestanden, en verwijderen
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
`tags::write_art(path, Option<(mime, bytes)>, options) -> Result<bool, _>`
wisselt gericht `PictureType::CoverFront` en laat de rest van de tag staan — de
tekstuele velden én andere afbeeldingen. De oude cover wordt eerst verwijderd,
anders belandt de nieuwe ernaast en is niet te zeggen welke een speler kiest.
De `bool` zegt of er werkelijk geschreven is; dezelfde hoes nog eens plaatsen
raakt het bestand niet aan. Hervalidatie in `atomic::replace` leest de hoes
terug en vergelijkt mime én bytes.

De UI zit op de bestaande hoespagina: één multipart-formulier met vier
submitknoppen (`embed-dit`, `embed-alle`, `verwijder-dit`, `verwijder-alle`).
Wat er gebeurt staat in de knoptekst, met het aantal tracks erin; geen los
keuzevakje dat over het hoofd te zien is. De albumweergave is bewust ongemoeid
gelaten: daar zit een urlencoded formulier, en dat multipart maken zou de hele
batch-parser raken voor een actie die hier beter past.

AC #2 leest "alle geselecteerde tracks van een album"; de map ís het album
(zoals ook de albumweergave hem opvat), dus de knop bedient alle tracks in de
map. De doelbestanden komen uit `fs::list_directory` — dat is alleen een
`read_dir`, geen leesronde over de tags.

Het rapport per bestand komt uit `batch::SaveReport`: dat model is daar
ontstaan voor de batch-tagbewerking en stelt precies dezelfde vraag.

Axum's `DefaultBodyLimit` staat standaard op 2 MB; die is opgetrokken naar
`MAX_UPLOAD_MB`, anders zou een hoes van vijf megabyte op een kale 413 stranden
in plaats van op de melding uit `art::prepare`.

`tests/common` heeft er een `post_multipart` bij gekregen; de uploadtests gaan
daarmee over een echte socket naar de echte binary.

Meegenomen: `#![allow(dead_code)]` kon uit `art.rs` — alles wordt nu gebruikt.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Een hoes uploaden, embedden en verwijderen (FR-13 en FR-16), vanaf de
hoespagina die task-19 opleverde.

Eén uploadveld met twee knoppen — alleen dit bestand, of alle tracks in deze
map — en dezelfde twee om een bestaande hoes te verwijderen. Het schrijven gaat
via `tags::write_art`: gericht de front cover wisselen, de rest van de tag
ongemoeid laten, en door `atomic::replace` met hervalidatie. Bestand voor
bestand, met per bestand een uitkomst; een fout houdt de rest niet tegen. Na
afloop toont de pagina de opnieuw ingelezen situatie.

Hiermee is ook AC #3 van task-19 volledig: een bestand zonder hoes toont niet
alleen dát er geen is, maar biedt nu ook de manier om er een toe te voegen.

7 nieuwe unit-tests in `tags::` (embedden in MP3 en FLAC, vervangen,
verwijderen, tags ongemoeid, niets te doen, audio bit-voor-bit gelijk) en 8
nieuwe integratietests over een echte socket. `cargo fmt --check`, `cargo
clippy -- -D warnings` en `cargo test` (248 + 19 + overige) zijn groen. Commit
c52625d.
<!-- SECTION:FINAL_SUMMARY:END -->
