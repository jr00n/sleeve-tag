---
id: TASK-24
title: 'docker-compose.yml voor UGOS met volumes, env en healthcheck'
status: To Do
assignee: []
created_date: '2026-08-26 22:26'
updated_date: '2026-08-26 22:29'
labels: []
milestone: m-5
dependencies:
  - TASK-23
documentation:
  - PRD.md
priority: high
type: chore
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De gebruiker moet de app op de NAS kunnen starten met `docker compose up -d` zonder handmatige tussenstappen. Daarvoor hoort een meegeleverde `docker-compose.yml` bij het project, met commentaar gericht op UGOS.

Inhoud: de muziekshare gemount op `/music` (read-write), optioneel `/config` voor instellingen en logs, alle omgevingsvariabelen uit §8.3 met de NAS-standaarden (PUID=1000, PGID=10), een poortmapping en een healthcheck op `/healthz`.

Vastgelegde keuze over het sharepad (was open punt in PRD §12): de app kent het host-pad nooit. In de container is `MUSIC_ROOT` altijd `/music`; het echte pad op de UGREEN is uitsluitend de linkerkant van de volume-mount en komt uit een `.env` naast de compose-file (`MUSIC_HOST_PATH`). Zo blijft het NAS-specifieke pad buiten Git en kan dezelfde compose-file lokaal met een testmap draaien. Een ingecheckte `.env.example` documenteert de variabele.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Het project bevat een docker-compose.yml waarmee de container op de NAS met `docker compose up -d` start
- [ ] #2 De muziekshare wordt read-write gemount als `${MUSIC_HOST_PATH}:/music` en de container draait met MUSIC_ROOT=/music
- [ ] #3 Er is een ingecheckte .env.example met MUSIC_HOST_PATH en toelichting; de echte .env staat in .gitignore
- [ ] #4 Alle omgevingsvariabelen uit het PRD staan in het bestand met de NAS-standaardwaarden, inclusief PUID=1000 en PGID=10
- [ ] #5 Er is een healthcheck gedefinieerd die /healthz gebruikt, zodat Docker een vastgelopen container herstart
- [ ] #6 Het bestand bevat commentaar dat uitlegt wat een UGOS-gebruiker moet aanpassen
- [ ] #7 De UI is na het starten bereikbaar via http://<nas>:<port> en via Tailscale
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
