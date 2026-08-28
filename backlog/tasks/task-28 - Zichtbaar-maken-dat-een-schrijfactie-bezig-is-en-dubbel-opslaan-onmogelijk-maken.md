---
id: TASK-28
title: >-
  Zichtbaar maken dat een schrijfactie bezig is, en dubbel opslaan onmogelijk
  maken
status: In Progress
assignee:
  - claude
created_date: '2026-08-28 20:08'
updated_date: '2026-08-28 20:13'
labels: []
milestone: m-5
dependencies: []
modified_files:
  - static/app.js
  - static/app.css
  - templates/base.html
  - templates/edit.html
  - templates/cover.html
  - templates/albumpreviewform.html
  - tests/busy.rs
  - README.md
priority: high
type: enhancement
ordinal: 24500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Op de NAS duurde één titelwijziging in een FLAC van 3,3 GB ruim twee minuten (gemeten 2026-08-28: submit rond 19:50:5x, `bestand geschreven` om 19:52:53). Dat komt niet door traagheid in de app maar door de omvang van het bestand: `atomic::replace` kopieert het volledige bestand, en lofty moest de hele FLAC-stream herschrijven omdat er geen PADDING-blok in zat.

In die twee minuten geeft de UI geen enkel teken van leven. De pagina staat stil, de knop ziet er onveranderd uit, en niets zegt of de app bezig is of vastzit. Erger: er is niets dat een tweede klik op "Opslaan" tegenhoudt, en dat start een tweede schrijfactie op hetzelfde bestand terwijl de eerste nog loopt.

Deze taak lost de zichtbaarheid op, niet de duur. Het versnellen van de kopie (reflink op btrfs) is een aparte afweging en hoort niet hier.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bij een schrijfactie toont de aangeklikte knop dat hij bezig is, met een spinner en een tekst die bij die actie past
- [x] #2 Zolang een schrijfactie loopt, leveren verdere klikken op een knop in datzelfde formulier geen tweede verzoek op
- [x] #3 Er staat een melding bij dat een groot bestand een paar minuten kan duren, zodat wachten geen giswerk is
- [x] #4 Alleen schrijfacties krijgen deze behandeling; hulpacties als hernummeren, selecteren en annuleren blijven meteen reageren
- [x] #5 De knopwaarde (name=actie) wordt nog steeds meegestuurd — uitschakelen mag de inhoud van het verzoek niet veranderen
- [x] #6 Zonder JavaScript blijft het formulier gewoon werken; de bezig-weergave is een toevoeging en geen voorwaarde
- [ ] #7 Terugkeren naar de pagina met de terug-knop van de browser laat geen uitgeschakelde knop achter
- [x] #8 De spinner respecteert prefers-reduced-motion
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

Progressive enhancement in gewoon JavaScript, geen htmx. De formulieren die
schrijven zijn normale POST-formulieren; htmx zit alleen op de vinkjes in de
albumweergave. Een nieuw bestand `static/app.js` (net als `htmx.min.js` lokaal
geserveerd — de NAS heeft geen internet nodig) hangt één luisteraar op
`submit` en doet de rest.

**Alleen schrijfacties.** De knop die schrijft krijgt in de template een
`data-bezig="…"` met de tekst die hij tijdens het werk toont. Hulpacties
(hernummeren, selecteren, annuleren, voorbeeld) krijgen dat attribuut niet en
blijven zich gedragen zoals nu. Zo staat op één plek — in de template, naast de
knop zelf — of een actie traag genoeg is om te melden, en hoeft de JS geen
namen van acties te kennen.

**De valkuil: `name="actie"`.** Meerdere formulieren sturen mee wélke knop is
ingedrukt. Een knop uitschakelen vóórdat de browser het formulier serialiseert,
laat die waarde wegvallen en zou van "Definitief opslaan" een verzoek zonder
actie maken. Het uitschakelen gebeurt daarom in een `setTimeout(…, 0)`: dan is
het verzoek al samengesteld.

**Dubbel opslaan.** Het formulier krijgt een markering zodra het bezig is; een
volgende `submit` op datzelfde formulier wordt geweigerd. Dat vangt ook Enter in
een tekstveld af, niet alleen een tweede klik.

**Terugkeren met de terug-knop.** Firefox en Safari halen een pagina uit de
bfcache mét de uitgeschakelde knop erin. Een `pageshow`-luisteraar zet alles
terug.

## Stappen

1. `static/app.js` — de luisteraar, de bezig-staat, het herstel bij `pageshow`.
2. `static/app.css` — spinner (met `prefers-reduced-motion`), stijl voor een
   uitgeschakelde knop, en de regel met de waarschuwing over grote bestanden.
3. `templates/base.html` — `app.js` inladen, `defer` net als htmx.
4. `templates/edit.html`, `cover.html`, `albumpreviewform.html` — `data-bezig`
   op de knoppen die schrijven; de overige knoppen blijven ongemoeid.
5. `tests/busy.rs` — de binary serveert `app.js`, de schrijvende knoppen dragen
   het attribuut en de hulpacties niet.
6. README: een regel over waarom een grote FLAC minuten kost.

## Buiten deze taak

De duur zelf. Dat een bestand van 3,3 GB volledig gekopieerd wordt, volgt uit
de atomische schrijfstrategie; sneller maken (reflink) is een eigen afweging.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Uitgevoerd als progressive enhancement in `static/app.js`, lokaal geserveerd net als htmx. Één luisteraar op `submit`: is het formulier al bezig, dan gaat het verzoek niet uit; anders krijgt de aangeklikte knop een spinner met de tekst uit zijn eigen `data-bezig`, en verschijnt onder het formulier een statusregel over de duur.

De markering staat in de template naast de knop en niet in de JS. Zo bepaalt de template welke acties traag genoeg zijn om te melden, en hoeft het script geen enkele actienaam te kennen. Gemarkeerd: opslaan (bewerkformulier), embedden en verwijderen (hoespagina), definitief opslaan (batch). Niet gemarkeerd: selecteren, hernummeren, artiest → albumartiest, hoofdletters, invoer leegmaken, annuleren, voorbeeld.

De valkuil zat in `name="actie"`: een knop die uitgeschakeld wordt vóórdat de browser het formulier serialiseert, stuurt zijn waarde niet mee — dan zou 'Definitief opslaan' als verzoek zónder actie aankomen. Het uitschakelen gebeurt daarom in een `setTimeout(…, 0)`. Bij de batchformulieren speelt dat niet, want die gaan via htmx en dat serialiseert al in de submit-handler van het formulier zelf, dus vóór de document-luisteraar.

AC #7 (terug-knop) is in code afgehandeld met een `pageshow`-luisteraar op `event.persisted`, die de klasse, de uitgeschakelde knoppen, de melding en het oorspronkelijke opschrift (bewaard in `data-was`) terugzet. Niet in een echte browser geverifieerd: de Chrome-extensie was in deze sessie niet verbonden. Het blijft daarom als enige criterium open tot de eigenaar het op de NAS heeft geprobeerd.

Getest met `tests/busy.rs` — zes tests over de echte binary: het script wordt lokaal geserveerd, elke pagina laadt het, de schrijvende knoppen dragen de markering met hun tekst, en de hulpacties juist niet. Wat in de browser gebeurt valt buiten het bereik van een test zonder browser; `node --check` bevestigt in elk geval de syntaxis.
<!-- SECTION:NOTES:END -->
