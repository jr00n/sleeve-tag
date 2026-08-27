---
id: TASK-6
title: Veilige padafhandeling binnen MUSIC_ROOT
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:23'
updated_date: '2026-08-27 05:00'
labels: []
milestone: m-1
dependencies:
  - TASK-2
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De app krijgt paden binnen via URL's en formulieren en schrijft in de echte muziekbibliotheek. Zonder een strikte, centrale padcontrole is path traversal mogelijk en kan de app buiten de share schrijven. Alle latere fasen (browsen, bewerken, art) moeten via deze module lopen.

Regels uit het PRD: elk binnenkomend pad wordt gecanonicaliseerd (`std::fs::canonicalize`) en gecontroleerd tegen `MUSIC_ROOT`; symlinks die buiten de root wijzen worden geweigerd; navigatie boven de root is onmogelijk. Alleen bestanden met extensie `.mp3` of `.flac` en een herkend containerformaat gelden als bewerkbaar.

Deze module is de enige plek waar een door de gebruiker aangeleverd pad naar een filesystem-pad wordt omgezet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Een functie zet een door de gebruiker aangeleverd relatief pad om naar een gevalideerd absoluut pad binnen MUSIC_ROOT, of geeft een fout
- [x] #2 Pogingen met `..`, absolute paden, of een symlink die buiten de root wijst worden geweigerd
- [x] #3 Er is een aparte controle die bepaalt of een bestand bewerkbaar is (extensie .mp3/.flac én herkend containerformaat)
- [x] #4 Foutgevallen leveren een fouttype op dat de webserver kan vertalen naar HTTP 400/403/404 zonder het absolute pad te lekken
- [x] #5 Unit-tests dekken: geldig pad, traversal via `..`, absoluut pad, symlink binnen de root (toegestaan), symlink buiten de root (geweigerd), en een bestand met verkeerde extensie
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

`src/fs.rs` bevat alleen een doc-comment. `config` levert `music_root` al gecanonicaliseerd aan, dus deze module kan die
root als betrouwbaar anker gebruiken. `web` heeft een `WebError` die fouten naar een HTTP-respons vertaalt; daar komt een
variant bij.

## Ontwerp

Een `Bibliotheek`-struct die de gecanonicaliseerde root vasthoudt en de enige poort is van gebruikerspad naar
filesystem-pad:

- `resolveer(relatief)` — valideert en canonicaliseert, of geeft een fout.
- `resolveer_bewerkbaar_bestand(relatief)` — idem, plus de eis dat het een bewerkbaar audiobestand is.
- `is_bewerkbaar(pad)` — extensie `.mp3`/`.flac` (hoofdletterongevoelig) én een herkend containerformaat.
- `relatief_pad(absoluut)` — het pad terug naar de vorm die de UI mag tonen, zodat er nooit een absoluut pad in de
  interface belandt.

Validatie in twee lagen, bewust dubbelop:

1. **Vóór canonicalisatie**: componenten die absoluut zijn (`RootDir`, `Prefix`) of omhoog wijzen (`ParentDir`) worden
   meteen geweigerd. Dat vangt de aanval af zonder het filesystem aan te raken.
2. **Na canonicalisatie**: `std::fs::canonicalize` volgt symlinks; het resultaat moet met de root beginnen. Zo wordt een
   symlink die de bibliotheek uit wijst alsnog geweigerd, terwijl een symlink binnen de root gewoon werkt.

## Fouttype

`PadFout` met drie varianten: `BuitenBibliotheek`, `NietGevonden`, `NietOndersteund`. De `Display`-teksten bevatten
bewust géén pad — die melding gaat naar de browser. Het volledige pad hoort in het log, niet in de respons.

`web::WebError` krijgt een `From<PadFout>` die ze vertaalt naar 403, 404 en 415. Daarmee is acceptatiecriterium #4 niet
alleen "geschikt" maar ook echt gebruikt.

## Containerformaat via `tags::`

De architectuurregel is dat `lofty` uitsluitend in `tags::` wordt aangeroepen, en de architectuurtest dwingt dat af. Voor
de inhoudscontrole komt er daarom een kleine functie `tags::herkent_formaat(pad)`; `fs::` roept die aan. De rest van
`tags::` (het lezen van het tagmodel) is de volgende taak.

## Tests

Unit-tests op een tempdir met gekopieerde fixtures: geldig bestand en geldige map, lege invoer die naar de root wijst,
traversal met `..`, absoluut pad, symlink binnen de root (toegestaan), symlink naar buiten (geweigerd), niet-bestaand
pad, verkeerde extensie, een `.mp3` met onzininhoud (juiste extensie maar geen geldig containerformaat), en een test die
controleert dat geen enkele foutmelding het absolute pad bevat.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
De inhoudscontrole bleek strenger te moeten dan gedacht. `Probe::guess_file_type()` van lofty valt terug op de bestandsextensie, en de JPEG-signatuur FF D8 lijkt genoeg op een MPEG frame-sync: `tests/fixtures/cover.jpg` werd als Mpeg geraden. Pas bij het uitlezen van de audio-eigenschappen viel door de mand dat er geen geldige frames in zitten ('failed to parse Mpeg file'). Die stap is dus geen luxe maar de eigenlijke controle. Nagemeten met een wegwerp-voorbeeld dat daarna weer is verwijderd.

Waarom dit belangrijk is: zonder die controle zou een JPEG die als track.mp3 in de bibliotheek staat als bewerkbaar worden getoond, en zou de app er straks tags in schrijven. Dat raakt de dataverlies-eis uit het PRD.

Validatie gebeurt in twee lagen. Eerst worden padcomponenten geweigerd die absoluut zijn of omhoog wijzen, zonder het filesystem aan te raken. Daarna volgt canonicalisatie en de controle of het resultaat nog met de root begint. Alleen die tweede laag vangt een symlink die de bibliotheek uit wijst; alleen de eerste vangt `..` zonder I/O. De test voor de symlink bevat expliciet de opmerking dat hij uitsluitend door de tweede laag wordt gevangen.

PadFout wordt in web:: vertaald naar 403 (buiten de bibliotheek), 404 (niet gevonden) en 415 (niet ondersteund). Bewust 403 en niet 404 voor een pad buiten de root, zodat een geweigerd verzoek in de logs te onderscheiden is van een dode link. Een test controleert dat geen enkele foutmelding een pad-achtige tekst bevat, want die melding gaat naar de browser.

AppState bevat nu de Bibliotheek in plaats van de hele Config. Config had na deze wijziging geen enkele lezer meer; velden als max_art_size komen terug zodra de taken die ze gebruiken er zijn.

`fs` heeft tijdelijk een module-brede `allow(dead_code)`: `resolveer` en `is_bewerkbaar` worden pas door de mapbrowser-taak aangeroepen. Die regel hoort daar weg te gaan.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Wat er is gebouwd

De enige poort tussen een door de gebruiker aangeleverd pad en het filesystem. Elke latere handler — browsen, bewerken, art — gaat hierdoorheen; zonder deze module is één vergeten controle genoeg om buiten de muziekbibliotheek te lezen of te schrijven.

## Wijzigingen

- **`src/fs.rs`**: `Bibliotheek` met `resolveer`, `resolveer_bewerkbaar_bestand`, `relatief_pad` en `is_bewerkbaar`, plus het fouttype `PadFout`.
- **`src/tags/mod.rs`**: `herkent_formaat`, de enige plek waar naar het containerformaat wordt gekeken — lofty blijft binnen `tags::`, zoals de architectuurtest afdwingt.
- **`src/web/mod.rs`**: `WebError` vertaalt `PadFout` naar 403, 404 en 415; `AppState` draagt nu de bibliotheek.

## Twee lagen validatie, bewust dubbelop

1. **Vóór canonicalisatie** worden componenten geweigerd die absoluut zijn of omhoog wijzen. Vangt `..` zonder het filesystem aan te raken.
2. **Ná canonicalisatie** moet het resultaat nog met de root beginnen. Dit is de enige laag die een symlink vangt die de bibliotheek uit wijst — daar staat geen `..` in, dus de eerste laag laat hem door.

De testsuite legt dat onderscheid expliciet vast, zodat iemand die later een laag "overbodig" vindt ziet waarom beide er zijn.

## Wat de tests boven water haalden

`Probe::guess_file_type()` van lofty valt terug op de bestandsextensie, en de JPEG-signatuur `FF D8` lijkt genoeg op een MPEG frame-sync om als MP3 door te gaan: `cover.jpg` werd als Mpeg geraden, ook zonder `.mp3`-naam. Pas het uitlezen van de audio-eigenschappen laat zien dat er geen geldige frames in zitten.

Zonder die extra stap zou een JPEG die als `track.mp3` in de bibliotheek staat als bewerkbaar worden getoond — en zou de app er straks tags in schrijven. Dat raakt rechtstreeks de dataverlies-eis uit het PRD.

## Tests

52 groen (was 41); elf nieuwe:

- geldig bestand, geldige map, en lege invoer die naar de root wijst
- traversal met `..` in vijf varianten, en absolute paden
- symlink binnen de bibliotheek (toegestaan) en naar buiten (geweigerd)
- niet-bestaand pad, verkeerde extensie, map in plaats van bestand
- een JPEG met `.mp3`-extensie: juiste naam, verkeerde inhoud
- hoofdletterongevoelige extensiecontrole
- geen enkele foutmelding bevat een pad-achtige tekst

## Openstaand

`fs` heeft een tijdelijke module-brede `allow(dead_code)`: `resolveer` en `is_bewerkbaar` worden pas door de mapbrowser aangeroepen. Die regel hoort in die taak te verdwijnen.
<!-- SECTION:FINAL_SUMMARY:END -->
