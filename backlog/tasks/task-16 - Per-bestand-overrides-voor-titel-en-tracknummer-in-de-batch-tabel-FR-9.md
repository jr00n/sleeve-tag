---
id: TASK-16
title: Per-bestand overrides voor titel en tracknummer in de batch-tabel (FR-9)
status: To Do
assignee: []
created_date: '2026-08-26 22:24'
labels: []
milestone: m-3
dependencies:
  - TASK-15
documentation:
  - PRD.md
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
- [ ] #1 Titel en tracknummer zijn per rij inline te bewerken in de batch-tabel
- [ ] #2 Ingevulde overrides blijven behouden bij het wisselen van selectie of het invullen van gedeelde velden
- [ ] #3 Een override wint van een gedeelde waarde voor hetzelfde bestand
- [ ] #4 Ongeldige invoer in een rij wordt bij de rij zelf gemeld en blokkeert alleen die rij
- [ ] #5 De inline bewerking is bruikbaar op een telefoonscherm
- [ ] #6 Integratietests dekken het samenspel van gedeelde velden en per-bestand overrides
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
