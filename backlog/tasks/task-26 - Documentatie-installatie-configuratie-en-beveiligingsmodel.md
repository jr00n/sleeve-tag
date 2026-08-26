---
id: TASK-26
title: 'Documentatie: installatie, configuratie en beveiligingsmodel'
status: To Do
assignee: []
created_date: '2026-08-26 22:26'
updated_date: '2026-08-26 22:29'
labels: []
milestone: m-5
dependencies:
  - TASK-24
documentation:
  - PRD.md
priority: medium
type: docs
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De app biedt in het MVP bewust geen authenticatie; toegang wordt op netwerkniveau afgeschermd (alleen bereikbaar binnen het LAN en via Tailscale). Dat is een verdedigbare keuze, maar alleen als hij expliciet gedocumenteerd is, inclusief de waarschuwing dat de app nooit rechtstreeks vanaf internet ontsloten mag worden.

Verder hoort in de documentatie: wat Sleeve is en niet is (geen speler, geen bibliotheekbeheer, geen koppeling met een mediaserver), installatie op de NAS, alle omgevingsvariabelen met hun standaardwaarden, de ontwikkelworkflow op de Mac, en hoe Navidrome de wijzigingen zelf oppikt via zijn periodieke scan.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 De README beschrijft wat Sleeve doet, de ondersteunde formaten (MP3, FLAC) en de expliciete non-goals uit het PRD
- [ ] #2 Alle omgevingsvariabelen staan gedocumenteerd met betekenis en standaardwaarde
- [ ] #3 Installatie op de UGREEN NAS staat stap voor stap beschreven, inclusief volumes en PUID/PGID
- [ ] #4 Het beveiligingsmodel is expliciet beschreven: geen authenticatie, uitsluitend bereikbaar via LAN en Tailscale, niet vanaf internet ontsluiten
- [ ] #5 De ontwikkelworkflow op macOS (cargo run met een lokale MUSIC_ROOT, cargo watch of bacon, kwaliteitspoort) staat beschreven
- [ ] #6 Er staat vermeld dat Navidrome wijzigingen zelf oppikt bij zijn periodieke scan en dat Sleeve daar niets voor doet
- [ ] #7 De README legt uit dat MUSIC_ROOT in de container altijd /music is en dat het host-pad van de share alleen via MUSIC_HOST_PATH in .env wordt opgegeven
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
