---
id: TASK-5
title: Multi-stage Dockerfile met statische linux/amd64-build
status: To Do
assignee: []
created_date: '2026-08-26 22:22'
labels: []
milestone: m-0
dependencies: []
documentation:
  - PRD.md
priority: medium
type: chore
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De app draait op een UGREEN NAS (UGOS, Linux) terwijl er op een Apple Silicon MacBook ontwikkeld wordt. Er is dus een cross-build nodig naar `linux/amd64` (te bevestigen met `uname -m` op de NAS) die een statisch gelinkte binary oplevert in een minimale runtime-image.

Build-stage op `rust:<stable>-slim`, runtime-stage op distroless/static of alpine met target `x86_64-unknown-linux-musl`. Bouwen gebeurt vanaf de Mac met `docker buildx build --platform linux/amd64`; `cargo zigbuild` is het alternatief als musl-cross lastig blijkt.

De harde eis uit het PRD is een image onder 30 MB. Distributie naar de NAS gebeurt in deze fase nog handmatig via `docker save | ssh nas docker load`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `docker buildx build --platform linux/amd64` levert een image dat op de NAS start en /healthz beantwoordt
- [ ] #2 De resulterende image is kleiner dan 30 MB
- [ ] #3 De binary is statisch gelinkt en draait in een runtime-image zonder Rust-toolchain
- [ ] #4 Het bouwcommando en de handmatige distributiestap (`docker save | ssh nas docker load`) staan in de README
- [ ] #5 Docker-layers zijn zo geordend dat een wijziging in de broncode geen volledige herbouw van dependencies veroorzaakt
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
