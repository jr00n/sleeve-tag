---
id: TASK-35
title: 'Filteren op wat aandacht vraagt, met de telling in de kopbalk'
status: Done
assignee: []
created_date: '2026-08-30 07:04'
updated_date: '2026-08-30 12:25'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) zet naast het zoekveld een knop "Needs attention" met een telling erachter. Eén klik laat alleen de bestanden zien waar iets aan mankeert; nog een klik zet de lijst terug.

Sleeve signaleert al per bestand wat er ontbreekt of afwijkt (FR-4) en toont die labels in de lijst, maar er is geen manier om erop te filteren. In een map met honderd bestanden waar er drie een tracknummer missen, moet je die drie nu zelf zoeken.

De telling hoort bij de map die je bekijkt en zegt hoeveel bestanden daar ten minste één signalering hebben. Het filter werkt samen met het zoekveld dat er al is: samen versmallen ze de lijst, ze vervangen elkaar niet.

De signalering blijft constateren en niets meer: dit filter verandert daar niets aan en stelt geen correcties voor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 De mapweergave laat zien hoeveel bestanden in deze map ten minste één signalering hebben.
- [x] #2 Met één klik toont de lijst alleen die bestanden, en met nog een klik weer alles; de knop laat zien welke van de twee aan staat.
- [x] #3 Het filter en het zoekveld werken samen: staat er ook een zoekterm, dan blijft over wat aan allebei voldoet.
- [x] #4 De gekozen stand overleeft het verversen van de pagina en staat in de URL, zodat een gefilterde lijst te delen en te bookmarken is.
- [x] #5 Zonder JavaScript werkt het filter ook: het is dan een gewone link of knop die de pagina opnieuw laadt.
- [x] #6 Een map waarin niets aandacht vraagt, zegt dat met zoveel woorden in plaats van een lege lijst te tonen.
- [x] #7 Het filteren is met tests gedekt, inclusief de combinatie met een zoekterm en een map waarin alles in orde is.
- [x] #8 README is bijgewerkt.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 cargo fmt --check slaagt
- [x] #2 cargo clippy -- -D warnings slaagt
- [x] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [x] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [x] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Gebouwd in worktree `agent-a10e7cb642a08994a`, commit 7e1f0e6.

**Laagverdeling.** Tellen en filteren zitten in `browse::`; de handler leest
alleen de querystring en de templates renderen. `checks::` is niet aangeraakt en
blijft alleen constateren.

- `browse::Filter { query, only_flagged }` met `Filter::from_query(q, aandacht)`
  — wat "aan" betekent wordt op één plek beslist, niet in de handler.
  `browse::listing` neemt nu een `&Filter` in plaats van een `&str`.
- `TrackSummary::needs_attention()` — één definitie voor telling én filter.
- `Listing::flagged_count` wordt geteld ná `review()` maar vóór élk filter, dus
  over de hele map; `Listing::only_flagged` draagt de stand.
- `Listing::has_flagged()` en `Listing::attention_url()` bouwen de schakelaar:
  de link zet de stand om en bewaart de zoekterm, percent-encoded via een nieuwe
  `QUERY_ESCAPES` (`&`, `=`, `+` erbij) zodat een zoekterm de URL niet kan breken.

**URL en werking zonder JavaScript.** `?aandacht=1` naast de bestaande `?q=`;
de knop is een `<a href>` naar dezelfde pagina. Het zoekformulier krijgt een
hidden `aandacht`-veld wanneer het filter aan staat, anders zou zoeken (en de
HTMX-verversing met `hx-push-url`) het filter stilzwijgend uitzetten.

**Weergave.** Knop naast het zoekveld met naam + telling; aan is hij gevuld,
plus een uit beeld genomen maar voorleesbare tekst over de stand. Vraagt er
niets aandacht, dan staat er geen knop maar "Niets in deze map vraagt aandacht."
— een knop naar een lege lijst is een doodlopend spoor. In `listing.html` een
eigen lege-lijstboodschap voor de gefilterde stand (met en zonder zoekterm).
Alle nieuwe CSS gebruikt uitsluitend bestaande Nocturne-tokens; geen enkele
nieuwe kleurwaarde buiten het tokenblok.

**Tests.** Negen unit-tests in `browse::tests` (tellen, filteren, filter+zoekterm
als AND, nette map, mapmeldingen overleven het filter, toggle-URL, escaping,
duiding van de parameter) en vier integratietests in `tests/browse.rs`
(telling + link in de HTML, versmallen en terugschakelen, combinatie met
zoekterm inclusief de uitleg bij een leeg resultaat, en een map waarin alles in
orde is). Alles via `place_fixture` in een tempdir; de echte bibliotheek wordt
niet aangeraakt.

**Kwaliteitspoort.** `cargo fmt --check` groen, `cargo clippy -- -D warnings`
groen (ook met `--all-targets`), `cargo test` 431 tests groen.

README: nieuwe subsectie "Filteren op wat aandacht vraagt" onder de signalering,
met de URL-tabel, plus een verwijzing vanuit de zoekveld-alinea.
<!-- SECTION:NOTES:END -->
