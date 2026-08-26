---
id: TASK-4
title: Testfixtures voor MP3 en FLAC genereren en inchecken
status: To Do
assignee: []
created_date: '2026-08-26 22:22'
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
- [ ] #1 `tests/fixtures/` bevat MP3- en FLAC-bestanden in de varianten: geen tags, volledige tags, met embedded art, en een MP3 met ID3v1-tag
- [ ] #2 De gebruikte ffmpeg-commando's zijn vastgelegd in een script of README naast de fixtures
- [ ] #3 Er is een testhelper die een fixture naar een tempdir kopieert en het pad teruggeeft
- [ ] #4 Een test faalt zichtbaar wanneer een fixture ontbreekt, in plaats van stilzwijgend over te slaan
- [ ] #5 De totale omvang van de fixtures blijft klein genoeg om comfortabel in Git te leven (richtlijn: onder 1 MB)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
