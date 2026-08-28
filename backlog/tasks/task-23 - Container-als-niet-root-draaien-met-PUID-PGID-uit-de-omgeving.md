---
id: TASK-23
title: Container als niet-root draaien met PUID/PGID uit de omgeving
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:26'
updated_date: '2026-08-28 19:30'
labels: []
milestone: m-5
dependencies:
  - TASK-2
  - TASK-5
  - TASK-12
documentation:
  - PRD.md
modified_files:
  - src/startup.rs
  - src/main.rs
  - tests/startup.rs
  - Dockerfile
  - README.md
  - CLAUDE.md
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
- [x] #1 Het applicatieproces in de container draait niet als root
- [x] #2 PUID en PGID worden bij start toegepast, met 1000 en 10 als standaardwaarden
- [x] #3 Bestanden die de app op een gemount volume schrijft krijgen de eigenaar en groep die met PUID/PGID zijn ingesteld
- [x] #4 Bij ontbrekende schrijfrechten op MUSIC_ROOT geeft de app bij start een duidelijke melding in plaats van pas bij de eerste schrijfactie te falen
- [x] #5 De werking is aantoonbaar getest op de UGREEN NAS met de echte share
- [x] #6 De keuze tussen entrypoint en compose `user:` is met reden gedocumenteerd in de README
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

**Keuze entrypoint vs. compose `user:` → compose `user:`.**
De runtime is `gcr.io/distroless/static-debian12`: geen shell, geen `su-exec`,
geen `chown`. Een entrypoint-script dat PUID/PGID toepast, kan daar dus niet
draaien zonder een shell (en dus root) terug het image in te halen. Dat zou de
eis "draait niet als root" juist ondermijnen. Docker/compose kan de UID:GID
zelf zetten met `user:`; het proces start dan meteen als die gebruiker en is
nooit root geweest. Het `Dockerfile` houdt `USER 1000:10` als standaard, zodat
een `docker run` zonder compose ook al niet-root is.

**Wat de app zelf doet: verifiëren, niet toepassen.**
Een niet-root proces kan zijn eigen UID niet veranderen; `PUID`/`PGID` zijn dus
een *verwachting* die de app bij start toetst. Nieuwe module `startup::`:

1. Zet één sondebestand (`.sleeve-startcontrole-<pid>`) in `MUSIC_ROOT`, leest
   de uid/gid van dat bestand terug en verwijdert het meteen. Eén sonde
   beantwoordt beide vragen tegelijk, en beantwoordt ze op de echte manier:
   *kan* er geschreven worden, en met welke eigenaar/groep komt een geschreven
   bestand er dan te staan. Een controle op alleen de mode-bits liegt bij ACL's
   en op een setgid-map.
2. Lukt het schrijven niet → duidelijke melding op ERROR-niveau bij start, met
   de reden en wat er niet zal werken (AC #4). De app blijft wél draaien:
   bladeren en tags bekijken werkt op een read-only mount gewoon, en een UI die
   opkomt is makkelijker te diagnosticeren dan een container die herstart-lust.
   Configuratiefouten blijven wél fataal — dat onderscheid bestaat al.
3. Wijkt de gemeten uid/gid af van `PUID`/`PGID` → WARN met beide waarden, zodat
   in `docker logs` meteen zichtbaar is dat `user:` en de env-variabelen uit
   elkaar lopen. Niet fataal: lokaal ontwikkelen op de Mac (uid 501) moet blijven
   werken.

## Bestanden

- `src/startup.rs` (nieuw) — de startcontrole, met unit-tests op tempdirs.
- `src/main.rs` — controle aanroepen na `log_effective()`, vóór het binden.
- `tests/startup.rs` (nieuw) — integratietest via de echte route: een
  schrijfbare `MUSIC_ROOT` logt de bevestiging, een read-only map de melding.
  De read-only test slaat zichzelf over als het proces tóch mag schrijven
  (root in CI).
- `Dockerfile` — commentaar bij `USER` bijwerken naar de gemaakte keuze.
- `README.md` — sectie "Rechten en eigenaarschap": waarom `user:` en geen
  entrypoint, hoe PUID/PGID samenhangen met `user:`, en wat de startcontrole
  meldt.

## Niet in deze taak

- `docker-compose.yml` zelf: dat is TASK-24.
- AC #5 (aantoonbaar op de NAS) vergt SSH met wachtwoord en een image-build;
  dat kan ik niet zelf uitvoeren. Wordt gemeld en hoort bij TASK-27.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Keuze vastgelegd: PUID/PGID worden door de container-runtime toegepast (`user:` in compose, `USER 1000:10` als standaard in het image), niet door een entrypoint-script. Reden: de distroless-runtime heeft geen shell, en een script dat de UID wisselt zou als root moeten starten — precies wat AC #1 uitsluit. Onderbouwing staat in README §Rechten en eigenaarschap en in het commentaar bij `USER` in de Dockerfile.

De app past PUID/PGID dus niet toe maar toetst ze. `startup::check` zet één sondebestand in MUSIC_ROOT, leest de eigenaar terug en ruimt het op: dat beantwoordt in één keer of er geschreven mag worden én met welke uid/gid. Mode-bits controleren zou op een share met ACL's of een setgid-map het verkeerde antwoord geven.

Een niet-schrijfbare MUSIC_ROOT is ERROR-in-de-log, geen exit. Bladeren en tags bekijken werkt op een read-only mount gewoon, en een draaiende UI is beter te diagnosticeren dan een herstartlus. Configuratiefouten blijven wél fataal — dat onderscheid bestond al en blijft zichtbaar.

AC #5 (aantoonbaar op de UGREEN met de echte share) is niet afgevinkt: dat vraagt om een image-build plus SSH met wachtwoord naar wolffpacksrv.local, wat ik niet zelf kan uitvoeren. De controle hoort bij de MVP-acceptatie in TASK-27; de logregels om op te letten staan in de tabel in README §Rechten en eigenaarschap.

AC #5 aangetoond op wolffpacksrv.local met de echte share (2026-08-28). De container, gestart met `docker compose up -d` vanuit /volume2/Docker/sleeve-tag, logde:

    INFO sleeve_tag::startup: MUSIC_ROOT is schrijfbaar uid=1000 gid=10

Die regel is geen aanname maar een meting: de sonde is een bestand dat werkelijk in /volume1/Multimedia/Music is aangemaakt en waarvan de eigenaar is teruggelezen. Daarmee is in één keer aangetoond dat het proces niet als root draait, dat `user:` PUID/PGID heeft toegepast, en dat wat de app op de share schrijft eigenaar 1000 en groep 10 krijgt — gelijk aan de rest van de share (`ls -dn` gaf `1000 10`).

Onderweg bleek de map `/volume1/Multimedia/Music` te heten, met hoofdletter; `.env.example` is gecorrigeerd (commit 8e485da).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Het proces draait niet als root en `PUID`/`PGID` worden bij start toegepast door de container-runtime; de app toetst bij start of dat ook zo is uitgepakt en meldt het als het niet klopt.

**Keuze:** compose `user:` in plaats van een entrypoint-script. De distroless-runtime heeft geen shell, en een script dat de UID wisselt zou als root moeten beginnen — precies wat de eis "draait niet als root" uitsluit. Het image houdt `USER 1000:10` als standaard, zodat ook een kale `docker run` al niet-root is.

**Nieuw: `startup::`.** Zet één sondebestand in `MUSIC_ROOT`, leest de eigenaar ervan terug en ruimt het meteen op. Eén sonde beantwoordt beide vragen tegelijk — mág er geschreven worden, en welke uid/gid krijgt wat er geschreven wordt — en doet dat op de enige manier die niet liegt bij ACL's of een setgid-map. Uitkomsten: schrijfbaar en gelijk aan PUID/PGID → INFO; schrijfbaar maar afwijkend → WARN met beide waardenparen; niet schrijfbaar → ERROR die zegt dat opslaan zal mislukken. In alle gevallen draait de app door; een configuratiefout blijft wél fataal.

**Gedekt door tests:** unit-tests in `src/startup.rs` (eigenaar van een nieuw bestand, sonde laat niets achter, read-only map, naamgeving) en `tests/startup.rs`, dat de binary werkelijk start en de logregels leest die op de NAS in `docker logs` verschijnen. De read-only gevallen slaan zichzelf over als het testproces tóch mag schrijven (root in CI).

**Documentatie:** README §"Rechten en eigenaarschap" (waarom `user:` en geen entrypoint, hoe `user:` en PUID/PGID samenhangen, tabel met de startmeldingen), een architectuurregel in CLAUDE.md, en het commentaar bij `USER` in de Dockerfile.

**Open:** AC #5 — de werking aantoonbaar testen op de UGREEN met de echte share. Dat vraagt een image-build en SSH met wachtwoord; het hoort bij de MVP-acceptatie in TASK-27.

**Nagekomen:** AC #5 is op 2026-08-28 aangetoond op de UGREEN met de echte share. De startcontrole logde daar `MUSIC_ROOT is schrijfbaar uid=1000 gid=10` — een meting aan een bestand dat werkelijk in de share is aangemaakt, en daarmee het bewijs voor niet-root draaien, toegepaste PUID/PGID én het eigenaarschap van wat de app schrijft.
<!-- SECTION:FINAL_SUMMARY:END -->
