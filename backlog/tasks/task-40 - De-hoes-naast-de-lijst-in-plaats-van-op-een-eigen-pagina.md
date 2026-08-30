---
id: TASK-40
title: De hoes naast de lijst in plaats van op een eigen pagina
status: To Do
assignee: []
created_date: '2026-08-30 07:13'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) zet de hoes in een paneel naast de bestandslijst: de afbeelding, wat erover te zeggen valt ("JPEG · 1000×1000 · 284 KB", of "wisselt binnen de selectie"), een knop om hem te vervangen, een knop om hem in de selectie te zetten, en het vinkje om ook een cover.jpg in de map te schrijven.

Sleeve heeft dit allemaal, maar verspreid: de hoespagina hoort bij één bestand, en een hoes voor een selectie zit in de voorbeeldweergave van een batch. Terwijl je de tabel invult, is niet te zien welke hoes er in die bestanden zit.

De architectuurregel blijft staan: een hoes reist alleen mee in de laatste stap. Het paneel toont en kiest; wat ermee gebeurt, beslist de gebruiker in de voorbeeldweergave, en dat blijft de enige route naar het schrijven.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 De albumweergave toont de hoes van de selectie naast de lijst, met formaat, afmetingen en grootte erbij.
- [ ] #2 Loopt de selectie over bestanden met verschillende hoezen, of hebben sommige er geen, dan zegt het paneel dat in plaats van er één uit te kiezen en de rest te verzwijgen.
- [ ] #3 Een nieuwe hoes is vanuit het paneel te kiezen of erheen te slepen, en de knop noemt op hoeveel bestanden hij terechtkomt.
- [ ] #4 De keuze om ook een cover.jpg in de map te schrijven staat bij die actie, en gaat mee naar de stap die werkelijk schrijft.
- [ ] #5 Er wordt niets geschreven vanuit het paneel zelf: de afbeelding reist mee in de laatste stap, en de voorbeeldweergave blijft de enige route naar het schrijven.
- [ ] #6 De bestaande hoespagina per bestand blijft werken; deze taak voegt een weg toe en haalt er geen weg.
- [ ] #7 Op een smal scherm valt het paneel onder of boven de lijst en dringt het de tabel niet weg.
- [ ] #8 Het paneel is met tests gedekt, inclusief een selectie met verschillende hoezen en een selectie waarin niet elk bestand er een heeft.
- [ ] #9 README en, waar de regels veranderen, CLAUDE.md zijn bijgewerkt.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
