---
id: TASK-16
title: Per-bestand overrides voor titel en tracknummer in de batch-tabel (FR-9)
status: Done
assignee: []
created_date: '2026-08-26 22:24'
updated_date: '2026-08-28 06:01'
labels: []
milestone: m-3
dependencies:
  - TASK-15
documentation:
  - PRD.md
modified_files:
  - src/batch.rs
  - templates/album.html
  - templates/albumform.html
  - static/app.css
  - tests/album.rs
  - README.md
  - CLAUDE.md
priority: medium
type: feature
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Bij het corrigeren van een album zijn album en albumartiest gedeeld, maar titel en tracknummer per bestand verschillend. Die twee moeten daarom inline in dezelfde tabel te bewerken zijn, zonder dat de gebruiker per track naar een apart formulier moet.

De overrides gaan mee in dezelfde diff-preview en dezelfde opslagronde als de gedeelde velden.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Titel en tracknummer zijn per rij inline te bewerken in de batch-tabel
- [x] #2 Ingevulde overrides blijven behouden bij het wisselen van selectie of het invullen van gedeelde velden
- [x] #3 Een override wint van een gedeelde waarde voor hetzelfde bestand
- [x] #4 Ongeldige invoer in een rij wordt bij de rij zelf gemeld en blokkeert alleen die rij
- [x] #5 De inline bewerking is bruikbaar op een telefoonscherm
- [x] #6 Integratietests dekken het samenspel van gedeelde velden en per-bestand overrides
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
`RowField` (Track, Title) als tegenhanger van `SharedField`. De invoer staat in
het formulier onder `nummer:<bestandsnaam>` en `titel:<bestandsnaam>`; er wordt
op de eerste dubbele punt gesplitst, zodat een bestandsnaam er zelf ook een mag
bevatten. `Form::parse` zet ze in een `BTreeMap<String, Override>`, dus ze gaan
met elk verzoek mee en overleven het wisselen van selectie.

`batch::intents(listing, form)` levert het plan per bestand: eerst de gedeelde
velden, dan de overrides eroverheen. Die `insert`-volgorde ís de regel dat een
override wint (AC #3), en dat plan is meteen wat de diff-preview van task-18
nodig heeft. Bestanden die niet geselecteerd zijn of waaraan niets verandert,
blijven eruit.

Een fout in een rij komt in `Row::problems` en laat alleen die rij uit het plan
vallen; `AlbumPage::problems` blijft voor de gedeelde velden. De inline velden
zijn 16px groot en knop-hoog (`.rijveld`), zodat iOS niet inzoomt en ze met een
duim te raken zijn; de tabel scrollde al binnen zijn eigen rand.

Er wordt nog steeds niets geschreven — de test die dat bewaakt, stuurt nu ook
overrides mee.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Titel en tracknummer zijn per rij inline te bewerken in de albumtabel (FR-9),
in hetzelfde formulier als de selectie en de gedeelde velden.

Dezelfde regel als bij een gedeeld veld: niets voorgevuld, leeg laten verandert
niets, en de huidige waarde staat als grijze tekst in het veld. Een override
geldt voor dat ene bestand en wint van wat de gedeelde velden ermee zouden
doen. Onbruikbare invoer wordt bij de rij gemeld en blokkeert alleen die rij.

11 nieuwe unit-tests in `batch::` en 4 integratietests in `tests/album.rs`;
`cargo fmt --check`, `cargo clippy -- -D warnings` en `cargo test` zijn groen.
Commit d32520e.
<!-- SECTION:FINAL_SUMMARY:END -->
