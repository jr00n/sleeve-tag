---
id: TASK-2
title: Configuratie inlezen uit omgevingsvariabelen
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:22'
updated_date: '2026-08-26 22:56'
labels: []
milestone: m-0
dependencies: []
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De app wordt volledig via omgevingsvariabelen geconfigureerd, omdat hij als container draait zonder configuratiebestand. Zonder deze laag kan geen enkele latere fase weten waar de muziek staat of hoe groot album art mag worden.

Te ondersteunen variabelen met hun betekenis: `MUSIC_ROOT` (pad naar de gemounte muziekshare, verplicht), `PORT` (HTTP-poort), `PUID`/`PGID` (eigenaar van weggeschreven bestanden op de NAS; standaard 1000 en 10), `MAX_ART_SIZE` (maximale resolutie van embedded art, standaard 1000x1000), `LOG_LEVEL` en `BACKUP_ON_WRITE` (standaard uit).

De feitelijke toepassing van PUID/PGID gebeurt in fase 5; deze taak zorgt alleen dat de waarden gelezen en gevalideerd worden.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Alle genoemde omgevingsvariabelen worden ingelezen in een getypeerde configuratiestruct met de in het PRD genoemde standaardwaarden
- [x] #2 Een ontbrekende of niet-bestaande `MUSIC_ROOT` laat de app met een duidelijke foutmelding stoppen in plaats van te starten
- [x] #3 Ongeldige waarden (bijv. niet-numerieke PORT) geven een begrijpelijke foutmelding die de naam van de variabele noemt
- [x] #4 De effectieve configuratie wordt bij start gelogd
- [x] #5 Unit-tests dekken standaardwaarden, geldige overrides en foutgevallen
- [x] #6 Tests zetten MUSIC_ROOT altijd op een tempdir met gekopieerde fixtures, zodat een test de echte bibliotheek per constructie niet kan raken
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

`src/config.rs` bestaat als lege module met alleen een doc-comment. clap 4.6 staat met features `derive` en `env` in
Cargo.toml. In `main.rs` staat nu een tijdelijke `log_directive`-helper die `LOG_LEVEL` rechtstreeks uit de omgeving leest;
die verhuist in deze taak naar `config`.

## Ontwerp

Een `Config`-struct met clap `Parser` en per veld `#[arg(long, env = "...")]`. Dat geeft naast de omgevingsvariabelen ook
CLI-flags, wat lokaal ontwikkelen op de Mac makkelijker maakt (`cargo run -- --music-root ~/muziek-test`).

Per veld een eigen `value_parser`-functie die bij een fout de **naam van de omgevingsvariabele** in de melding zet.
Dat is nodig voor acceptatiecriterium #3: clap noemt uit zichzelf alleen de CLI-flag, niet de variabelenaam.

Velden en standaardwaarden:

| Veld | Env | Default | Validatie |
|---|---|---|---|
| `music_root` | `MUSIC_ROOT` | geen (verplicht) | moet bestaan en een map zijn; wordt gecanonicaliseerd |
| `port` | `PORT` | 8080 | u16, > 0 |
| `puid` | `PUID` | 1000 | u32 |
| `pgid` | `PGID` | 10 | u32 |
| `max_art_size` | `MAX_ART_SIZE` | 1000x1000 | `N` of `BxH`, beide > 0 |
| `log_level` | `LOG_LEVEL` | info | lege waarde valt terug op info |
| `backup_on_write` | `BACKUP_ON_WRITE` | false | true/false/1/0/yes/no/on/off, case-insensitief |

`MAX_ART_SIZE` accepteert zowel `1000` als `1000x1000`, omdat het PRD de waarde als 1000x1000 opschrijft maar verkleinen
de beeldverhouding behoudt; intern is het een `MaxArtSize { width, height }`.

`music_root` wordt hier gecanonicaliseerd zodat `fs::` later een betrouwbaar anker heeft. Dat ondermijnt de architectuurregel
niet: die gaat over paden die uit requests binnenkomen, niet over de configuratie zelf.

## Teststrategie

Twee lagen, allebei zonder de omgeving van het testproces te muteren (`std::env::set_var` is in edition 2024 unsafe en
onbetrouwbaar met parallelle tests):

1. **Unit-tests op de parsers** in `src/config.rs` — pure functies, dus geldige waarden, randgevallen en foutmeldingen
   (inclusief de eis dat de variabelenaam in de melding staat) zijn direct te testen.
2. **Integratietest `tests/config_env.rs`** die de gebouwde binary via `env!("CARGO_BIN_EXE_sleeve-tag")` als subprocess
   start met `.env_clear()` en een gecontroleerde set variabelen. Dat verifieert de echte env-route end-to-end: ontbrekende
   `MUSIC_ROOT`, niet-bestaande `MUSIC_ROOT`, niet-numerieke `PORT`, en de geslaagde start met de effectieve configuratie
   in de logregel. `MUSIC_ROOT` wijst daarbij altijd naar een tempdir.

`tempfile` wordt als dev-dependency toegevoegd; die is in de fixtures-taak toch nodig.

## Stappen

1. `tempfile` als dev-dependency toevoegen.
2. `src/config.rs` invullen: `Config`, `MaxArtSize`, de parsers, en `Config::log_effective()` voor acceptatiecriterium #4.
3. `main.rs` omzetten: eerst config parsen, dan de subscriber initialiseren op `config.log_level`, dan de effectieve
   configuratie loggen. De tijdelijke `log_directive`-helper en zijn tests verhuizen naar `config`.
4. `tests/config_env.rs` schrijven.
5. README aanvullen met de tabel van omgevingsvariabelen en hun standaardwaarden.
6. Kwaliteitspoort draaien.

## Aandachtspunten

- Het PRD noemt geen standaardpoort; 8080 gekozen en gedocumenteerd. De compose-file mapt straks toch expliciet.
- Schrijfrechten op `MUSIC_ROOT` worden hier **niet** gecontroleerd; dat is expliciet een acceptatiecriterium van de
  PUID/PGID-taak in fase 5.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
clap maakt van een `bool`-veld automatisch een waardeloze flag. Daardoor werd de eigen value_parser overgeslagen en zou `BACKUP_ON_WRITE=misschien` stilzwijgend als 'aan' zijn gelezen. Opgelost met een expliciete `action = ArgAction::Set`; de integratietest dekt dit geval nu.

De logregels gaan naar stdout (PRD 8.5), clap-foutmeldingen naar stderr. De integratietest controleert allebei op de juiste stream.

ANSI-kleuren stonden altijd aan, waardoor er escape-codes tussen veldnaam en waarde in de log stonden (`port<esc>=8080`). Onleesbaar in `docker logs` en niet doorzoekbaar. Nu `with_ansi(stdout().is_terminal())`: kleur alleen wanneer een mens meekijkt.

De standaardwaarden en het ontbreken van MUSIC_ROOT worden bewust niet als unit-test gecontroleerd: clap leest daar de omgeving van het testproces, dus een gezette MUSIC_ROOT of PORT in de shell zou de uitkomst bepalen. Die gevallen staan in tests/config_env.rs, dat de binary met `env_clear()` als subprocess start.

MAX_ART_SIZE accepteert zowel `1000` als `1000x1000`. Het PRD noteert de standaard als 1000x1000, maar omdat verkleinen de beeldverhouding behoudt is een enkel getal net zo bruikbaar; de compose-file mag daardoor leesbaar blijven.

Standaardpoort 8080 gekozen; het PRD noemt geen waarde. Vastgelegd in de README-tabel.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Wat er is gebouwd

De configuratielaag van Sleeve: alle instellingen komen uit omgevingsvariabelen, getypeerd en gevalideerd, met foutmeldingen die de variabele bij naam noemen. Zonder deze laag weet geen enkele latere fase waar de muziek staat.

## Wijzigingen

- **`src/config.rs`**: `Config` (clap `Parser`) met `music_root`, `port`, `puid`, `pgid`, `max_art_size`, `log_level` en `backup_on_write`, elk met `#[arg(long, env = ...)]` zodat dezelfde waarden ook als CLI-flag werken. Per veld een eigen `value_parser` die bij een fout de naam van de omgevingsvariabele noemt — clap noemt uit zichzelf alleen de CLI-flag, en bij een container die niet opstart wil je juist de variabele zien.
- **`MaxArtSize`** accepteert `1000` en `1000x1000`; verkleinen behoudt de beeldverhouding, dus het zijn bovengrenzen per as.
- **`MUSIC_ROOT`** wordt gecontroleerd op bestaan en type, en gecanonicaliseerd — zo heeft de padmodule straks een betrouwbaar anker.
- **`Config::log_effective()`** logt de configuratie waarmee de app daadwerkelijk draait.
- **`src/main.rs`**: eerst config parsen, dan de subscriber op het geconfigureerde niveau, dan de effectieve configuratie loggen. De tijdelijke `log_directive`-helper is vervangen. ANSI-kleuren staan nu alleen aan bij een echte terminal.
- **README**: tabel met alle variabelen, standaardwaarden en betekenis.

## Tests

21 tests groen (was 4):

- 14 unit-tests op de parsers: geldige waarden, randgevallen (poort 0, afmeting 0, lege `LOG_LEVEL`, hoofdletters) en foutmeldingen, telkens met de eis dat de variabelenaam erin staat.
- 6 integratietests in `tests/config_env.rs` die de binary met `env_clear()` als subprocess starten: standaardwaarden, overrides, ontbrekende en niet-bestaande `MUSIC_ROOT`, ongeldige waarden, en de terugval van een lege `LOG_LEVEL`. Zo hangt niets af van wat er in de shell van de ontwikkelaar of CI-runner staat.
- De architectuurguard uit de vorige taak draait ongewijzigd mee.

## Twee dingen die de tests boven water haalden

- clap maakt van een `bool` een waardeloze flag, waardoor de eigen parser werd overgeslagen: `BACKUP_ON_WRITE=misschien` zou als "aan" zijn gelezen. Opgelost met `action = ArgAction::Set`.
- ANSI-kleurcodes stonden tussen veldnaam en waarde in de logregels, wat `docker logs` onleesbaar en ondoorzoekbaar maakt.

## Openstaand

Schrijfrechten op `MUSIC_ROOT` worden hier niet gecontroleerd; dat is expliciet een acceptatiecriterium van de PUID/PGID-taak in fase 5.
<!-- SECTION:FINAL_SUMMARY:END -->
