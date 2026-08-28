---
id: TASK-32
title: 'Album art voor een selectie bestanden, via de albumweergave'
status: To Do
assignee: []
created_date: '2026-08-28 21:31'
updated_date: '2026-08-28 21:31'
labels: []
milestone: m-6
dependencies:
  - TASK-31
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
- [ ] #1 In de albumweergave kan een hoes worden klaargezet die geldt voor de aangevinkte bestanden, met slepen en met de bestandskiezer
- [ ] #2 De voorbeeldweergave toont per geselecteerd bestand wat er met de hoes zou gebeuren: toevoegen of vervangen, en wat er nu in zit
- [ ] #3 Er wordt niets geschreven voordat op Definitief opslaan is gedrukt; de albumweergave opent nog steeds geen bestand
- [ ] #4 Het rapport meldt per bestand of de hoes geschreven is, in dezelfde lijst als de tagwijzigingen
- [ ] #5 Bestanden die niet zijn aangevinkt blijven onaangeraakt, ook als ze in dezelfde map staan
- [ ] #6 De losse cover.jpg kan meegeschreven worden, met dezelfde bevestiging bij een bestaand bestand als op de hoespagina
- [ ] #7 Een afbeelding die te groot is of geen JPEG/PNG, wordt in de browser tegengehouden vóór het versturen, net als elders
- [ ] #8 Een gekozen afbeelding gaat niet verloren wanneer de selectie of een veld wordt aangepast, of de gebruiker weet waarom hij hem opnieuw moet kiezen
- [ ] #9 Een fout bij één bestand houdt de rest niet tegen, zoals bij de tagbatch
- [ ] #10 De bestaande routes voor één bestand en voor de hele map blijven werken zoals ze deden
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
