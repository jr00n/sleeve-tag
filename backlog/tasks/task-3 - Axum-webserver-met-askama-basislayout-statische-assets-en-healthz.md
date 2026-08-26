---
id: TASK-3
title: 'Axum-webserver met askama-basislayout, statische assets en /healthz'
status: To Do
assignee: []
created_date: '2026-08-26 22:22'
labels: []
milestone: m-0
dependencies: []
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De UI wordt serverside gerenderd met askama-templates plus HTMX; er is bewust geen node-toolchain en geen aparte frontend-build. HTMX wordt als lokaal meegeleverde JS-file geserveerd, zodat de app zonder internetverbinding werkt op de NAS.

Deze taak levert de webserver-basis waarop alle latere pagina's aansluiten: een basislayout met de naam "Sleeve" en favicon, serveren van statische bestanden, request-logging via tower-http, en het healthcheck-endpoint dat Docker gebruikt.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `cargo run` start een axum-server op de geconfigureerde poort en toont een pagina met de weergavenaam Sleeve
- [ ] #2 Er is een askama-basislayout waarin latere pagina's kunnen worden opgenomen, responsive en bruikbaar op een telefoonscherm
- [ ] #3 HTMX wordt vanaf een lokaal meegeleverd bestand geserveerd; de pagina laadt geen resources van externe hosts
- [ ] #4 `GET /healthz` geeft HTTP 200 met een korte statusbody
- [ ] #5 Requests worden gelogd naar stdout in leesbaar formaat op het geconfigureerde logniveau
- [ ] #6 Een integratietest controleert dat /healthz 200 geeft en dat de startpagina rendert
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
