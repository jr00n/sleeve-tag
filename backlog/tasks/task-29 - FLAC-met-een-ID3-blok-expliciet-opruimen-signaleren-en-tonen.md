---
id: TASK-29
title: 'FLAC met een ID3-blok: expliciet opruimen, signaleren en tonen'
status: To Do
assignee: []
created_date: '2026-08-28 20:08'
labels: []
milestone: m-5
dependencies: []
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
- [ ] #1 Bij het schrijven van een FLAC worden ID3v1- en ID3v2-blokken expliciet verwijderd door `tags::`, en niet als bijwerking van lofty
- [ ] #2 Er is een fixture van een FLAC met een ID3v2-blok, en een test die aantoont dat het bestand na een bewerking alleen Vorbis-comments overhoudt
- [ ] #3 Het opslagrapport meldt dat er een ID3-blok is aangetroffen en verwijderd
- [ ] #4 `checks::` signaleert een FLAC met een ID3-blok vóór het bewerken, zodat de gebruiker het weet zonder eerst iets te schrijven
- [ ] #5 De pagina met ruwe tags toont alle tags die in het bestand zitten, met per tag de soort, en niet alleen de primaire
- [ ] #6 De logruis is gedempt: het doorbladeren van een map met zulke bestanden levert geen regel per bestand meer op
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
