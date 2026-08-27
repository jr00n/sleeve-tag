---
id: TASK-7
title: 'Tags lezen via het genormaliseerde tagmodel (tags::)'
status: In Progress
assignee:
  - claude
created_date: '2026-08-26 22:23'
updated_date: '2026-08-27 05:03'
labels: []
milestone: m-1
dependencies:
  - TASK-1
  - TASK-4
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De frontend werkt uitsluitend met een genormaliseerd tagmodel; de backend vertaalt van en naar het containerformaat. Deze taak levert de leeszijde van die vertaling voor MP3 (ID3v2) en FLAC (Vorbis comments).

De veldafbeelding staat in PRD.md §7: title/TIT2/TITLE, artist/TPE1/ARTIST, album_artist/TPE2/ALBUMARTIST, album/TALB/ALBUM, track+track_total/TRCK `n/total`/TRACKNUMBER+TRACKTOTAL, disc+disc_total/TPOS/DISCNUMBER+DISCTOTAL, year/TDRC/DATE, genre/TCON/GENRE, composer/TCOM/COMPOSER, comment/COMM/COMMENT, en art via APIC type 3 respectievelijk METADATA_BLOCK_PICTURE type 3.

Naast de gemodelleerde velden moeten de ruwe, aanwezige tags opvraagbaar zijn (nodig voor de geavanceerde weergave), evenals technische eigenschappen: duur en formaat. Multi-value velden worden in het MVP als één string behandeld. Alle `lofty`-aanroepen blijven binnen deze module.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een leesfunctie geeft voor een MP3 en voor een FLAC het volledige genormaliseerde tagmodel terug, inclusief duur en formaat
- [ ] #2 Ontbrekende velden komen terug als leeg/afwezig, niet als lege string die van een echt lege tag te onderscheiden is
- [ ] #3 Gecombineerde velden (TRCK/TPOS `n/total`) worden correct gesplitst naar nummer en totaal
- [ ] #4 Aanwezigheid, formaat, afmetingen en bytegrootte van embedded front cover art zijn opvraagbaar zonder de hele afbeelding te hoeven decoderen wanneer alleen de metadata nodig is
- [ ] #5 Er is een aparte functie die alle ruwe aanwezige tags (ID3-frames respectievelijk Vorbis-comments) als sleutel-waardelijst teruggeeft
- [ ] #6 Een bestand dat geen geldig MP3/FLAC blijkt geeft een duidelijke fout in plaats van een panic
- [ ] #7 Tests draaien tegen de fixtures uit tests/fixtures/ en dekken beide formaten, met en zonder tags en met embedded art
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
