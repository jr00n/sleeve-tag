---
id: TASK-12
title: Atomisch schrijven met hervalidatie en optionele backup
status: To Do
assignee: []
created_date: '2026-08-26 22:24'
labels: []
milestone: m-2
dependencies:
  - TASK-6
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Harde eis uit het PRD: nooit dataverlies. De app schrijft in een bibliotheek die niet opnieuw op te bouwen is, en de container kan tijdens een schrijfactie worden afgebroken. Deze taak levert de schrijfstrategie die alle latere schrijfacties (tags en album art) gebruiken.

Werkwijze uit §8.4: schrijf naar een tijdelijk bestand in dezelfde map, valideer door het opnieuw in te lezen, en hernoem het pas daarna over het origineel. Bij elke fout blijft het origineel onaangetast. Bij `BACKUP_ON_WRITE=true` komt er een `.bak` naast het bestand te staan; standaard staat dit uit om de share niet te vervuilen.

Bijkomende eis uit acceptatiecriterium 2 van het MVP: eigenaar, groep en permissies van het originele bestand blijven na het schrijven ongewijzigd. Elke schrijfactie wordt gelogd met pad en gewijzigde velden.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een schrijfhelper vervangt de inhoud van een bestand alleen na een geslaagde hervalidatie van het tijdelijke bestand
- [ ] #2 Bij een fout tijdens schrijven of validatie blijft het origineel byte-voor-byte ongewijzigd en blijft er geen tijdelijk bestand achter
- [ ] #3 Het tijdelijke bestand staat in dezelfde map als het origineel, zodat het hernoemen atomisch is
- [ ] #4 Eigenaar, groep en permissies van het originele bestand blijven na een schrijfactie ongewijzigd
- [ ] #5 Met BACKUP_ON_WRITE=true wordt een .bak naast het bestand geplaatst; met de standaardwaarde niet
- [ ] #6 Elke geslaagde schrijfactie wordt gelogd met pad en de gewijzigde velden
- [ ] #7 Een test simuleert een mislukking halverwege het schrijven en toont aan dat het origineel intact is
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
