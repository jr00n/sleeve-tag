---
id: TASK-20
title: 'Album art valideren, verkleinen en hercoderen (FR-15)'
status: Done
assignee: []
created_date: '2026-08-26 22:25'
updated_date: '2026-08-28 13:00'
labels: []
milestone: m-4
dependencies:
  - TASK-2
documentation:
  - PRD.md
modified_files:
  - src/art.rs
  - src/config.rs
  - tests/config_env.rs
  - PRD.md
  - README.md
  - CLAUDE.md
priority: medium
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Geuploade hoezen zijn vaak veel groter dan nodig; een 3000x3000 scan in elk van de twaalf tracks blaast het album op en vertraagt elke speler. De app moet te grote art daarom optioneel verkleinen naar een configureerbare maximale resolutie (standaard 1000x1000 via `MAX_ART_SIZE`) met instelbare JPEG-kwaliteit.

Deze taak levert de beeldverwerking als losstaande, testbare laag: valideren dat een upload werkelijk een JPEG of PNG is, afmetingen bepalen, verkleinen met behoud van beeldverhouding, en opnieuw encoderen.

Aanname voor de open vraag uit PRD §12 (JPEG-only of PNG behouden): het bronformaat wordt behouden wanneer de afbeelding niet verkleind hoeft te worden; bij verkleinen wordt naar JPEG gecodeerd tenzij het origineel transparantie bevat. Bevestig dit met de eigenaar voordat de taak wordt afgerond.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Een geuploade afbeelding wordt gevalideerd op werkelijk formaat (JPEG of PNG), niet alleen op bestandsnaam of content-type header
- [x] #2 Afbeeldingen groter dan MAX_ART_SIZE worden verkleind met behoud van beeldverhouding; kleinere afbeeldingen blijven ongewijzigd
- [x] #3 De JPEG-kwaliteit bij hercoderen is configureerbaar en heeft een gedocumenteerde standaardwaarde
- [x] #4 Een bestand dat geen geldige afbeelding is wordt geweigerd met een begrijpelijke melding, zonder panic
- [x] #5 Er is een bovengrens aan de geaccepteerde uploadgrootte zodat een enorme upload de NAS niet plat legt
- [x] #6 Unit-tests dekken: te grote JPEG, te grote PNG, al kleine afbeelding, PNG met transparantie, en een ongeldig bestand
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
De open vraag uit PRD §12 is met de eigenaar bevestigd: de aanname uit de taak
geldt. Het bronformaat blijft behouden zolang de afbeelding binnen
`MAX_ART_SIZE` past — dan gaan de bytes ongewijzigd het bestand in. Moet er
verkleind worden, dan JPEG, tenzij er werkelijk doorzichtige pixels in zitten.
De vraag is uit PRD §12 gehaald en onder "Beantwoord" vastgelegd, samen met de
al eerder beantwoorde vraag over de standaardsortering.

`art::prepare(data, Limits) -> Result<Prepared, ArtError>`:
- valideren met `image::guess_format` op de magic bytes (AC #1), alleen JPEG en
  PNG; de groottegrens wordt vóór het decoderen getoetst (AC #5), zodat een
  enorme upload geen geheugen kost;
- verkleinen alleen omlaag, met `thumbnail` dat de beeldverhouding behoudt;
- transparantie wordt echt gemeten (een pixel met alpha < 255) en niet uit het
  alfakanaal afgeleid: een ondoorzichtig alfakanaal kost alleen bytes.

`Prepared` draagt de nieuwe afmetingen én de originele, zodat `is_resized()`
kan zeggen wat er gebeurd is — dat heeft task-21 nodig voor de melding.

Twee nieuwe omgevingsvariabelen met eigen parser en foutmelding, in de stijl
van de bestaande: `ART_QUALITY` (1–100, standaard 85; 0 en >100 worden
geweigerd in plaats van bijgeknipt) en `MAX_UPLOAD_MB` (standaard 10; 0 wordt
geweigerd omdat het elke upload zou blokkeren).

Een half afgekapte JPEG komt er soms doorheen: JPEG-decoders zijn tolerant en
maken er een deels grijs plaatje van. De test legt vast dat beide uitkomsten
goed zijn zolang er geen panic is en wat eruit komt leesbaar blijft — hetzelfde
uitgangspunt als de bestaande test voor `thumbnail`.

`art::` blijft dead_code toestaan: `prepare` wordt pas in task-21 aangeroepen,
maar is hier al volledig getest.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
De beeldverwerking als losstaande, testbare laag (FR-15): `art::prepare`
valideert op de bytes zelf (alleen JPEG en PNG, tot `MAX_UPLOAD_MB`), verkleint
alleen wat boven `MAX_ART_SIZE` uitkomt met behoud van de beeldverhouding, en
hercodeert alleen wat verkleind is — naar JPEG met kwaliteit `ART_QUALITY`,
tenzij er werkelijk doorzichtige pixels in zitten.

Past de afbeelding al binnen de grenzen, dan komen de bytes ongewijzigd terug:
geen hercodering, geen kwaliteitsverlies, en een PNG blijft een PNG. Dat is de
met de eigenaar bevestigde beslissing op de open vraag uit PRD §12, die nu als
besluit in het PRD staat in plaats van als open punt.

13 nieuwe unit-tests in `art::` (te grote JPEG, te grote PNG, PNG met en zonder
doorzichtige pixels, al passende afbeelding, beeldverhouding, instelbare
kwaliteit, geen afbeelding, GIF, half bestand, te grote upload), 2 in
`config::` en de nieuwe variabelen in de config-integratietest. `cargo fmt
--check`, `cargo clippy -- -D warnings` en `cargo test` (241 + overige) zijn
groen. Commit f4125be.
<!-- SECTION:FINAL_SUMMARY:END -->
