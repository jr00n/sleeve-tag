---
id: TASK-13
title: 'Tags wegschrijven via tags:: met behoud van niet-gemodelleerde velden'
status: To Do
assignee: []
created_date: '2026-08-26 22:24'
labels: []
milestone: m-2
dependencies:
  - TASK-7
  - TASK-12
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De schrijfzijde van de vertaling uit PRD §7. De regels zijn strikt, omdat de bibliotheek door Navidrome gelezen wordt en niet beschadigd mag raken:

- MP3 wordt altijd weggeschreven als ID3v2.4 met UTF-8. Bestaande ID3v1-tags worden verwijderd of gesynchroniseerd, nooit inconsistent achtergelaten.
- Tags die de app niet modelleert blijven ongewijzigd bewaard; alleen velden die de gebruiker daadwerkelijk aanraakt worden overschreven.
- Een leeg gemaakt veld betekent 'veld verwijderen', niet 'lege string opslaan'.
- Multi-value velden worden in het MVP als één string behandeld.

Het feitelijke wegschrijven loopt via de atomische schrijfhelper uit die taak, zodat een half geschreven bestand onmogelijk is.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een schrijffunctie neemt het genormaliseerde tagmodel aan en schrijft de velden correct weg voor zowel MP3 als FLAC
- [ ] #2 MP3-bestanden zijn na het schrijven ID3v2.4 met UTF-8, ook wanneer ze daarvoor een oudere versie hadden
- [ ] #3 Een bestaande ID3v1-tag is na het schrijven verwijderd of in lijn met de ID3v2-tag, nooit afwijkend
- [ ] #4 Tags die niet in het model voorkomen zijn na het schrijven onveranderd aanwezig
- [ ] #5 Een veld dat leeg is gemaakt is na het schrijven verwijderd uit het bestand in plaats van als lege waarde aanwezig
- [ ] #6 Gecombineerde velden worden correct weggeschreven (TRCK/TPOS als `n/total`, TRACKNUMBER/TRACKTOTAL apart)
- [ ] #7 De audio-inhoud van het bestand is na een tagwijziging bit-identiek aan die van daarvoor
- [ ] #8 Tests schrijven naar kopieën van de fixtures en lezen het resultaat terug; een test verifieert het resultaat ook met een onafhankelijke tool (bijv. ffprobe) wanneer die beschikbaar is
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
