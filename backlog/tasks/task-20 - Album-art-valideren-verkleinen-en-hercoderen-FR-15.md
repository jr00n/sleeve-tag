---
id: TASK-20
title: 'Album art valideren, verkleinen en hercoderen (FR-15)'
status: To Do
assignee: []
created_date: '2026-08-26 22:25'
labels: []
milestone: m-4
dependencies:
  - TASK-2
documentation:
  - PRD.md
priority: medium
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Geuploade hoezen zijn vaak veel groter dan nodig; een 3000x3000 scan in elk van de twaalf tracks blaast het album op en vertraagt elke speler. De app moet te grote art daarom optioneel verkleinen naar een configureerbare maximale resolutie (standaard 1000x1000 via `MAX_ART_SIZE`) met instelbare JPEG-kwaliteit.

Deze taak levert de beeldverwerking als losstaande, testbare laag: valideren dat een upload werkelijk een JPEG of PNG is, afmetingen bepalen, verkleinen met behoud van beeldverhouding, en opnieuw encoderen.

Aanname voor de open vraag uit PRD §12 (JPEG-only of PNG behouden): het bronformaat wordt behouden wanneer de afbeelding niet verkleind hoeft te worden; bij verkleinen wordt naar JPEG gecodeerd tenzij het origineel transparantie bevat. Bevestig dit met de eigenaar voordat de taak wordt afgerond.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een geuploade afbeelding wordt gevalideerd op werkelijk formaat (JPEG of PNG), niet alleen op bestandsnaam of content-type header
- [ ] #2 Afbeeldingen groter dan MAX_ART_SIZE worden verkleind met behoud van beeldverhouding; kleinere afbeeldingen blijven ongewijzigd
- [ ] #3 De JPEG-kwaliteit bij hercoderen is configureerbaar en heeft een gedocumenteerde standaardwaarde
- [ ] #4 Een bestand dat geen geldige afbeelding is wordt geweigerd met een begrijpelijke melding, zonder panic
- [ ] #5 Er is een bovengrens aan de geaccepteerde uploadgrootte zodat een enorme upload de NAS niet plat legt
- [ ] #6 Unit-tests dekken: te grote JPEG, te grote PNG, al kleine afbeelding, PNG met transparantie, en een ongeldig bestand
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
