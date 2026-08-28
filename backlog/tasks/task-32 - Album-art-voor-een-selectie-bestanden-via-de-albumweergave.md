---
id: TASK-32
title: 'Album art voor een selectie bestanden, via de albumweergave'
status: Done
assignee:
  - claude
created_date: '2026-08-28 21:31'
updated_date: '2026-08-28 21:52'
labels: []
milestone: m-6
dependencies:
  - TASK-31
modified_files:
  - src/batch.rs
  - src/web/mod.rs
  - static/app.css
  - templates/albumpreviewform.html
  - tests/album.rs
  - README.md
  - CLAUDE.md
priority: medium
type: feature
ordinal: 24900
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Een hoes kan nu in één bestand of in álle bestanden van een map. Wat ontbreekt is het geval ertussenin: de bestanden die je hebt aangevinkt.

Het aanleidende geval van de eigenaar: een dubbelalbum waarvan beide discs in één map staan. Je vinkt de eerste helft aan en zet daar disc 1 op, dan de tweede helft met disc 2 — dat werkt al voor tags. Maar voor de hoes is de map ineens weer de eenheid, terwijl je net hebt vastgesteld dat die map twee groepen bevat. Soms hebben die groepen ook werkelijk een andere hoes: uitgaves waarbij disc 2 een eigen kant heeft.

De selectie bestaat al en zit in de albumweergave (`/album/…`), samen met de gedeelde velden en de voorbeeldweergave. De hoes hoort daar dus ook thuis, als gedeeld veld naast album en albumartiest.

De onderdelen zijn er allemaal: `art::prepare` valideert en verkleint, `tags::write_art` schrijft gericht de front cover, `atomic::place` zet de losse `cover.jpg` neer, en `batch::` kent al een plan-en-voorbeeld-flow met een rapport per bestand. Wat er niet is, is een plan dat naast tags ook een afbeelding kan dragen.

Dit is de eerste keer dat `batch::` iets in zijn plan krijgt dat geen tekst is. De regel dat de albumweergave alleen voorstelt en niets opent of schrijft, blijft gelden: de bytes komen uit het formulier, `batch::preview` beschrijft wat ermee zou gebeuren, en pas `actie=opslaan` schrijft — bestand voor bestand, met hervalidatie per bestand zoals nu.

Aandachtspunt bij het ontwerp: de albumweergave gaat via htmx en post zichzelf bij elk vinkje opnieuw. Een gekozen afbeelding overleeft zo'n ronde niet vanzelf — een bestandsveld is niet te vullen vanuit de server. Dat moet een bewuste keuze worden en geen verrassing: ofwel de afbeelding wordt pas in de laatste stap gevraagd, ofwel ze wordt in de browser vastgehouden en opnieuw aan het formulier gehangen.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 In de albumweergave kan een hoes worden klaargezet die geldt voor de aangevinkte bestanden, met slepen en met de bestandskiezer
- [x] #2 De voorbeeldweergave toont per geselecteerd bestand wat er met de hoes zou gebeuren: toevoegen of vervangen, en wat er nu in zit
- [x] #3 Er wordt niets geschreven voordat op Definitief opslaan is gedrukt; de albumweergave opent nog steeds geen bestand
- [x] #4 Het rapport meldt per bestand of de hoes geschreven is, in dezelfde lijst als de tagwijzigingen
- [x] #5 Bestanden die niet zijn aangevinkt blijven onaangeraakt, ook als ze in dezelfde map staan
- [x] #6 De losse cover.jpg kan meegeschreven worden, met dezelfde bevestiging bij een bestaand bestand als op de hoespagina
- [x] #7 Een afbeelding die te groot is of geen JPEG/PNG, wordt in de browser tegengehouden vóór het versturen, net als elders
- [x] #8 Een gekozen afbeelding gaat niet verloren wanneer de selectie of een veld wordt aangepast, of de gebruiker weet waarom hij hem opnieuw moet kiezen
- [x] #9 Een fout bij één bestand houdt de rest niet tegen, zoals bij de tagbatch
- [x] #10 De bestaande routes voor één bestand en voor de hele map blijven werken zoals ze deden
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
## De vraag die het ontwerp bepaalt: waar komt de afbeelding het formulier in?

De albumweergave post zichzelf bij élk vinkje opnieuw, via htmx. Zou het
bestandsveld daar staan, dan reisde de hele afbeelding mee bij iedere klik —
bij een hoes van een paar megabyte is dat onwerkbaar, en een bestandsveld is
door de server ook niet terug te vullen, dus na de eerste ronde zou hij weg
zijn.

**Daarom komt de hoes in de voorbeeldweergave.** Dat is de enige stap die
rechtstreeks naar het schrijven leidt: de afbeelding reist precies één keer, op
het moment dat er ook werkelijk iets mee gebeurt. Het past bovendien bij wat die
pagina is — het plan dat je goedkeurt.

Dat maakt AC #2 ook haalbaar zonder de afbeelding te kennen: of een bestand een
hoes *krijgt* of dat de bestaande *vervangen* wordt, volgt uit wat er nu in zit.
Dat weet de server al.

En AC #8 wordt er triviaal van: de afbeelding steekt geen enkele
htmx-ronde over, dus hij kan onderweg ook niet verdwijnen.

## Stappen

1. `batch::Form` krijgt de twee keuzes die de hoespagina ook kent
   (`mapbestand`, `overschrijf`), en `parse` wordt gesplitst zodat dezelfde
   invoer ook uit een multipart-formulier kan komen.
2. `web::album_selection` neemt voortaan het hele verzoek aan en kijkt naar het
   content-type: urlencoded zoals nu, of multipart met de afbeelding erbij. Eén
   route, twee vormen.
3. `batch::FileDiff` krijgt erbij of het bestand geselecteerd is en wat er nu
   aan hoes in zit. De voorbeeldweergave toont per bestand "hoes wordt
   toegevoegd" of "hoes wordt vervangen (nu JPEG 600×600)".
4. `templates/albumpreviewform.html` krijgt `enctype="multipart/form-data"` en
   het neerzetvak uit TASK-30, met de grens uit `MAX_UPLOAD_MB` en het vinkje
   voor de losse `cover.jpg`.
5. Het schrijven: per geselecteerd bestand eerst de tags zoals nu, daarna
   `tags::write_art`. Beide leveren regels in hetzelfde rapport. Een fout bij
   het ene bestand houdt de rest niet tegen. De losse `cover.jpg` komt ná de
   bestanden, met een eigen regel — zoals op de hoespagina.

## Wat blijft zoals het was

Zonder afbeelding gedraagt de batch zich precies zoals nu. De routes voor één
bestand en voor de hele map blijven bestaan; deze weg komt ernaast en vervangt
niets.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Twee dingen kwamen pas bij het bouwen boven water.

Ten eerste liet `batch::intents` een aangevinkt bestand vallen zodra er niets aan zijn vélden veranderde — terecht, want dan valt er niets te schrijven. Met een hoes erbij ligt dat anders: die geldt voor de hele selectie, ook voor een bestand waarvan de tags al kloppen. Vandaar `intents_with_selection`, dat alleen gebruikt wordt als er werkelijk een afbeelding meekomt.

Ten tweede stond in het voorbeeld bij zo'n bestand 'Blijft ongewijzigd; dit bestand wordt niet aangeraakt'. Dat werd onwaar op het moment dat er een hoes bij kon. Nu staat er 'Blijft ongewijzigd op de tags; wordt alleen aangeraakt als je hieronder een hoes kiest'. Een bestaande test hield me daar netjes aan.

De opslaanknop verschijnt voortaan ook zonder tagwijziging, zolang er iets is aangevinkt: anders zou een hoes-alleen-batch onbereikbaar zijn.

`album_selection` neemt nu het hele verzoek aan en kijkt naar het content-type. Urlencoded voor elke ronde van de albumtabel, multipart alleen voor de voorbeeldstap. Eén route, twee vormen — en `batch::Form::from_pairs` leest beide.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Een hoes kan nu in precies de bestanden die je hebt aangevinkt, via de voorbeeldweergave van een batch.

**Waar de afbeelding het formulier in komt, bepaalde het ontwerp.** De albumtabel post zichzelf bij élk vinkje opnieuw; een bestandsveld daar zou de afbeelding bij iedere klik meesturen, en de server kan zo'n veld daarna niet terugvullen — na één ronde was hij weg. De hoes hoort daarom bij de voorbeeldweergave: de enige stap die rechtstreeks naar het schrijven leidt. De afbeelding reist precies één keer, op het moment dat er ook werkelijk iets mee gebeurt, en kan onderweg dus niet verdwijnen (AC #8).

Dat het voorbeeld tóch per bestand kan zeggen wat er gebeurt (AC #2), komt doordat "toevoegen of vervangen" volgt uit wat er nu in het bestand zit — niet uit de nieuwe afbeelding. Bij een aangevinkt bestand staat er: *"Kies je hieronder een hoes: hoes wordt vervangen (nu JPEG 600×600)."*

**Eén route, twee vormen.** `album_selection` kijkt naar het content-type: urlencoded voor elke ronde van de tabel, multipart alleen voor de voorbeeldstap. `batch::Form::from_pairs` leest beide, zodat de velden maar op één plek uitgelezen worden.

**Het schrijven** gaat bestand voor bestand zoals het al deed: eerst de tags, dan de hoes, allebei met hun eigen regel in hetzelfde rapport. Een fout bij het ene bestand houdt de rest niet tegen. De losse `cover.jpg` komt ná de bestanden, met dezelfde bevestiging bij een bestaand bestand als op de hoespagina. Een afbeelding die niet deugt, houdt de hele batch tegen — ook de tags, want half uitvoeren van een plan dat niet klopt is erger dan niets doen.

**Getest** met vijf nieuwe integratietests: het vak verschijnt met het juiste aantal, een niet-aangevinkt bestand krijgt geen hoesregel én blijft byte-voor-byte gelijk, tags en hoes komen in één ronde en in één rapport, de losse `cover.jpg` verschijnt alleen met het vinkje, en een batch zónder afbeelding gedraagt zich precies zoals voorheen.
<!-- SECTION:FINAL_SUMMARY:END -->
