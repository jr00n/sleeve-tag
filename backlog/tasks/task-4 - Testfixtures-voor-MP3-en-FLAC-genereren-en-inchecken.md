---
id: TASK-4
title: Testfixtures voor MP3 en FLAC genereren en inchecken
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:22'
updated_date: '2026-08-26 23:24'
labels: []
milestone: m-0
dependencies: []
documentation:
  - PRD.md
priority: high
type: chore
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Tests mogen nooit tegen de echte muziekbibliotheek draaien. Daarvoor is een set kleine, ingecheckte fixtures nodig onder `tests/fixtures/`: audio van een seconde stilte, eenmalig gegenereerd met ffmpeg, in varianten die de latere fasen kunnen uitdagen.

Benodigde varianten: MP3 en FLAC zonder tags, met volledige tags, met embedded album art, en een MP3 met een bestaande ID3v1-tag (fase 2 moet die opruimen of synchroniseren). Ook een testhelper die fixtures naar een tempdir kopieert, zodat geen enkele test het origineel muteert.

De generatie moet reproduceerbaar zijn: leg het ffmpeg-commando vast zodat een fixture later opnieuw gemaakt kan worden.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `tests/fixtures/` bevat MP3- en FLAC-bestanden in de varianten: geen tags, volledige tags, met embedded art, en een MP3 met ID3v1-tag
- [x] #2 De gebruikte ffmpeg-commando's zijn vastgelegd in een script of README naast de fixtures
- [x] #3 Er is een testhelper die een fixture naar een tempdir kopieert en het pad teruggeeft
- [x] #4 Een test faalt zichtbaar wanneer een fixture ontbreekt, in plaats van stilzwijgend over te slaan
- [x] #5 De totale omvang van de fixtures blijft klein genoeg om comfortabel in Git te leven (richtlijn: onder 1 MB)
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

Fixtures worden eenmalig met ffmpeg gegenereerd door `tests/fixtures/genereer-fixtures.sh` en ingecheckt. Alle audio is
een seconde stilte; wat de fixtures onderscheidt zijn hun tags.

Varianten: MP3 en FLAC zonder tags, met de volledige tagset uit PRD §7, en met embedded front cover. Voor MP3 daarnaast
twee ID3v1-varianten, omdat het PRD eist dat een ID3v1-tag nooit inconsistent achterblijft: één met uitsluitend ID3v1, en
één waarin ID3v1 en ID3v2 verschillende waarden bevatten. Plus losse cover-afbeeldingen (JPEG en PNG) voor de art-taken.

De toegang loopt via `src/testfixtures.rs` (alleen onder `cfg(test)`): benoemde constanten per fixture, een `fixture_pad`
die paniekt met een bruikbare melding als een bestand ontbreekt, en kopieerhelpers naar een tempdir.

## Uitgevoerd

1. Genereerscript geschreven en gedraaid; drie problemen onderweg opgelost (zie de notities).
2. Fixtures geverifieerd met `ffprobe` en op byteniveau: duur, aanwezigheid van ID3v2-header en ID3v1-blok, en tags.
3. `src/testfixtures.rs` met constanten, `fixture_pad`, `kopieer_naar` en `kopieer_naar_tempdir`.
4. Tests: alle fixtures aanwezig en niet leeg, totale omvang onder 1 MB, kopie identiek aan origineel, schrijven naar de
   kopie laat het origineel ongemoeid, meerdere fixtures naast elkaar in één map, en een ontbrekende fixture laat de test
   zichtbaar falen.
5. README en CLAUDE.md bijgewerkt.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Drie fouten in het genereerscript die de fixtures stilzwijgend waardeloos maakten, elk gevonden door de uitvoer te controleren in plaats van aan te nemen dat ffmpeg deed wat er stond:

1. `-t 1` stond na `-i`, waardoor het bij de varianten met een tweede input (de cover) als optie voor die volgende input gold. `anullsrc` liep daardoor oneindig door; het bestand groeide tot 390 MB voordat ik het afbrak. Opgelost door de duur in de bron te zetten: `anullsrc=...:d=1`.
2. `-shortest` kapte vervolgens de audio af tot de lengte van het ene coverframe: 0,039 seconde in plaats van 1. Die vlag is weg, want de bron eindigt nu vanzelf.
3. ffmpeg negeert `-write_id3v2 0` zodra er metadata is. `id3v1-only.mp3` had daardoor gewoon ook een volledige ID3v2-tag (264 bytes, identiek aan tagged.mp3), en `untagged.mp3` begon met een lege ID3v2-header. Opgelost met een expliciete strip-stap in het script.

De fixture `id3v1-inconsistent.mp3` kan niet met ffmpeg alleen gemaakt worden: die schrijft ID3v1 en ID3v2 altijd met dezelfde waarden. De 128 bytes lange ID3v1-tag wordt daarom met de hand aangeplakt, met afwijkende waarden. Dit is precies de situatie die de schrijftaak van fase 2 moet opruimen.

Cover-afbeeldingen (JPEG en PNG) zijn als fixture meegenomen. Ze zijn nodig als tussenstap om de art-varianten te maken, en de art-taken in fase 4 hebben ze sowieso nodig.

Totale omvang 84 KB, ruim onder de richtlijn van 1 MB. Een test bewaakt die grens, zodat een toekomstige fixture het niet ongemerkt oprekt.

De helper staat in src/ onder `cfg(test)` en is daarmee beschikbaar voor unit-tests. Integratietests in tests/ kunnen hem niet importeren, omdat dit een binary crate is. Zodra een integratietest fixtures nodig heeft, is de keuze: hem via `#[path]` delen, of het project opsplitsen in lib + bin. Nu niet gedaan omdat er nog geen behoefte is.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Wat er is gebouwd

Een set ingecheckte testfixtures plus de helper om ze veilig te gebruiken, zodat geen enkele test ooit de echte muziekbibliotheek hoeft aan te raken.

## Fixtures (`tests/fixtures/`, samen 84 KB)

| Bestand | Bijzonderheid |
|---|---|
| `untagged.mp3` / `untagged.flac` | geen enkele tag |
| `tagged.mp3` / `tagged.flac` | volledige tagset uit PRD §7 |
| `tagged-with-art.mp3` / `tagged-with-art.flac` | idem, plus embedded front cover |
| `id3v1-only.mp3` | uitsluitend ID3v1, geen ID3v2 |
| `id3v1-inconsistent.mp3` | ID3v2 zegt "Stilte in D", ID3v1 zegt "Oude titel uit ID3v1" |
| `cover.jpg` / `cover.png` | losse afbeeldingen voor de art-taken |

Alles is één seconde stilte: de tests gaan over tags, niet over geluid. `tests/fixtures/genereer-fixtures.sh` maakt ze opnieuw en documenteert elk ffmpeg-commando.

## Helper

`src/testfixtures.rs` (alleen onder `cfg(test)`): benoemde constanten, `fixture_pad` die paniekt met een bruikbare melding als een bestand ontbreekt, en `kopieer_naar` / `kopieer_naar_tempdir`.

## Tests

37 tests groen (was 31); zes nieuwe:

- alle fixtures aanwezig en niet leeg
- totale omvang onder 1 MB — bewaakt dat een toekomstige fixture de grens niet ongemerkt oprekt
- kopie is byte-identiek aan het origineel
- schrijven naar de kopie laat het origineel in de repo ongemoeid
- meerdere fixtures passen naast elkaar in één map
- een ontbrekende fixture laat de test zichtbaar falen in plaats van stilzwijgend over te slaan

## Drie dingen die ffmpeg anders deed dan het commando suggereerde

Alle drie gevonden door de uitvoer te controleren in plaats van aan te nemen dat het goed ging:

1. `-t 1` na `-i` gold voor de *volgende* input, dus `anullsrc` liep oneindig door — het bestand groeide tot 390 MB. De duur staat nu in de bron zelf.
2. `-shortest` kapte de audio daarna af tot de lengte van het ene coverframe: 0,039 seconde.
3. ffmpeg negeert `-write_id3v2 0` zodra er metadata is, waardoor `id3v1-only.mp3` gewoon óók een volledige ID3v2-tag had. Een expliciete strip-stap lost dat op.

Zonder die controles waren de fixtures groen-maar-waardeloos geweest: tests die "een bestand met alleen ID3v1" heten maar iets anders testen.

## Openstaand

De helper zit in `src/` onder `cfg(test)` en is daarmee alleen voor unit-tests bruikbaar; integratietests kunnen hem niet importeren omdat dit een binary crate is. Zodra een integratietest fixtures nodig heeft, is de keuze: delen via `#[path]`, of het project opsplitsen in lib + bin.
<!-- SECTION:FINAL_SUMMARY:END -->
