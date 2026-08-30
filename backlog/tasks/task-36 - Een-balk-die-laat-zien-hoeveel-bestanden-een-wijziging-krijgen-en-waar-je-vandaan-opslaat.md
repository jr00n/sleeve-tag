---
id: TASK-36
title: >-
  Een balk die laat zien hoeveel bestanden een wijziging krijgen, en waar je
  vandaan opslaat
status: To Do
assignee: []
created_date: '2026-08-30 07:04'
updated_date: '2026-08-30 07:05'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) laat onderaan het scherm een balk staan zodra er iets openstaat: "3 files have staged changes", met daarnaast Verwerpen, Voorbeeld en Opslaan. De balk blijft in beeld terwijl je door de lijst scrollt.

In de albumweergave van Sleeve is nu niet te zien hoeveel bestanden er werkelijk iets krijgen. Je vult gedeelde velden en overrides in, en pas in de voorbeeldweergave blijkt hoeveel bestanden daadwerkelijk veranderen — soms nul, omdat er al stond wat je intikte. Die uitkomst hoort al zichtbaar te zijn terwijl je bezig bent, en de weg naar het voorbeeld en het opslaan hoort niet onderaan een lange tabel te liggen.

Wat de balk zegt, volgt uit wat er in de bestanden staat en uit wat er is ingevuld; het is dezelfde uitkomst die de voorbeeldweergave laat zien, alleen geteld. Er verandert niets aan wanneer er geschreven wordt: de voorbeeldweergave blijft de enige route daarheen, en de balk is een ingang, geen nieuwe route.

Buiten scope: wijzigingen bewaren die niet in het formulier staan, of ze laten overleven tussen mappen of sessies. De balk beschrijft wat er nú in het formulier staat.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 De albumweergave laat doorlopend zien hoeveel bestanden een wijziging zouden krijgen, en dat aantal klopt met wat de voorbeeldweergave daarna toont.
- [ ] #2 Staat er niets open, dan zegt de balk dat en zijn opslaan en voorbeeld niet aan te klikken.
- [ ] #3 Vanuit de balk zijn de ingevulde velden in één klik leeg te maken, en is de voorbeeldweergave in één klik te bereiken.
- [ ] #4 De balk blijft in beeld terwijl je door de tabel scrollt, en dekt op een telefoon niet de regel af waar je mee bezig bent.
- [ ] #5 Er wordt niets geschreven vanuit de balk zelf: de voorbeeldweergave blijft de enige stap die naar het schrijven leidt.
- [ ] #6 Zonder JavaScript blijft de albumweergave werken zoals ze deed; de telling komt dan mee met de pagina die de server teruggeeft.
- [ ] #7 De telling is met tests gedekt, inclusief het geval waarin een ingevulde waarde gelijk is aan wat er al in de bestanden staat en er dus niets verandert.
- [ ] #8 README en, waar de regels veranderen, CLAUDE.md zijn bijgewerkt.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
