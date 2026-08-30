---
id: TASK-38
title: 'De bestandslijst per schijf groeperen, met een kop en een knop per groep'
status: To Do
assignee: []
created_date: '2026-08-30 07:13'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) breekt de bestandslijst op in groepen per schijf: "Disc 1", "Disc 2", en apart "No disc number" voor wat er buiten valt. Elke kop noemt hoeveel bestanden de groep telt en hoeveel daarvan aandacht vragen, en heeft een knop om die hele schijf in één keer te selecteren.

Sleeve toont nu één doorlopende lijst. Bij een set van meerdere schijven is daardoor niet te zien waar de ene ophoudt en de volgende begint, en is een hele schijf selecteren handwerk. Juist bij die sets gaat het vaakst iets mis met de nummering.

Deze taak gaat over het tonen en selecteren. Het invullen van disc- en tracknummers hoort bij de hulpacties (task-34).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 De bestandslijst groepeert op discnummer, met per groep een kop en hoeveel bestanden erin zitten.
- [ ] #2 Bestanden zonder discnummer vormen een eigen groep die als laatste staat en als zodanig benoemd is.
- [ ] #3 De kop van een groep zegt hoeveel bestanden daarin aandacht vragen, of niets wanneer dat er geen zijn.
- [ ] #4 In de albumweergave is een hele groep met één klik te selecteren, zonder de rest van de selectie aan te tasten wanneer dat niet de bedoeling is.
- [ ] #5 Een map waarin geen enkel bestand een discnummer heeft, ziet er niet anders uit dan nu: één lijst zonder overbodige kop.
- [ ] #6 De volgorde binnen een groep blijft de sortering die er al was.
- [ ] #7 De groepering is met tests gedekt, inclusief een map met twee schijven, een map zonder discnummers, en een map waarin sommige bestanden er wel en andere er geen hebben.
- [ ] #8 README is bijgewerkt.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
