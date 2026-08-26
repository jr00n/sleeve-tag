---
id: TASK-6
title: Veilige padafhandeling binnen MUSIC_ROOT
status: To Do
assignee: []
created_date: '2026-08-26 22:23'
labels: []
milestone: m-1
dependencies:
  - TASK-2
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De app krijgt paden binnen via URL's en formulieren en schrijft in de echte muziekbibliotheek. Zonder een strikte, centrale padcontrole is path traversal mogelijk en kan de app buiten de share schrijven. Alle latere fasen (browsen, bewerken, art) moeten via deze module lopen.

Regels uit het PRD: elk binnenkomend pad wordt gecanonicaliseerd (`std::fs::canonicalize`) en gecontroleerd tegen `MUSIC_ROOT`; symlinks die buiten de root wijzen worden geweigerd; navigatie boven de root is onmogelijk. Alleen bestanden met extensie `.mp3` of `.flac` en een herkend containerformaat gelden als bewerkbaar.

Deze module is de enige plek waar een door de gebruiker aangeleverd pad naar een filesystem-pad wordt omgezet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een functie zet een door de gebruiker aangeleverd relatief pad om naar een gevalideerd absoluut pad binnen MUSIC_ROOT, of geeft een fout
- [ ] #2 Pogingen met `..`, absolute paden, of een symlink die buiten de root wijst worden geweigerd
- [ ] #3 Er is een aparte controle die bepaalt of een bestand bewerkbaar is (extensie .mp3/.flac én herkend containerformaat)
- [ ] #4 Foutgevallen leveren een fouttype op dat de webserver kan vertalen naar HTTP 400/403/404 zonder het absolute pad te lekken
- [ ] #5 Unit-tests dekken: geldig pad, traversal via `..`, absoluut pad, symlink binnen de root (toegestaan), symlink buiten de root (geweigerd), en een bestand met verkeerde extensie
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
