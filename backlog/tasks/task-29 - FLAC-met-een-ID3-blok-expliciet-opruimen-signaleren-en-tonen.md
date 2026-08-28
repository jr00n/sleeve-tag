---
id: TASK-29
title: 'FLAC met een ID3-blok: expliciet opruimen, signaleren en tonen'
status: Done
assignee:
  - claude
created_date: '2026-08-28 20:08'
updated_date: '2026-08-28 20:39'
labels: []
milestone: m-5
dependencies: []
modified_files:
  - src/tags/mod.rs
  - src/checks.rs
  - src/browse.rs
  - src/edit.rs
  - src/main.rs
  - src/web/mod.rs
  - src/batch.rs
  - src/testfixtures.rs
  - templates/rawtags.html
  - tests/id3inflac.rs
  - tests/fixtures/id3-in-flac.flac
  - tests/fixtures/zet-id3v2-voor-flac.py
  - tests/fixtures/genereer-fixtures.sh
  - README.md
  - CLAUDE.md
priority: medium
type: bug
ordinal: 24600
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Op de NAS bleek een grote groep FLAC-bestanden naast de Vorbis-comments ook een ID3v2-blok te dragen (afkomstig van de ripper). De FLAC-standaard kent dat niet, en lofty meldt bij elk inlezen `Encountered an ID3v2 tag. This tag cannot be rewritten to the FLAC file!` — bij het doorbladeren van één map leverde dat ongeveer negentig regels logruis op.

Drie problemen, in volgorde van belang:

1. **Het opruimen gebeurt bij toeval.** `tags::remove_stale_tags` doet alleen iets als de geschreven tag ID3v2 is (dus voor MP3, waar ID3v1 wordt verwijderd). Voor FLAC doet die functie niets. Dat het ID3-blok tóch verdween na een bewerking — geverifieerd op 2026-08-28: het bestand begon daarna met `fLaC` in plaats van `ID3` — komt doordat lofty's FLAC-writer het laat vallen. Dat is ongedocumenteerd gedrag van een dependency. Blijft het blok in een volgende versie wél staan, dan heeft het bestand twee tags die verschillende dingen zeggen, precies de tegenstrijdigheid die PRD §7 verbiedt voor ID3v1 naast ID3v2.
2. **Het gebeurt stilzwijgend.** Er wordt metadata uit het bestand verwijderd zonder dat het rapport dat meldt. Dat botst met "niets ongevraagd wijzigen" uit CLAUDE.md: ook het weghalen van iets wat er niet hoorde te zitten, hoort zichtbaar te zijn.
3. **Het is nergens te zien.** `tags::read_raw_tags` toont alleen de primaire tag, dus op `/tags/…` — juist de pagina die laat zien wat er écht in een bestand staat — is dat ID3-blok onzichtbaar. `checks::` kan het ook niet signaleren, want dat krijgt alleen het genormaliseerde model.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bij het schrijven van een FLAC worden ID3v1- en ID3v2-blokken expliciet verwijderd door `tags::`, en niet als bijwerking van lofty
- [x] #2 Er is een fixture van een FLAC met een ID3v2-blok, en een test die aantoont dat het bestand na een bewerking alleen Vorbis-comments overhoudt
- [x] #3 Het opslagrapport meldt dat er een ID3-blok is aangetroffen en verwijderd
- [x] #4 `checks::` signaleert een FLAC met een ID3-blok vóór het bewerken, zodat de gebruiker het weet zonder eerst iets te schrijven
- [x] #5 De pagina met ruwe tags toont alle tags die in het bestand zitten, met per tag de soort, en niet alleen de primaire
- [x] #6 De logruis is gedempt: het doorbladeren van een map met zulke bestanden levert geen regel per bestand meer op
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
## Vastgesteld gedrag van lofty (geprobeerd, niet aangenomen)

Voordat er iets gebouwd werd: een fixture gemaakt en gemeten wat lofty 0.25.1
werkelijk doet met een FLAC die een ID3v2-blok draagt.

- `file.tags()` levert **beide** blokken; `primary_tag()` is de Vorbis-comments.
  De informatie is er dus al — ze werd alleen weggegooid.
- `TagType::Id3v2.remove_from` op een FLAC werkt en levert `Ok(())`; het bestand
  begint daarna met `fLaC`.
- `TagType::Id3v1.remove_from` op een FLAC **faalt** en brak elke schrijfactie
  af. Vandaar dat er alleen verwijderd wordt wat er ook werkelijk in zit.

## Aanpak

1. `tags::foreign_tag_types` bepaalt welke blokken niet bij het formaat horen
   (MP3: alles behalve ID3v2/ID3v1; FLAC: alles behalve Vorbis-comments). Dat is
   een constatering, geen oordeel: `Track::foreign_tags` draagt hem naar buiten.
2. `remove_stale_tags` ruimt bij een FLAC het ID3-blok op, zoals het bij een MP3
   al de ID3v1-tag deed — maar alleen wat aanwezig is, en alleen op het pad dat
   het bestand tóch herschrijft.
3. `write` en `write_art` geven een `Written` terug in plaats van `()`/`bool`:
   of er iets veranderd is, en wat er is opgeruimd. De aanroepers zetten dat in
   de melding (bewerkformulier) en in het rapport per bestand (batch, hoes).
4. `read_raw_tags` levert `Vec<RawBlock>` in plaats van één blok, met per blok de
   soort, of het primair is, en of het er thuishoort. Het template toont één
   tabel per blok, met een waarschuwing bij het blok dat er niet hoort.
5. `checks::TrackIssue::ForeignTagBlock`, gevoed door `Entry::foreign_tags`.
6. Logruis: `main::log_filter` zet de tagbibliotheek standaard op `error`. De
   naam van die crate blijft in `tags::LOG_TARGET` — `tests/architecture.rs`
   verbiedt terecht dat de rest van de app hem noemt.

## Fixture

ffmpeg kan dit niet maken: de flac-muxer kent geen `-write_id3v2`. Daarom
`tests/fixtures/zet-id3v2-voor-flac.py`, dat een ID3v2.4-blok met een TIT2- en
TPE1-frame vóór het bestand plakt — dezelfde aanpak als het bestaande
`voeg-comm-frame-toe.py`. De titel in dat blok wijkt af van die in de
Vorbis-comments, zodat een test kan aantonen wélke gelezen wordt.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Het vermoeden uit de taakomschrijving klopte, maar niet helemaal: lofty gooit het ID3-blok inderdaad weg bij het herschrijven van een FLAC — en `TagType::Id3v1.remove_from` op een FLAC fáált, waardoor een blinde 'verwijder alles wat er niet hoort' elke schrijfactie op een FLAC brak. Vandaar dat `remove_stale_tags` eerst kijkt wát er in zit en alleen dát verwijdert.

Bewust niet opgeruimd bij een schrijfactie die niets verandert. Dat is geen halve maatregel maar dezelfde regel als elders: een bestand van 3,3 GB herschrijven om iets weg te halen wat de gebruiker niet heeft aangeraakt, verspringt de wijzigingsdatum en zet Navidrome aan het scannen, zonder dat er iets te zien valt. Tot de eerste echte bewerking meldt `checks::` het.

De architectuurtest hielp: het logfilter noemde eerst `lofty` rechtstreeks in main.rs, en `tests/architecture.rs` sloeg daarop aan. Terecht — de naam van die crate hoort binnen `tags::` te blijven. Nu staat hij in `tags::LOG_TARGET` en weet main alleen dát er een tagbibliotheek is die te luid is.

`Notice::Saved` draagt nu meerdere regels in plaats van één. De melding over een verwijderd blok is een tweede alinea boven het formulier, en geen langere eerste zin.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Een FLAC met een ID3-blok wordt nu herkend, getoond, gemeld en opgeruimd — en dat opruimen is een keuze van `tags::` geworden in plaats van een bijwerking van lofty.

**Eerst gemeten, toen gebouwd.** Met een nieuwe fixture is vastgesteld wat lofty 0.25.1 werkelijk doet: beide tagblokken zijn via `file.tags()` gewoon zichtbaar (die informatie werd alleen weggegooid), `Id3v2::remove_from` werkt op een FLAC, en `Id3v1::remove_from` faalt daar — wat een naïeve implementatie zou hebben laten omvallen op elke FLAC.

**Opruimen (AC #1, #2).** `remove_stale_tags` haalt bij een FLAC het ID3-blok weg zoals het bij een MP3 al de ID3v1-tag deed, maar alleen wat er werkelijk in zit. Alleen op het pad dat het bestand tóch herschrijft: verandert er niets aan de tags, dan blijft het bestand onaangeraakt, mét blok. Elke schrijfroute komt langs dezelfde opruiming — ook het embedden van een hoes — want anders zou het van de toevallig gekozen actie afhangen wat een bestand overhoudt.

**Melden (AC #3).** `write` en `write_art` geven een `Written` terug: of er iets veranderd is, en wat er is verdwenen. Boven het bewerkformulier komt dat als tweede alinea, in een batch als extra regel bij het bestand. Stilzwijgend metadata uit een bestand halen is een ongevraagde wijziging, ook als het iets is wat er nooit had moeten staan.

**Signaleren (AC #4).** `checks::TrackIssue::ForeignTagBlock` markeert zo'n bestand in de maplijst met "tagblok dat er niet hoort" — vóór het bewerken, zonder dat er iets geschreven hoeft te worden. `checks::` blijft daarbij wat het was: het krijgt de constatering van `tags::` binnen en opent zelf geen bestand.

**Tonen (AC #5).** De pagina met ruwe tags toont nu élk blok in een eigen tabel, met de soort erboven, welk blok gelezen en bijgewerkt wordt, en een waarschuwing bij het blok dat er niet in thuishoort. Juist die pagina hoort te laten zien wat er wérkelijk in staat.

**Logruis (AC #6).** De tagbibliotheek staat standaard op `error`: één map met negentig van die bestanden leverde negentig identieke regels op. Haar naam blijft in `tags::LOG_TARGET`, want `tests/architecture.rs` verbiedt terecht dat de rest van de app die crate noemt. Wie de meldingen wil zien: `LOG_LEVEL=info,lofty=warn`.

**Getest** met vijf unit-tests in `tags::`, één in `checks::` en zeven integratietests in `tests/id3inflac.rs` die de hele keten aflopen: signalering in de lijst, beide blokken op de tagpagina, de melding bij het opslaan, het blok dat blijft staan als er niets verandert, en de logruis in beide richtingen.
<!-- SECTION:FINAL_SUMMARY:END -->
