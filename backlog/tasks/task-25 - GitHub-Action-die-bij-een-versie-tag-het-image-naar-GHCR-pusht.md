---
id: TASK-25
title: GitHub Action die bij een versie-tag het image naar GHCR pusht
status: To Do
assignee: []
created_date: '2026-08-26 22:26'
labels: []
milestone: m-5
dependencies:
  - TASK-5
  - TASK-24
documentation:
  - PRD.md
priority: medium
type: chore
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Handmatig `docker save | ssh nas docker load` werkt, maar is bewerkelijk bij elke update. Vanaf fase 5 haalt de NAS het image op met `docker compose pull`, wat vraagt om een gepubliceerd image.

Een GitHub Action bouwt bij een versie-tag het linux/amd64-image en pusht het naar GHCR. De compose-file verwijst daarna naar het gepubliceerde image in plaats van naar een lokaal geladen image.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Een push van een versie-tag start een workflow die het linux/amd64-image bouwt en naar GHCR pusht
- [ ] #2 Het image is getagd met zowel de versie als een verplaatsbare tag voor de laatste release
- [ ] #3 De workflow draait de kwaliteitspoort (fmt, clippy, test) en publiceert niet wanneer die faalt
- [ ] #4 docker-compose.yml verwijst naar het GHCR-image zodat `docker compose pull` op de NAS werkt
- [ ] #5 Het releaseproces (taggen, pullen op de NAS) staat beschreven in de README
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
