---
id: TASK-11
title: 'Geavanceerde weergave met alle ruwe tags, alleen-lezen (FR-7)'
status: To Do
assignee: []
created_date: '2026-08-26 22:23'
labels: []
milestone: m-1
dependencies:
  - TASK-7
  - TASK-8
documentation:
  - PRD.md
priority: low
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Naast het genormaliseerde model wil de beheerder kunnen zien wat er werkelijk in een bestand staat: alle aanwezige ID3-frames of Vorbis-comments, inclusief velden die de app niet modelleert. Dit is diagnostisch en helpt te begrijpen waarom een mediaserver iets anders toont dan verwacht.

In het MVP is deze weergave uitdrukkelijk alleen-lezen; bewerken van ruwe frames is geen doel.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Per bestand is een 'geavanceerd'-weergave op te vragen die alle aanwezige ruwe tags als sleutel-waardelijst toont
- [ ] #2 Voor MP3 worden ID3-frames getoond, voor FLAC Vorbis-comments, telkens met de originele sleutelnaam
- [ ] #3 Binaire velden zoals embedded art worden samengevat (type en grootte) in plaats van als ruwe data getoond
- [ ] #4 De weergave biedt geen enkele manier om ruwe tags te wijzigen
- [ ] #5 Een integratietest controleert de weergave voor een MP3- en een FLAC-fixture met volledige tags
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
