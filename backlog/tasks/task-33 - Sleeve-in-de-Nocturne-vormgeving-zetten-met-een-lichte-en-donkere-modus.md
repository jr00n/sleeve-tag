---
id: TASK-33
title: 'Sleeve in de Nocturne-vormgeving zetten, met een lichte en donkere modus'
status: Done
assignee:
  - '@claude'
created_date: '2026-08-30 06:32'
updated_date: '2026-08-30 06:43'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
modified_files:
  - static/app.css
  - static/app.js
  - templates/base.html
  - tests/weergave.rs
  - tests/rawtags.rs
  - src/web/mod.rs
  - README.md
  - CLAUDE.md
type: enhancement
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Er ligt een uitgewerkt ontwerp voor Sleeve in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`), gebouwd op het design system "Nocturne". De huidige interface heeft een eigen, losstaande kleurenset en knopstijl die daar niet op aansluit.

Doel is dat de bestaande schermen — maplijst, bewerkpagina, hoespagina, albumweergave, voorbeeld en resultaat — er uitzien zoals het ontwerp: de Nocturne-tokens (kleuren, tonenreeksen, ruimtematen, hoeken, schaduwen, typografie), de knop- en veldstijl uit dat systeem, kaarten met een schaduwrand in plaats van een lijn, uitvloeiende scheidingsregels, en labels in de accentkleur.

Het ontwerp toont daarnaast een expliciete keuze tussen licht en donker in de kopbalk; nu volgt Sleeve alleen de systeemvoorkeur. Die keuze hoort erbij en wordt onthouden in de browser.

Buiten scope: de interactiemodellen die het ontwerp laat zien maar Sleeve niet kent (gestapelde wijzigingen, een "needs attention"-filter, hulpacties die er nog niet zijn). Dit is een vormgevingsslag, geen functionele uitbreiding.

Randvoorwaarde uit CLAUDE.md: uitsluitend lokale assets. Het lettertype uit het ontwerp (Inter) mag niet van een CDN komen; de opmaak valt terug op de systeemletter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 De kleuren, tonenreeksen, ruimtematen, hoeken en schaduwen uit het Nocturne-design-system staan als tokens in `static/app.css` en worden overal gebruikt; er staan geen losse kleurwaarden meer in de componentregels.
- [x] #2 Knoppen, invoervelden, kaarten, labels, tabellen en meldingen volgen de vormgeving uit het ontwerp: een primaire knop is een omlijnde accentknop en geen gevulde vlakte, velden staan op het oppervlakvlak met een scheidingsrand, en scheidingsregels vloeien aan de uiteinden uit.
- [x] #3 De kopbalk toont de naam Sleeve met het ID3-label en een keuze tussen donker en licht; die keuze wordt in de browser onthouden en geldt bij de volgende pagina meteen, zonder dat het scherm eerst in de andere modus opflitst.
- [x] #4 Zonder een opgeslagen keuze volgt de pagina de systeemvoorkeur, en zonder JavaScript blijft de pagina volledig leesbaar en bruikbaar.
- [x] #5 Alle pagina's zijn in beide modi leesbaar: tekst, gedempte tekst, signaleringen en foutmeldingen hebben in licht en donker voldoende contrast.
- [x] #6 Er worden geen assets van buiten opgehaald; de pagina werkt op een NAS zonder internetverbinding.
- [x] #7 `cargo fmt --check`, `cargo clippy -- -D warnings` en `cargo test` zijn groen, en de bestaande test die afdwingt dat `hidden` echt verbergt blijft slagen.
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
1. `static/app.css` — de tokenlaag vervangen. Nocturne-tokens (`--color-bg`, `--color-surface`, `--color-text`, `--color-accent`, `--color-divider`, de neutrale en accent-tonenreeksen, `--space-*`, `--radius-*`, `--shadow-*`, `--font-*`) komen bovenaan; de bestaande Nederlandse namen (`--achtergrond`, `--vlak`, `--rand`, `--tekst`, `--tekst-zacht`, `--accent`) blijven bestaan als alias daarop, zodat de rest van het bestand meteen meeverandert zonder dat elke regel wordt aangeraakt.

2. Themalagen: donker op kale `:root`; licht onder `@media (prefers-color-scheme: light)` afgeschermd met `:root:not([data-theme="dark"])`, en daarnaast `:root[data-theme="light"]` en `:root[data-theme="dark"]` zodat een expliciete keuze in beide richtingen wint. De lichte waarden komen uit het ontwerp (`#eef0fa` / `#f8f9ff` / `#292b31` / `#5d5294`).

3. Componentregels bijstellen naar het ontwerp: kopbalk met een haarlijnschaduw in plaats van een rand, knoppen doorzichtig met een randkleur (primair = omlijnd accent), velden op het oppervlakvlak met accent-caret, lijsten en kaders met `--shadow-sm` in plaats van een lijn, scheidingsregels als uitvloeiend verloop (de Nocturne-signatuur), signaleringen en signaallabels in de accentkleur, tabelrijen met een uitvloeiende onderregel.

4. `templates/base.html` — de kopbalk krijgt het ID3-label naast de naam en een keuzeschakelaar donker/licht. Een klein inline-script in de `<head>` zet `data-theme` vóór het eerste renderen, zodat er niets opflitst.

5. `static/app.js` — de schakelaar aansluiten: klik zet `data-theme` op `<html>` en bewaart de keuze in `localStorage`; zonder opgeslagen keuze blijft de systeemvoorkeur gelden. Zonder JavaScript blijft alles leesbaar (de schakelaar staat er dan niet).

6. Geen enkele externe asset: Inter wordt alleen als eerste naam in de letterstapel genoemd, met een terugval op de systeemletter. Geen `@import` naar Google Fonts.

7. Tests: de bestaande assertie op `[hidden]` in `tests/busy.rs` blijft staan; er komt een assertie bij dat `app.css` beide themalagen bevat en dat de kopbalk de schakelaar rendert. Daarna de kwaliteitspoort draaien.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Twee bestaande tests gingen ervan uit dat de ruwe-tagpagina nergens een `<button>` bevat. Die assertie ging over de inhoud, maar keek naar de hele pagina; met een schakelaar in de kopbalk klopte dat niet meer. Beide tests kijken nu vanaf `<main>`, wat precies is wat ze bedoelden: op deze pagina valt aan dit bestand niets te veranderen.

De uitvloeiende scheidingsregel ligt als `::before` over een regel heen en niet als achtergrond. Als achtergrond zou hij winnen van de kleur die een regel zelf kan dragen — gekozen, gewijzigd, mislukt — en die kleur is belangrijker dan de scheiding.

In de batchtabel blijft de scheiding een gewone lijn. De eerste kolom staat daar stil terwijl de rest scrollt; een verloop dat aan de rand van de tabel uitvloeit, zou halverwege het scherm ophouden.

Gecontroleerd in de browser tegen een tijdelijke bibliotheek met fixtures: bibliotheek, mapweergave, albumweergave, bewerkpagina en hoespagina, in beide modi, met de keuze die over pagina's heen blijft staan.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Sleeve draagt nu de vormgeving van het design system Nocturne, en biedt een keuze tussen donker en licht.

De kleuren, tonenreeksen, ruimtematen, hoeken en schaduwen staan als tokens boven in `static/app.css`; daaronder staat geen enkele kleurwaarde meer. De bestaande Nederlandse namen (`--achtergrond`, `--vlak`, `--rand`, `--tekst`, `--tekst-zacht`, `--accent`) zijn aliassen op die tokens gebleven, zodat de hele stijl meeverandert zonder dat elke component is herschreven.

Wat er anders uitziet: een knop is een omlijning en geen gevuld vlak — ook de primaire, die zich onderscheidt door de accentkleur; kaders en lijsten dragen een schaduwrand in plaats van een lijn; scheidingsregels vloeien aan de uiteinden uit; signaleringen en hun labels staan in de accentkleur in plaats van in een waarschuwingskleur, want het zijn constateringen; en de velden in de albumtabel vallen pas op zodra de muis erboven staat, zodat de lijst een lijst blijft.

De kopbalk toont de naam met het ID3-label en de keuze tussen donker en licht. Die keuze staat in `localStorage` en wordt door een klein script in de `<head>` toegepast vóór het eerste renderen, zodat de pagina niet in de andere modus opflitst. Zonder keuze geldt de systeemvoorkeur; zonder JavaScript staat de schakelaar er niet en blijft alles verder werken.

Er wordt niets van buiten gehaald: Inter staat alleen als eerste naam in de letterstapel, met terugval op de systeemletter.

`tests/weergave.rs` legt vast dat de kopbalk de keuze verborgen aanbiedt, dat het script vóór de stylesheet staat, dat de stijl beide richtingen kent, en dat pagina, stijl en script nergens naar buiten verwijzen.
<!-- SECTION:FINAL_SUMMARY:END -->
