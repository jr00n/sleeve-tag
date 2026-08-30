---
id: TASK-37
title: 'De bibliotheek als kaartenraster, met per map het aantal en het formaat'
status: To Do
assignee: []
created_date: '2026-08-30 07:12'
updated_date: '2026-08-30 07:15'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) toont de bibliotheek als een raster van kaarten. Elke kaart noemt de map, hoeveel bestanden erin zitten en in welk formaat.

Sleeve toont nu een platte lijst met alleen de mapnaam. Je ziet dus niet of een map twee bestanden bevat of honderdtwaalf, en of het MP3's of FLAC's zijn, voordat je hem opent.

Deze taak gaat over de instap: welke map ga je bewerken. Wat er daarna gebeurt, verandert niet.

**Wat een kaart niet toont, en waarom.** Het ontwerp zet op elke kaart ook wat er in die map mankeert — "3 zonder tracknummer", "1 zonder hoes". Dat is bewust weggelaten: die tellingen zijn alleen te maken door elk bestand in elke submap te openen en de tags te lezen, en op een NAS met een grote bibliotheek zou de startpagina daar merkbaar traag van worden. Een kaart mag alleen tonen wat uit de mapinhoud zelf volgt — namen, extensies, aantallen. De signalering blijft waar ze nu is: in de map die je opent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 De bibliotheek toont mappen als kaarten in een raster dat meeschaalt met de schermbreedte, en op een telefoon onder elkaar valt.
- [ ] #2 Elke kaart noemt hoeveel bewerkbare bestanden de map bevat en welke formaten daarin voorkomen.
- [ ] #3 Een kaart die geen bewerkbare bestanden bevat, zegt dat en laat geen misleidende telling zien.
- [ ] #4 De hele kaart is aanklikbaar en leidt naar de mapweergave.
- [ ] #5 Wat een kaart toont, komt uit de mapinhoud en niet uit de tags: er wordt geen enkel bestand geopend om de bibliotheek te kunnen tonen.
- [ ] #6 Het opsommen blijft binnen MUSIC_ROOT en toont nooit iets wat de app niet mag openen.
- [ ] #7 Een bibliotheek met veel mappen laadt niet merkbaar trager dan de lijst die er nu staat.
- [ ] #8 De weergave is met tests gedekt, inclusief een lege map en een map met alleen submappen.
- [ ] #9 README is bijgewerkt.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo fmt --check slaagt
- [ ] #2 cargo clippy -- -D warnings slaagt
- [ ] #3 cargo test slaagt zonder toegang tot de echte muziekbibliotheek
- [ ] #4 Nieuwe of gewijzigde functionaliteit is gedekt door unit- of integratietests
- [ ] #5 Relevante documentatie (README / CLAUDE.md) is bijgewerkt
<!-- DOD:END -->
