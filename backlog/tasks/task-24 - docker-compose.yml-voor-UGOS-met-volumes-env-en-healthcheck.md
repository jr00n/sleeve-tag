---
id: TASK-24
title: 'docker-compose.yml voor UGOS met volumes, env en healthcheck'
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:26'
updated_date: '2026-08-28 19:38'
labels: []
milestone: m-5
dependencies:
  - TASK-23
documentation:
  - PRD.md
modified_files:
  - docker-compose.yml
  - .env.example
  - src/health.rs
  - src/main.rs
  - src/config.rs
  - tests/health.rs
  - README.md
  - CLAUDE.md
priority: high
type: chore
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De gebruiker moet de app op de NAS kunnen starten met `docker compose up -d` zonder handmatige tussenstappen. Daarvoor hoort een meegeleverde `docker-compose.yml` bij het project, met commentaar gericht op UGOS.

Inhoud: de muziekshare gemount op `/music` (read-write), optioneel `/config` voor instellingen en logs, alle omgevingsvariabelen uit §8.3 met de NAS-standaarden (PUID=1000, PGID=10), een poortmapping en een healthcheck op `/healthz`.

Vastgelegde keuze over het sharepad: de app kent het host-pad nooit. In de container is `MUSIC_ROOT` altijd `/music`; het echte pad op de UGREEN is uitsluitend de linkerkant van de volume-mount en komt uit een `.env` naast de compose-file (`MUSIC_HOST_PATH`). Zo blijft het NAS-specifieke pad buiten Git en kan dezelfde compose-file lokaal met een testmap draaien. Een ingecheckte `.env.example` documenteert de variabele.

Het pad is inmiddels bekend en tijdens de Dockerfile-taak in de praktijk gebruikt: **`/volume1/Multimedia/music`** op `wolffpacksrv.local`. De app draaide daar met `docker run -d -p 8080:8080 -v /volume1/Multimedia/music:/music` en beantwoordde `/healthz`. Dit was het laatste openstaande punt uit PRD §12.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Het project bevat een docker-compose.yml waarmee de container op de NAS met `docker compose up -d` start
- [x] #2 De muziekshare wordt read-write gemount als `${MUSIC_HOST_PATH}:/music` en de container draait met MUSIC_ROOT=/music
- [x] #3 Er is een ingecheckte .env.example met MUSIC_HOST_PATH en toelichting; de echte .env staat in .gitignore
- [x] #4 Alle omgevingsvariabelen uit het PRD staan in het bestand met de NAS-standaardwaarden, inclusief PUID=1000 en PGID=10
- [x] #5 Er is een healthcheck gedefinieerd die /healthz gebruikt, zodat Docker een vastgelopen container herstart
- [x] #6 Het bestand bevat commentaar dat uitlegt wat een UGOS-gebruiker moet aanpassen
- [x] #7 De UI is na het starten bereikbaar via http://<nas>:<port> en via Tailscale
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 cargo fmt --check slaagt
- [x] #2 cargo clippy -- -D warnings slaagt
- [x] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [x] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [x] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Aanpak

**De healthcheck vraagt om code, niet alleen om YAML.**
De runtime is distroless: geen shell, geen `curl`, geen `wget`. Een
`healthcheck:` in compose kan dus alleen een binary aanroepen die al in het
image zit — en dat is er precies één. De binary krijgt daarom een tweede
bedrijfsmodus: `sleeve-tag --health` doet één HTTP-verzoek naar
`127.0.0.1:$PORT/healthz` en eindigt met 0 of 1. Dat is het gangbare patroon
voor distroless-images, kost geen extra dependency (een `GET` over een
`TcpStream` is een handvol regels, net als in de integratietests) en houdt de
healthcheck bij de app in plaats van bij de compose-file.

De vlag wordt vóór clap afgehandeld: de probe heeft alleen `PORT` nodig en mag
niet struikelen over een `MUSIC_ROOT` die op dat moment niet gezet is.

## Stappen

1. `src/health.rs` (nieuw): `probe(port)` opent een TCP-verbinding naar de
   loopback, stuurt `GET /healthz`, en kijkt of het antwoord met `200` begint.
   Korte timeouts — een healthcheck die blijft hangen is geen healthcheck.
2. `src/config.rs`: `port_from_env()` zodat de probe dezelfde parser en dezelfde
   standaardwaarde gebruikt als de server. Eén bron voor `PORT`.
3. `src/main.rs`: `--health` afvangen vóór `Config::parse()` en met de
   exitcode van de probe eindigen.
4. `docker-compose.yml`: `${MUSIC_HOST_PATH}:/music` read-write,
   `user: "${PUID:-1000}:${PGID:-10}"`, alle env-variabelen uit PRD §8.3 met de
   NAS-standaarden, poortmapping, `healthcheck` op de binary, `restart:
   unless-stopped`. Commentaar gericht op wat een UGOS-gebruiker aanpast.
5. `.env.example`: `MUSIC_HOST_PATH=/volume1/Multimedia/music` plus toelichting.
   `.env` staat al in `.gitignore`.
6. `tests/health.rs`: `--health` tegen een draaiende server geeft exitcode 0,
   tegen een vrije poort een exitcode ongelijk aan 0.
7. README: sectie over draaien met compose, en de healthcheck-modus benoemen.

## Niet in deze taak

- AC #7 (bereikbaar op de NAS en via Tailscale) vergt de NAS zelf; dat doet de
  gebruiker met het image dat nu gebouwd wordt.
- Een `/config`-volume: het PRD noemt het optioneel en de app schrijft nergens
  buiten de bibliotheek. Een leeg volume aanbieden dat niets doet, is
  misleidender dan het weglaten; wel als commentaar benoemd.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
De healthcheck bleek geen YAML-kwestie maar een codekwestie: distroless heeft geen shell, geen curl en geen wget, dus `healthcheck:` kan alleen de enige binary in het image aanroepen. Vandaar `sleeve-tag --health` als tweede bedrijfsmodus — één GET naar 127.0.0.1:$PORT/healthz, alleen een exitcode terug, en geen extra dependency (het verzoek past in een handvol regels over een TcpStream).

De modus draait bewust vóór `Config::parse` en leest alleen `PORT` via `config::port_from_env`. Zou hij de volledige configuratie inlezen, dan zou een verkeerd gezette MUSIC_ROOT de container ook ongezond laten *lijken* — om de verkeerde reden.

Geen /config-volume opgenomen, wel als commentaar benoemd: Sleeve houdt geen state bij en logt naar stdout. Een leeg volume aanbieden dat niets doet, is misleidender dan het weglaten.

Lokaal geverifieerd met het amd64-image onder podman: `podman compose up -d` start de container, `docker inspect` meldt na de start_period `healthy`, de UI antwoordt met 200, en `podman exec sleeve-tag /usr/local/bin/sleeve-tag --health` geeft exitcode 0. De startcontrole uit TASK-23 logde daarbij `MUSIC_ROOT is schrijfbaar uid=1000 gid=10`.

AC #7 (bereikbaar op de NAS en via Tailscale) staat nog open: dat vraagt de UGREEN zelf. Het image is gebouwd en klaar om over te zetten.

Op de NAS geverifieerd (2026-08-28), draaiend vanuit /volume2/Docker/sleeve-tag met het overgezette amd64-image:

- `docker compose ps` toont `Up 4 minutes (healthy)` — Docker heeft daar dus `sleeve-tag --health` aangeroepen en exitcode 0 gekregen. De healthcheck-modus werkt in de echte container, niet alleen lokaal.
- Vanaf het LAN antwoordt `http://wolffpacksrv.local:8080/` met 200 in 17 ms, `/healthz` met `ok`, en de startpagina toont de werkelijke mappen van de share (Live Sets & Festivals, Own Recordings, Singles & EPs, Studio Albums).

De Tailscale-helft van AC #7 kan ik niet vaststellen: op de ontwikkelmachine staat geen Tailscale-client. Dat deel wacht op een controle door de eigenaar vanaf een apparaat op het tailnet.

Onderweg twee dingen die in het geheugen zijn vastgelegd: `scp` naar deze NAS vereist `-O` (het SFTP-subsysteem is gechroot, waardoor een absoluut pad faalt met `remote mkdir: No such file or directory`), en de share heet `/volume1/Multimedia/Music` met hoofdletter — `.env.example` is daarop gecorrigeerd.

AC #7 volledig: de eigenaar bevestigde op 2026-08-28 dat de UI ook via Tailscale bereikbaar is, naast de LAN-controle hierboven. Daarmee is de opstelling uit PRD §10.1 aangetoond: `docker compose up -d` op de UGREEN, UI bereikbaar op http://<nas>:8080 én via het tailnet.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
`docker compose up -d` start Sleeve op de NAS, met alleen `MUSIC_HOST_PATH` in een `.env` als NAS-specifiek gegeven.

**`docker-compose.yml`** — `${MUSIC_HOST_PATH}:/music` read-write, `user: "${PUID:-1000}:${PGID:-10}"`, alle omgevingsvariabelen uit PRD §8.3 met de NAS-standaarden, poortmapping via `${HOST_PORT:-8080}`, `restart: unless-stopped` en een healthcheck. Het commentaar is op een UGOS-gebruiker gericht: wat hij moet aanpassen, hoe hij de juiste uid/gid vindt (`ls -n`), en waarom `MUSIC_ROOT` in de container altijd `/music` blijft. Een ontbrekende `MUSIC_HOST_PATH` geeft via `${VAR:?…}` meteen een leesbare fout in plaats van een mount van niets.

**`.env.example`** — met het pad van deze NAS als voorbeeld en toelichting per waarde; `.env` stond al in `.gitignore`.

**Nieuw: `health::`.** De healthcheck kon geen `curl` gebruiken — distroless heeft geen shell. `sleeve-tag --health` doet daarom zelf één verzoek aan `127.0.0.1:$PORT/healthz` en eindigt met exitcode 0 of 1. De modus draait vóór `Config::parse` en kent alleen `PORT`: een healthcheck die over een andere instelling struikelt, meet iets anders dan hij beweert. Geen extra dependency.

**Gedekt door tests:** `tests/health.rs` draait de binary echt met `--health` — tegen een draaiende server (0), tegen een vrije poort (≠0), en zonder `MUSIC_ROOT` in de omgeving. Plus een unit-test op een gesloten poort.

**Lokaal geverifieerd** met het amd64-image onder podman: `podman compose up -d` → status `healthy`, UI antwoordt 200, `--health` in de container geeft 0.

**Open:** AC #7 — bereikbaar op de NAS en via Tailscale. Dat vraagt de UGREEN zelf; het image staat klaar om over te zetten.

**Nagekomen:** AC #7 is op 2026-08-28 volledig aangetoond. Op de NAS meldt `docker compose ps` de container als `healthy`, de UI antwoordt vanaf het LAN met 200 en toont de echte mappen van de share, en de eigenaar bevestigde dat hij ook via Tailscale bereikbaar is.
<!-- SECTION:FINAL_SUMMARY:END -->
