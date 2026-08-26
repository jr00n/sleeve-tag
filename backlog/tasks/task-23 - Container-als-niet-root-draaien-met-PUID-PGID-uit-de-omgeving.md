---
id: TASK-23
title: Container als niet-root draaien met PUID/PGID uit de omgeving
status: To Do
assignee: []
created_date: '2026-08-26 22:26'
labels: []
milestone: m-5
dependencies:
  - TASK-2
  - TASK-5
  - TASK-12
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Op de UGREEN NAS moeten bestanden die de app schrijft dezelfde eigenaar en groep houden als de rest van de share, anders ziet Navidrome of de gebruiker ze niet meer goed. Op deze NAS is dat UID 1000 en GID 10.

Het proces mag niet als root draaien. `PUID` en `PGID` zijn via omgevingsvariabelen instelbaar en worden bij het starten toegepast, via een entrypoint of via `user:` in compose. Deze taak maakt af wat in fase 0 alleen als configuratiewaarde werd ingelezen.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Het applicatieproces in de container draait niet als root
- [ ] #2 PUID en PGID worden bij start toegepast, met 1000 en 10 als standaardwaarden
- [ ] #3 Bestanden die de app op een gemount volume schrijft krijgen de eigenaar en groep die met PUID/PGID zijn ingesteld
- [ ] #4 Bij ontbrekende schrijfrechten op MUSIC_ROOT geeft de app bij start een duidelijke melding in plaats van pas bij de eerste schrijfactie te falen
- [ ] #5 De werking is aantoonbaar getest op de UGREEN NAS met de echte share
- [ ] #6 De keuze tussen entrypoint en compose `user:` is met reden gedocumenteerd in de README
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
