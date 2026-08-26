---
id: TASK-2
title: Configuratie inlezen uit omgevingsvariabelen
status: To Do
assignee: []
created_date: '2026-08-26 22:22'
updated_date: '2026-08-26 22:29'
labels: []
milestone: m-0
dependencies: []
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De app wordt volledig via omgevingsvariabelen geconfigureerd, omdat hij als container draait zonder configuratiebestand. Zonder deze laag kan geen enkele latere fase weten waar de muziek staat of hoe groot album art mag worden.

Te ondersteunen variabelen met hun betekenis: `MUSIC_ROOT` (pad naar de gemounte muziekshare, verplicht), `PORT` (HTTP-poort), `PUID`/`PGID` (eigenaar van weggeschreven bestanden op de NAS; standaard 1000 en 10), `MAX_ART_SIZE` (maximale resolutie van embedded art, standaard 1000x1000), `LOG_LEVEL` en `BACKUP_ON_WRITE` (standaard uit).

De feitelijke toepassing van PUID/PGID gebeurt in fase 5; deze taak zorgt alleen dat de waarden gelezen en gevalideerd worden.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Alle genoemde omgevingsvariabelen worden ingelezen in een getypeerde configuratiestruct met de in het PRD genoemde standaardwaarden
- [ ] #2 Een ontbrekende of niet-bestaande `MUSIC_ROOT` laat de app met een duidelijke foutmelding stoppen in plaats van te starten
- [ ] #3 Ongeldige waarden (bijv. niet-numerieke PORT) geven een begrijpelijke foutmelding die de naam van de variabele noemt
- [ ] #4 De effectieve configuratie wordt bij start gelogd
- [ ] #5 Unit-tests dekken standaardwaarden, geldige overrides en foutgevallen
- [ ] #6 Tests zetten MUSIC_ROOT altijd op een tempdir met gekopieerde fixtures, zodat een test de echte bibliotheek per constructie niet kan raken
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
