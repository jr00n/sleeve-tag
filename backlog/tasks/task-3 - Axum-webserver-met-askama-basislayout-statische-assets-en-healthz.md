---
id: TASK-3
title: 'Axum-webserver met askama-basislayout, statische assets en /healthz'
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:22'
updated_date: '2026-08-26 23:13'
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
- [x] #1 `cargo run` start een axum-server op de geconfigureerde poort en toont een pagina met de weergavenaam Sleeve
- [x] #2 Er is een askama-basislayout waarin latere pagina's kunnen worden opgenomen, responsive en bruikbaar op een telefoonscherm
- [x] #3 HTMX wordt vanaf een lokaal meegeleverd bestand geserveerd; de pagina laadt geen resources van externe hosts
- [x] #4 `GET /healthz` geeft HTTP 200 met een korte statusbody
- [x] #5 Requests worden gelogd naar stdout in leesbaar formaat op het geconfigureerde logniveau
- [x] #6 Een integratietest controleert dat /healthz 200 geeft en dat de startpagina rendert
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
## Uitgangssituatie (onderzocht 2026-08-27)

`src/web/mod.rs` is leeg op de doc-comment na. `main.rs` leest de configuratie en sluit af. Beschikbaar: axum 0.8,
askama 0.16, tower-http 0.7 (features `fs` en `trace`), tokio 1.53 met `rt-multi-thread`, `macros`, `net` en `signal`.

Askama 0.16 heeft geen `askama_axum` meer: een handler rendert zelf en verpakt het resultaat in `axum::response::Html`.
Templates komen in `templates/` in de crate root. HTMX 2.0.10 is opgehaald van unpkg en staat in `static/htmx.min.js`.

## Ontwerp

- `templates/base.html` — basislayout met `{% block %}`-punten voor titel en inhoud, `meta viewport` voor telefoongebruik,
  en verwijzingen naar uitsluitend lokale assets.
- `templates/index.html` — startpagina die de basislayout uitbreidt.
- `static/` — `htmx.min.js`, een eigen `app.css` en een favicon; geserveerd met `ServeDir` van tower-http.
- `src/web/mod.rs` — `router(config)` bouwt de router: `/`, `/healthz`, `/static/*`, met `TraceLayer` voor request-logging.
  De router wordt als losse functie opgeleverd zodat hij in tests zonder netwerk te gebruiken is.
- `src/main.rs` — wordt async (`#[tokio::main]`), bindt op de geconfigureerde poort en start de server met graceful
  shutdown op SIGTERM/Ctrl-C. Dat laatste hoort bij een container die `docker stop` krijgt: een lopende request mag niet
  halverwege afgekapt worden. Dit is klein en raakt geen andere taak.

## Teststrategie

Drie lagen, geen extra runtime-dependencies:

1. **Unit-tests in `src/web/mod.rs`** met `tower::ServiceExt::oneshot` — de router zonder netwerk: `/healthz` geeft 200 met
   body, `/` rendert een pagina met "Sleeve", een onbekend pad geeft 404, en `/static/htmx.min.js` wordt geserveerd.
   `tower` komt als dev-dependency binnen (feature `util`).
2. **Guard tegen externe resources**: een test die de gerenderde startpagina scant op `http://` en `https://` in
   `src`/`href`-attributen. Acceptatiecriterium #3 is anders alleen met het blote oog te controleren, en juist op de NAS
   (geen internet) valt het pas op als het misgaat.
3. **Integratietest `tests/server.rs`**: start de binary als subprocess op een vrije poort, wacht tot hij luistert, en doet
   via een rauwe `TcpStream` een `GET /healthz` en `GET /`. Dat bewijst acceptatiecriterium #1 — dat de server werkelijk
   op de geconfigureerde poort luistert — zonder een HTTP-client-crate toe te voegen. Dezelfde test controleert met
   `LOG_LEVEL=debug` dat een request een logregel oplevert (acceptatiecriterium #5).

## Stappen

1. `tower` als dev-dependency (feature `util`).
2. `static/app.css` en favicon toevoegen; herkomst en versie van htmx vastleggen in de README.
3. `templates/base.html` en `templates/index.html`.
4. `src/web/mod.rs`: router, handlers, unit-tests.
5. `src/main.rs`: async main, luisteren op de geconfigureerde poort, graceful shutdown.
6. `tests/server.rs`.
7. README aanvullen (statische assets, htmx-versie, hoe de UI te bereiken).
8. Kwaliteitspoort.

## Aandachtspunten

- De startpagina blijft inhoudelijk leeg: de mapbrowser is fase 1. Deze taak levert alleen het frame.
- `ServeDir` mag nooit buiten `static/` kunnen serveren; dat is standaardgedrag, maar de test op een onbekend pad legt het
  vast.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
De bestaande tests uit de configuratietaak hingen na deze wijziging: ze gebruikten `Command::output()`, dat op procesafsluiting wacht, en de binary is nu een langlopende server. De procesbesturing is verplaatst naar `tests/common/mod.rs` (starten, meelezen met stdout in een achtergrondthread, wachten tot de poort luistert, killen bij Drop). Meelezen in een thread is niet optioneel: een volgelopen stdout-pipe zou de server blokkeren.

Askama 0.16 heeft geen `askama_axum` meer. Handlers renderen zelf en verpakken het resultaat in `axum::response::Html`; renderfouten gaan via een eigen `WebError` die de technische oorzaak logt en de bezoeker een korte melding geeft.

Graceful shutdown op SIGTERM/Ctrl-C toegevoegd. `docker stop` stuurt SIGTERM; bij een app die tags naar bestanden schrijft mag een lopend verzoek niet halverwege afgekapt worden. Aantoonbaar werkend: de log toont 'SIGTERM ontvangen, afsluiten' gevolgd door 'Sleeve afgesloten'.

De server bindt op 0.0.0.0 omdat hij in een container draait; afscherming gebeurt op netwerkniveau (LAN en Tailscale), zoals het PRD vastlegt.

Statische assets worden relatief aan de werkdirectory geserveerd (`static/`). De Dockerfile-taak moet daarom een WORKDIR zetten waarin die map staat.

Acceptatiecriterium 2 is visueel geverifieerd in Chrome op 390x844 (telefoonformaat): geen horizontale scroll, het lange bibliotheekpad breekt af binnen de code-tag, en het donkere thema volgt de systeeminstelling. Een unit-test dekt alleen de aanwezigheid van de viewport-regel.

htmx 2.0.10 is opgehaald van unpkg en ingecheckt onder static/. Versie en herkomst staan in de README, met de instructie om die bij een update bij te werken.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Wat er is gebouwd

De webserver-basis waarop alle latere pagina's aansluiten: een axum-router met askama-templates, lokaal meegeleverde assets, request-logging en het healthcheck-endpoint dat Docker gebruikt.

## Wijzigingen

- **`src/web/mod.rs`**: `router()` als losse functie (zodat tests hem zonder netwerk kunnen aanroepen) met `/`, `/healthz` en `/static/*` via `ServeDir`, plus `TraceLayer` voor request-logging. `AppState` geeft de configuratie aan handlers door. `WebError` vertaalt een renderfout naar HTTP 500: technische oorzaak in het log, korte melding voor de bezoeker.
- **`templates/base.html` + `index.html`**: basislayout met blokken voor titel, kopbalk en inhoud, inclusief de viewport-regel die een telefoon nodig heeft.
- **`static/`**: htmx 2.0.10, een eigen telefoon-eerst stylesheet en een favicon.
- **`src/main.rs`**: async main die op de geconfigureerde poort bindt en netjes afsluit op SIGTERM of Ctrl-C. Kan de poort niet gebonden worden, dan stopt de app met een duidelijke logregel in plaats van stil te falen.
- **`tests/common/mod.rs`**: gedeelde procesbesturing voor alle integratietests.
- **README**: hoe de UI te bereiken, de herkomst van de assets, en waarom er geen frontend-build is.

## Tests

31 tests groen (was 21):

- 6 router-tests via `oneshot` (geen netwerk): healthz, startpagina, statische bestanden, 404 op een onbekend pad, en een pad met `..` dat niet buiten `static/` mag kijken.
- 1 guard die de gerenderde pagina scant op externe hosts. Op de NAS is er geen internet; een externe stylesheet zou daar pas opvallen als de pagina half leeg blijft.
- 4 servertests die de echte binary starten en er over TCP mee praten — dat is het enige dat bewijst dat hij werkelijk op de geconfigureerde poort luistert. De HTTP-verzoeken worden met de hand geschreven, zodat er geen HTTP-client-crate bijkomt.
- De configuratietests uit de vorige taak zijn omgebouwd naar dezelfde helper.

Aanvullend visueel geverifieerd in Chrome op 390x844: geen horizontale scroll, lange paden breken af, donker thema volgt het systeem.

## Wat de wijziging brak

De configuratietests hingen: die wachtten met `Command::output()` op procesafsluiting, en de binary is nu een server die blijft draaien. Opgelost door de procesbesturing naar een gedeelde helper te verplaatsen die met de uitvoer meeleest in een achtergrondthread — nodig, want een volgelopen stdout-pipe zou de server blokkeren.

## Aandachtspunt voor de Dockerfile-taak

Statische assets worden relatief aan de werkdirectory geserveerd, dus het image heeft een `WORKDIR` nodig waarin `static/` staat.
<!-- SECTION:FINAL_SUMMARY:END -->
