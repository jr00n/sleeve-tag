---
id: TASK-30
title: Een hoes kunnen neerslepen op de hoespagina
status: In Progress
assignee:
  - claude
created_date: '2026-08-28 20:21'
updated_date: '2026-08-28 20:46'
labels: []
milestone: m-6
dependencies: []
modified_files:
  - static/app.js
  - static/app.css
  - templates/cover.html
  - tests/art.rs
  - README.md
priority: medium
type: enhancement
ordinal: 24700
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Een hoes uploaden gaat nu via `<input type="file">` op de hoespagina: bladeren, in een bestandskiezer zoeken, openen. Wie een afbeelding al in een venster ernaast heeft staan — een browsertab met de hoes, de Finder, een map met scans — wil hem er gewoon op kunnen slepen.

Het gaat om een toevoeging op het bestaande formulier (`templates/cover.html`), niet om een nieuwe route: het slepen vult hetzelfde bestandsveld dat er al is. De keuzevakjes eromheen (`mapbestand`, `overschrijf`) en de knoppen "Alleen dit bestand" / "alle tracks" blijven bepalen wat er met de afbeelding gebeurt. Er wordt dus nog steeds niets geschreven zonder dat de gebruiker een knop indrukt.

De server hoeft niets nieuws te kunnen: een neergesleept bestand komt via `DataTransfer` in dezelfde `<input type="file">` terecht en gaat als hetzelfde multipart-formulier de deur uit. `art::prepare` valideert al op de bytes zelf, dus een gesleept bestand krijgt precies dezelfde behandeling als een gekozen bestand — ook wanneer het geen JPEG of PNG blijkt.

Sluit aan bij de bezig-weergave uit TASK-28: ook hier hoort zichtbaar te zijn dat er gewerkt wordt zodra er wordt ingediend.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Een afbeelding op het uploadvak slepen vult het bestaande bestandsveld, zodat het formulier verder ongewijzigd werkt
- [x] #2 Tijdens het slepen laat het vak zien dat het bestand daar losgelaten kan worden
- [x] #3 Na het neerzetten is te zien welk bestand gekozen is, met naam en een voorbeeldweergave
- [x] #4 Slepen voegt niets toe zonder klik: het uploaden gebeurt nog steeds pas bij Alleen dit bestand of alle tracks
- [x] #5 Iets anders dan een afbeelding neerzetten geeft meteen een melding in plaats van een mislukte upload
- [x] #6 Meerdere bestanden tegelijk neerzetten neemt er niet stilzwijgend een van; het zegt dat er een hoes tegelijk gaat
- [x] #7 Zonder JavaScript blijft het bestandsveld gewoon werken; slepen is een toevoeging en geen voorwaarde
- [x] #8 Op een aanraakscherm verandert er niets ten opzichte van nu
- [x] #9 Een bestand ergens anders op de pagina laten vallen opent het niet in het browservenster
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

Een toevoeging op het bestaande veld, in hetzelfde `static/app.js` dat sinds
TASK-28 al bestaat. Geen nieuwe route, geen serverwijziging: een neergesleept
bestand komt via `DataTransfer` in dezelfde `<input type="file">` terecht en
gaat als hetzelfde multipart-formulier de deur uit. `art::prepare` valideert
sowieso op de bytes zelf.

**De hint staat `hidden` in de template.** `app.js` haalt hem tevoorschijn. Zo
ziet iemand zonder JavaScript geen uitnodiging om te slepen die nergens toe
leidt, en blijft het veld doen wat het deed. De stippellijn hangt in CSS aan
diezelfde hint (`:has(.neerzetvak__hint:not([hidden]))`), zodat het vak zonder
JavaScript ook visueel niets belooft.

**De voorbeeld-`<img>` wordt door JS gemaakt**, niet door de template. Een lege
`<img>` meesturen zou een bestand zonder hoes een afbeelding geven die er nooit
komt — en een bestaande test bewaakt precies dat.

**Wat er niet doorheen komt.** Meer dan één bestand: melding, en niets
ingevuld — stilzwijgend de eerste pakken zou de gebruiker een andere afbeelding
geven dan hij dacht neer te zetten. Iets anders dan JPEG of PNG: melding met het
aangetroffen type. Een sleepactie zonder bestand (tekst, een link): ook een
melding.

**Naast het vak neerzetten** wordt op `window` afgevangen; anders opent de
browser het bestand en is de pagina met het half ingevulde formulier weg. Alleen
wanneer er ook werkelijk een neerzetvak op de pagina staat.

## Wat een test zonder browser kan vaststellen

Dat het vak er is, de hint verborgen begint, en het bestandsveld onveranderd is
(`tests/art.rs`). Het sleepgedrag zelf draait in de browser en wordt door de
eigenaar lokaal geprobeerd; `node --check` bewaakt in elk geval de syntaxis.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Slepen vult het bestaande bestandsveld en verder niets: de vinkjes en de knoppen bepalen nog steeds wát er gebeurt, en er wordt niets geschreven voordat er op een knop is gedrukt. Dat is dezelfde regel als bij de hulpacties in de albumweergave.

De bestaande test `a_file_without_art_says_so_on_its_cover_page` sloeg aan op mijn eerste versie: die bewaakt dat een bestand zónder hoes geen enkele `<img>` krijgt, en ik had een lege voorbeeld-`<img>` in de template gezet. Terecht — de voorbeeldweergave wordt nu door `app.js` gemaakt op het moment dat er iets te tonen valt.

Niet in een echte browser geverifieerd: de Chrome-extensie is in deze sessie niet verbonden. Er draait wel een verse instantie op http://localhost:18090 met een testbibliotheek (nooit de echte), zodat de eigenaar het slepen zelf kan proberen.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Een hoes kan op de hoespagina worden neergesleept. Het slepen vult het bestandsveld dat er al stond; alles daarna — de vinkjes, de knoppen, `art::prepare`, het atomisch schrijven — is ongewijzigd.

**Wat er gebeurt.** Boven het vak zweven zet er een rand omheen. Loslaten vult het veld en toont een miniatuur met de bestandsnaam en de omvang. Kiezen via de bestandskiezer doet nu hetzelfde, want ook dan hoor je te zien wát je gekozen hebt.

**Wat er niet gebeurt.** Er wordt niets geüpload: opslaan blijft een knop. Meerdere bestanden tegelijk levert een melding op en géén stilzwijgende keuze voor de eerste. Iets anders dan JPEG of PNG wordt meteen gemeld, met het aangetroffen type erbij, in plaats van pas na een mislukte upload. En een bestand dat naast het vak belandt, opent de browser niet meer — dat zou de pagina met het half ingevulde formulier wegvagen.

**Zonder JavaScript verandert er niets.** De uitnodiging om te slepen staat `hidden` in de template en komt pas tevoorschijn als het script draait; de stippellijn hangt in CSS aan diezelfde hint. Wie geen JavaScript heeft, ziet de bestandsinvoer zoals altijd — en geen belofte die niet waargemaakt wordt. Op een aanraakscherm vuren de sleepgebeurtenissen simpelweg niet.

**Getest** met `tests/art.rs`: het vak is er, de hint begint verborgen, en het bestandsveld heeft nog steeds dezelfde naam, hetzelfde type en dezelfde `accept`. Het sleepgedrag zelf draait in de browser; dat valt buiten het bereik van een test zonder browser.
<!-- SECTION:FINAL_SUMMARY:END -->
