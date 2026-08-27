---
id: TASK-12
title: Atomisch schrijven met hervalidatie en optionele backup
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:24'
updated_date: '2026-08-27 21:38'
labels: []
milestone: m-2
dependencies:
  - TASK-6
documentation:
  - PRD.md
modified_files:
  - src/main.rs
  - src/atomic.rs
  - tests/common/mod.rs
  - README.md
  - CLAUDE.md
priority: high
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Harde eis uit het PRD: nooit dataverlies. De app schrijft in een bibliotheek die niet opnieuw op te bouwen is, en de container kan tijdens een schrijfactie worden afgebroken. Deze taak levert de schrijfstrategie die alle latere schrijfacties (tags en album art) gebruiken.

Werkwijze uit §8.4: schrijf naar een tijdelijk bestand in dezelfde map, valideer door het opnieuw in te lezen, en hernoem het pas daarna over het origineel. Bij elke fout blijft het origineel onaangetast. Bij `BACKUP_ON_WRITE=true` komt er een `.bak` naast het bestand te staan; standaard staat dit uit om de share niet te vervuilen.

Bijkomende eis uit acceptatiecriterium 2 van het MVP: eigenaar, groep en permissies van het originele bestand blijven na het schrijven ongewijzigd. Elke schrijfactie wordt gelogd met pad en gewijzigde velden.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Een schrijfhelper vervangt de inhoud van een bestand alleen na een geslaagde hervalidatie van het tijdelijke bestand
- [x] #2 Bij een fout tijdens schrijven of validatie blijft het origineel byte-voor-byte ongewijzigd en blijft er geen tijdelijk bestand achter
- [x] #3 Het tijdelijke bestand staat in dezelfde map als het origineel, zodat het hernoemen atomisch is
- [x] #4 Eigenaar, groep en permissies van het originele bestand blijven na een schrijfactie ongewijzigd
- [x] #5 Met BACKUP_ON_WRITE=true wordt een .bak naast het bestand geplaatst; met de standaardwaarde niet
- [x] #6 Elke geslaagde schrijfactie wordt gelogd met pad en de gewijzigde velden
- [x] #7 Een test simuleert een mislukking halverwege het schrijven en toont aan dat het origineel intact is
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

### 1. `src/atomic.rs` (nieuw) — de schrijfstrategie
Eén functie die alle latere schrijfacties gebruiken. Ze kent geen tags en geen afbeeldingen; ze weet alleen hoe je de inhoud van een bestand vervangt zonder het kwijt te raken.

```rust
pub fn replace<E>(
    path: &Path,
    options: Options,
    changes: &str,
    prepare: impl FnOnce(&Path) -> Result<(), E>,
    validate: impl FnOnce(&Path) -> Result<(), E>,
) -> Result<(), WriteError<E>>
```

De volgorde ligt vast in de functie, niet bij de aanroeper — zo is AC #1 een eigenschap van de code en geen afspraak:

1. Kopieer het origineel naar een tijdelijk bestand **in dezelfde map** (AC #3). De kopie is exact, zodat `prepare` alleen hoeft te veranderen wat het wil veranderen; voor tag-I/O is dat noodzakelijk, want lofty heeft een echt audiobestand nodig.
2. `prepare` maakt het tijdelijke bestand klaar.
3. `validate` leest het tijdelijke bestand opnieuw in. Faalt dit, dan gaat er niets over het origineel heen.
4. Eigenaar, groep en rechten van het origineel worden op het tijdelijke bestand gezet (AC #4).
5. Bij `backup` een `.bak` naast het origineel, gemaakt van het origineel zoals het nu nog is.
6. `std::fs::rename` over het origineel — binnen dezelfde map is dat atomair.

Het tijdelijke bestand hangt aan een guard met een `Drop` die het opruimt. Bij elke vroege terugkeer via `?` verdwijnt het dus vanzelf (AC #2); pas na een geslaagde `rename` wordt de guard ontwapend.

De naam is `.<bestandsnaam>.<pid>.sleeve-tmp`: met een punt ervoor, zodat de mapbrowser hem toch al overslaat, en met de pid erin tegen botsingen.

Twee foutsoorten worden apart gehouden: mislukt `prepare`, dan is er niets aan de hand; mislukt `validate`, dan hebben we zojuist een onbruikbaar bestand geproduceerd en verdient dat een eigen, luider logbericht.

### 2. Eigenaar en rechten (AC #4)
`std::fs::copy` neemt de rechten mee, maar het tijdelijke bestand krijgt de uid/gid van het proces. Klopt die niet met die van het origineel, dan wordt `chown` geprobeerd. Lukt dat niet, dan **faalt de schrijfactie** en blijft het origineel ongemoeid: stilletjes de eigenaar van een bestand in de bibliotheek veranderen is precies wat het PRD verbiedt, en op de NAS met de juiste `PUID`/`PGID` doet dit geval zich niet voor.

Dit is Unix-specifieke code. De app draait op Linux in de container en op macOS tijdens ontwikkelen; een Windows-pad is geen doel.

### 3. Logging (AC #6)
`atomic::replace` logt bij succes één regel met het pad en de meegegeven omschrijving van wat er veranderd is. De aanroeper levert die omschrijving, want alleen die weet welke velden er zijn aangeraakt.

### 4. Configuratie
`Options { backup: bool }` komt uit `config.backup_on_write`. `AppState` krijgt dat veld erbij zodra een handler schrijft (task-14); in deze taak blijft het bij de helper en zijn tests.

### 5. Tests
Alles met echte bestanden in een tempdir, nooit tegen de bibliotheek.

- Een geslaagde vervanging: de inhoud is nieuw, het tijdelijke bestand is weg.
- `prepare` faalt: origineel byte-voor-byte gelijk, geen restanten in de map (AC #2, #7).
- `validate` faalt terwijl `prepare` het bestand al had verminkt: origineel nog steeds gelijk — dit is het geval dat er werkelijk toe doet.
- Een paniek in `prepare` laat het origineel ook intact (de `Drop`-guard doet zijn werk).
- Het tijdelijke bestand staat in dezelfde map als het origineel (AC #3), gecontroleerd vanuit `prepare`.
- Rechten blijven gelijk, ook bij een afwijkende modus als `0o640` (AC #4).
- `.bak` verschijnt alleen met `backup: true`, en bevat de oude inhoud (AC #5).
- Een mislukte schrijfactie laat geen `.bak` achter.
- De logregel bevat pad en omschrijving (AC #6), gemeten met een `tracing`-subscriber die de regels opvangt.

### 6. Documentatie
README: een sectie over de schrijfstrategie, met de volgorde en wat er bij elke fout gebeurt. CLAUDE.md heeft de regel "Schrijven is atomisch" al; die wordt aangevuld met de vindplaats (`atomic::replace`) zodat latere schrijfacties er niet omheen gaan.

### Afwijkingen van het plan

- De logtests konden niet met `with_default` per test; er is één globale `tracing`-subscriber nodig die naar een thread-lokale buffer schrijft. Zie de implementatienotities.
- Niet gepland maar wel nodig: de poortrace in `tests/common/mod.rs` opnieuw aangepakt. De vorige oplossing dekte maar de helft van het probleem.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**De volgorde zit in de functie, niet in een afspraak.** `atomic::replace` neemt twee closures (`prepare` en `validate`) en bepaalt zelf wanneer ze draaien. Daarmee is AC #1 een eigenschap van de code: een aanroeper kán de hervalidatie niet overslaan. Was validatie iets wat de aanroeper zelf in zijn schrijfclosure moest doen, dan was het een belofte geweest.

**Het tijdelijke bestand begint als exacte kopie van het origineel.** Niet als leeg bestand: lofty heeft een echt audiobestand nodig om tags in te schrijven, en zo hoeft de aanroeper alleen te veranderen wat hij wil veranderen. Vastgelegd in een eigen test, want het is een contract waar task-13 op leunt.

**Opruimen via `Drop`, niet via foutpaden.** De `TempFile`-guard verwijdert het bestand bij elke vroege terugkeer, inclusief een paniek in de aanroeper. Getest met `catch_unwind`. Na een geslaagde `rename` wordt de guard ontwapend — anders zou een later bestand met dezelfde naam per ongeluk opgeruimd kunnen worden.

**Eigenaar overnemen mislukt? Dan gaat de schrijfactie niet door.** Het tijdelijke bestand draagt de uid/gid van het proces; zonder correctie zou het hernoemen stilletjes de eigenaar van een bibliotheekbestand veranderen. Weigeren is hier beter dan doorgaan: het PRD verbiedt ongevraagde wijzigingen, en op de NAS met de juiste PUID/PGID doet dit geval zich niet voor. `std::fs::copy` neemt de rechten al mee, maar ze worden pas ná `prepare` gezet — die kan het bestand opnieuw hebben aangemaakt.

**De backup wordt pas ná een geslaagde validatie gemaakt.** Anders zou een mislukte schrijfactie een `.bak` achterlaten van een wijziging die nooit is doorgevoerd. Aparte test daarvoor.

**Logtests vroegen een globale `tracing`-subscriber.** Mijn eerste opzet gebruikte `tracing::subscriber::with_default` per test. Die faalde consistent in een volledige run en slaagde in isolatie: `tracing` onthoudt pér logregel-in-de-code of er iemand luistert, en dat geheugen is globaal. Met 128 parallelle tests komt er een thread zónder subscriber langs dezelfde regel, waarna de test mét subscriber een lege buffer overhoudt. Opgelost met één globale subscriber die naar een thread-lokale buffer schrijft; regels van andere threads vallen op de grond.

**Poortrace in de testharnas nu echt gedicht.** De vorige oplossing (task-9) ving alleen op dat óns proces stierf. Wat overbleef: een geslaagde TCP-verbinding zegt alleen dat er *iets* op die poort luistert — met vier integratiebinaries naast elkaar kon dat de server van een andere test zijn. Ongeveer één op de drie volledige runs viel daarop om. `wait_until_listening` wacht nu op de regel 'webserver luistert' in de stdout van het eigen kindproces. Op één poort kan maar één proces luisteren, dus dat bewijst eigendom. Tien opeenvolgende volledige runs daarna zonder uitval.

**`#![allow(dead_code)]` is tijdelijk.** Er is nog geen productie-aanroeper; die komt in task-13. De uitzondering hoort weg zodra `tags::write` bestaat.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Atomisch schrijven met hervalidatie en optionele backup

De schrijfstrategie waar alle latere schrijfacties op leunen. `src/atomic.rs` (nieuw) weet niets van tags of afbeeldingen; het weet alleen hoe je de inhoud van een bestand vervangt zonder het kwijt te raken.

```rust
atomic::replace(path, options, changes, prepare, validate)
```

De volgorde staat vast in de functie: kopiëren naar een tijdelijk bestand in dezelfde map → `prepare` → `validate` → eigenaar/groep/rechten overnemen → optioneel `.bak` → `rename`. Daarmee is "vervangen pas na een geslaagde hervalidatie" een eigenschap van de code en niet een afspraak met de aanroeper.

### Beslissingen

- **Twee closures in plaats van één.** Zou de aanroeper zelf moeten valideren binnen zijn schrijfstap, dan kon hij dat overslaan. Nu niet.
- **Het tijdelijke bestand is een exacte kopie**, geen leeg bestand: lofty heeft een echt audiobestand nodig om tags in te schrijven. Task-13 leunt daarop, dus het is als contract getest.
- **Opruimen via `Drop`.** Elk foutpad — inclusief een paniek — laat de guard het tijdelijke bestand verwijderen, zonder dat het foutpad daar zelf aan hoeft te denken. Na de `rename` wordt de guard ontwapend.
- **Eigenaar niet over te nemen? Dan schrijven we niet.** Weigeren is hier beter dan een bibliotheekbestand stilletjes van eigenaar laten veranderen. Op de NAS met de juiste `PUID`/`PGID` doet dat geval zich niet voor.
- **De backup wordt pas ná de validatie gemaakt**, zodat een mislukte schrijfactie geen `.bak` achterlaat van iets wat nooit is gebeurd.
- **Prepare- en validatiefouten blijven gescheiden.** Bij het eerste is er niets gebeurd; bij het tweede is er zojuist een onbruikbaar bestand geproduceerd, en dat verdient een eigen foutregel in het log.

### Tests

163 tests groen (128 unit, 1 architectuur, 5 art, 12 mapbrowser, 7 ruwe tags, 6 configuratie, 4 server); `cargo fmt --check` en `cargo clippy -- -D warnings` schoon.

Vijftien tests in `atomic`, allemaal tegen echte bestanden in een tempdir:

- Geslaagd schrijven vervangt de inhoud en laat niets achter.
- Het tijdelijke bestand staat in dezelfde map en is een kopie van het origineel.
- Een fout halverwege `prepare`, een mislukte `validate` ná een al verminkt tijdelijk bestand, en een paniek in `prepare` — in alle drie de gevallen is het origineel byte-voor-byte gelijk en is de map schoon.
- Rechten (`0o640`) en eigenaar/groep overleven de schrijfactie.
- `.bak` verschijnt alleen met de optie aan, bevat de vorige inhoud, en blijft weg bij een mislukte schrijfactie.
- De logregel bevat pad en gewijzigde velden; een mislukte validatie geeft een `ERROR` die zegt dat het origineel heel is.

### Twee dingen die onderweg omvielen

- **De logtests werkten niet met een subscriber per test.** `tracing` onthoudt globaal per logregel of er iemand luistert; met 128 parallelle tests komt er een thread zonder subscriber langs en blijft "niemand luistert" hangen. Opgelost met één globale subscriber die naar een thread-lokale buffer schrijft.
- **De poortrace in de testharnas was nog niet weg.** De oplossing uit task-9 ving alleen op dat óns proces stierf; een geslaagde TCP-verbinding kon nog steeds van de server van een andere test zijn. Ongeveer één op de drie volledige runs viel daarop om. `wait_until_listening` wacht nu op de regel "webserver luistert" in de stdout van het eigen kindproces — op één poort kan maar één proces luisteren, dus dat bewijst eigendom. Tien opeenvolgende volledige runs daarna zonder uitval.

### Vervolg

`atomic` heeft nog `#![allow(dead_code)]`: er is geen productie-aanroeper. Die uitzondering hoort weg zodra `tags::write` bestaat (task-13).
<!-- SECTION:FINAL_SUMMARY:END -->
