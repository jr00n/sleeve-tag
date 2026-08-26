---
id: TASK-17
title: >-
  Hulpacties voor batch-bewerking: hernummeren, artiest kopieren, hoofdletters
  normaliseren (FR-10)
status: To Do
assignee: []
created_date: '2026-08-26 22:25'
labels: []
milestone: m-3
dependencies:
  - TASK-15
documentation:
  - PRD.md
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
- [ ] #1 Een actie nummert de geselecteerde bestanden opeenvolgend volgens de huidige sortering in de tabel
- [ ] #2 Een actie kopieert per bestand de artiest naar de albumartiest voor de hele selectie
- [ ] #3 Een actie normaliseert het hoofdlettergebruik van tekstvelden en toont het resultaat als voorstel voordat het wordt toegepast
- [ ] #4 Elke hulpactie vult uitsluitend de invoervelden; er wordt niets naar bestanden geschreven
- [ ] #5 Een hulpactie is ongedaan te maken door de invoer terug te zetten voordat er opgeslagen wordt
- [ ] #6 Unit-tests dekken de transformaties, inclusief randgevallen zoals namen met voorzetsels en bestaande afkortingen bij hoofdletternormalisatie
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
