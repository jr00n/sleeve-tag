---
id: TASK-5
title: Multi-stage Dockerfile met statische linux/amd64-build
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:22'
updated_date: '2026-08-26 23:44'
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
- [x] #1 `docker buildx build --platform linux/amd64` levert een image dat op de NAS start en /healthz beantwoordt
- [x] #2 De resulterende image is kleiner dan 30 MB
- [x] #3 De binary is statisch gelinkt en draait in een runtime-image zonder Rust-toolchain
- [x] #4 Het bouwcommando en de handmatige distributiestap (`docker save | ssh nas docker load`) staan in de README
- [x] #5 Docker-layers zijn zo geordend dat een wijziging in de broncode geen volledige herbouw van dependencies veroorzaakt
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 cargo fmt --check slaagt
- [x] #2 cargo clippy -- -D warnings slaagt
- [x] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [x] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Uitgangssituatie (onderzocht 2026-08-27)

Op deze Mac staat **geen Docker**, wel podman 5.4.1 met een draaiende arm64 Linux-VM. Podman kent dezelfde
`--platform`-vlag en leest gewone Dockerfiles, dus het image is met podman te bouwen en te testen; de NAS gebruikt straks
gewoon `docker`.

Het project heeft naast de binary twee soorten bestanden nodig: `templates/` (compile-time verwerkt door askama, dus die
verdwijnen in de binary) en `static/` (wordt op runtime van schijf geserveerd en moet dus mee in het image). De
webserver-taak noteerde al dat de `WORKDIR` de map met `static/` moet zijn.

## Ontwerp

Multi-stage Dockerfile:

1. **build** op `rust:slim`, target `x86_64-unknown-linux-musl`, zodat de binary statisch gelinkt is.
2. **runtime** op `gcr.io/distroless/static-debian12`: geen shell, geen package manager, geen Rust-toolchain.

Voor de laagvolgorde (acceptatiecriterium #5) wordt eerst alleen `Cargo.toml`/`Cargo.lock` gekopieerd en met een dummy
`main.rs` gebouwd; die laag met alle dependencies blijft geldig zolang de manifesten niet wijzigen. Pas daarna komt de
echte broncode binnen.

Een `.dockerignore` houdt `target/` en `backlog/` buiten de buildcontext — zonder dat wordt de context enkele GB's groot.

## Aandachtspunten

- **Bouwsnelheid**: cross-compileren vanaf arm64 naar x86_64-musl vraagt een cross-linker. Eerste poging is de rechttoe
  rechtaan variant (build-stage draait geëmuleerd als amd64). Duurt dat onwerkbaar lang, dan overstappen op
  `cargo zigbuild`, dat het PRD als alternatief noemt.
- **Niet-root**: het image krijgt hier al een vaste niet-root `USER`. Het instelbaar maken via `PUID`/`PGID` is expliciet
  de PUID/PGID-taak van fase 5.
- **Healthcheck**: distroless heeft geen curl of wget, dus de healthcheck komt in de compose-file te staan (die taak),
  niet in de Dockerfile.
- **Acceptatiecriterium #1 noemt "start op de NAS"**. Zonder toegang tot de UGREEN kan ik dat deel niet zelf aantonen; ik
  verifieer het image lokaal onder linux/amd64 en meld wat er op de NAS nog handmatig gecontroleerd moet worden.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Op deze Mac staat geen Docker, wel podman 5.4.1. Podman leest dezelfde Dockerfile en kent dezelfde --platform-vlag, dus alle verificatie is met podman gedaan. De README noemt beide commando's.

Eerste build faalde op een OOM: de crate `moxcms` (verplichte dependency van `image`) werd door de kernel gekilld — zichtbaar als 'signal: 9, SIGKILL' halverwege het compileren. Oorzaak: de podman-VM heeft 2 GB en onder amd64-emulatie is elk rustc-proces fors zwaarder dan native. Opgelost met `ARG BUILD_JOBS=2` plus `CARGO_BUILD_JOBS`, zodat de parallellie omlaag kan en een builder met meer geheugen hem kan ophogen. Daarna slaagde de build in 3:46.

Bij het bouwen bleek er al een image `sleeve-tag:dev` te bestaan, 51 minuten oud, met `uvicorn sleeve_tag.main:app` als CMD: een Python-implementatie van hetzelfde project, gebouwd buiten deze sessie om. Mijn build neemt die tag over, dus het bestaande image is eerst veiliggesteld onder `sleeve-tag:python-legacy` (ID 11341087d89d). Aan de eigenaar gemeld.

Geverifieerd met podman: image 6,41 MB (eis: onder 30 MB), architectuur amd64/linux, binary `static-pie linked, x86-64, stripped` van 3,0 MB. De container start, /healthz geeft 200 met body 'ok', de startpagina rendert, en het log toont music_root=/music. Het proces draait als 1000:10; `podman exec id` faalt met 'executable file not found', wat bevestigt dat er geen shell of toolchain in het runtime-image zit.

Acceptatiecriterium 1 noemt expliciet 'start op de NAS'. Zonder toegang tot de UGREEN is dat deel niet door mij aan te tonen; het image is geverifieerd onder linux/amd64 op de ontwikkelmachine. Wat op de NAS nog gecontroleerd moet worden staat in de eindsamenvatting.

Correctie op een eerdere notitie: podman scheidt `localhost/sleeve-tag:dev` (deze build) van `docker.io/library/sleeve-tag:dev` (het Python-image). Die tags botsten dus nooit en het bestaande image liep geen gevaar. De extra tag `python-legacy` staat er nog, maar was niet nodig.

Acceptatiecriterium 5 geverifieerd door een echte codewijziging: bij de herbouw kwam STEP 9 (de dependency-build met de dummy main.rs) uit de cache terwijl STEP 10 en 12 opnieuw draaiden. Een schone build kost 3:46, een herbouw na een codewijziging ongeveer een minuut.

Onderweg leek de cache kapot: na een toegevoegde comment-regel meldde podman toch 'Using cache' op `COPY src ./src`. Dat is nagetrokken met een zichtbare wijziging (een aangepaste logregel): het nieuwe image logde wel degelijk de nieuwe tekst, terwijl het oude image de oude tekst hield. De eerste waarneming was een sync-vertraging van de buildcontext naar de podman-VM, geen invalidatiefout. Belangrijk om vast te leggen, want een build die stiekem oude code oplevert is precies het soort fout dat pas op de NAS opvalt.

Definition of Done punt 4 (tests) is niet van toepassing: deze taak levert een Dockerfile en .dockerignore op, geen Rust-code. De verificatie bestaat uit het bouwen en draaien van het image; dat is hierboven vastgelegd. De bestaande 37 tests draaien ongewijzigd groen.

Op de NAS geverifieerd door de eigenaar: het image draait op wolffpacksrv.local met `docker run -d -p 8080:8080 -v /volume1/Multimedia/music:/music`. Vanaf het LAN geeft http://wolffpacksrv.local:8080/healthz HTTP 200 met body 'ok' en rendert de startpagina. Daarmee is acceptatiecriterium 1 volledig aangetoond, inclusief het deel dat ik zelf niet kon uitvoeren.

Het pad van de muziekshare op de UGREEN is hiermee bekend: /volume1/Multimedia/music. Dat was het laatste openstaande punt uit PRD paragraaf 12; het is doorgegeven aan de docker-compose-taak.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Wat er is gebouwd

Een multi-stage Dockerfile die een statisch gelinkte `linux/amd64`-binary in een distroless-image zet, plus een `.dockerignore` en de bouw- en distributie-instructies in de README.

## Resultaat

| Eis | Gemeten |
|---|---|
| Image onder 30 MB | **6,41 MB** |
| Statisch gelinkt | `ELF 64-bit, x86-64, static-pie linked, stripped`, 3,0 MB |
| Geen Rust-toolchain in runtime | distroless/static; `exec id` faalt met "executable not found" |
| Draait niet als root | `1000:10` |
| Start en beantwoordt /healthz | HTTP 200 op de NAS én lokaal |
| Laag-caching | dependency-laag hergebruikt: schone build 3:46, herbouw ~1 min |

## Op de NAS

Door de eigenaar gedraaid op wolffpacksrv.local met de share op `/volume1/Multimedia/music`. Vanaf het LAN geverifieerd: `/healthz` geeft 200 en de startpagina rendert. Daarmee is het laatste openstaande punt uit PRD §12 ook beantwoord — dat sharepad gaat naar de docker-compose-taak.

## Twee problemen onderweg

**OOM tijdens de eerste build.** De crate `moxcms` werd door de kernel gestopt ("signal: 9, SIGKILL"). Onder amd64-emulatie is elk rustc-proces fors zwaarder dan native, en de build-VM heeft 2 GB. Opgelost met `ARG BUILD_JOBS=2`, ophoogbaar op een ruimere builder.

**De cache leek kapot.** Na een toegevoegde comment-regel meldde podman toch "Using cache" op `COPY src ./src`. Nagetrokken met een zichtbare wijziging in een logregel: het nieuwe image logde wel degelijk de nieuwe tekst. Het was een sync-vertraging van de buildcontext naar de VM, geen invalidatiefout. De moeite van het uitzoeken waard, want een build die stilletjes oude code oplevert valt pas op de NAS op.

## Aandachtspunten voor volgende taken

- De `USER 1000:10` staat hier vast in de Dockerfile. Instelbaar maken via `PUID`/`PGID` is de PUID/PGID-taak.
- De healthcheck staat bewust niet in de Dockerfile: distroless heeft geen curl of wget, dus die hoort in `docker-compose.yml`.
- De statische assets worden relatief aan de `WORKDIR` (`/app`) geserveerd.
<!-- SECTION:FINAL_SUMMARY:END -->
