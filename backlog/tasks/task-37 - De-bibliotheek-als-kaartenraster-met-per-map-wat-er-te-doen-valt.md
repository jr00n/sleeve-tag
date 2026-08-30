---
id: TASK-37
title: 'De bibliotheek als kaartenraster, met per map het aantal en het formaat'
status: Done
assignee: []
created_date: '2026-08-30 07:12'
updated_date: '2026-08-30 12:53'
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
- [x] #1 De bibliotheek toont mappen als kaarten in een raster dat meeschaalt met de schermbreedte, en op een telefoon onder elkaar valt.
- [x] #2 Elke kaart noemt hoeveel bewerkbare bestanden de map bevat en welke formaten daarin voorkomen.
- [x] #3 Een kaart die geen bewerkbare bestanden bevat, zegt dat en laat geen misleidende telling zien.
- [x] #4 De hele kaart is aanklikbaar en leidt naar de mapweergave.
- [x] #5 Wat een kaart toont, komt uit de mapinhoud en niet uit de tags: er wordt geen enkel bestand geopend om de bibliotheek te kunnen tonen.
- [x] #6 Het opsommen blijft binnen MUSIC_ROOT en toont nooit iets wat de app niet mag openen.
- [x] #7 Een bibliotheek met veel mappen laadt niet merkbaar trager dan de lijst die er nu staat.
- [x] #8 De weergave is met tests gedekt, inclusief een lege map en een map met alleen submappen.
- [x] #9 README is bijgewerkt.
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
Gebouwd in een eigen worktree, commit b2440f2, gemerged op main (4996b56).

**Tellen zonder een bestand te openen.** Nieuw type `fs::DirectorySummary
{ files, directories, formats }` en `Library::summarize`: één `read_dir` per
map, geen `stat` per entry (het `file_type` uit `read_dir` volstaat) en geen
enkel bestand geopend. Alleen een symlink wordt gevolgd en tegen `MUSIC_ROOT`
gehouden, net als in `list_directory` — een symlink naar buiten telt niet mee.
De telling gaat één niveau diep; dat staat zo gedocumenteerd.

**`browse::Folder`** kreeg `files`, `subfolders`, `formats`, `contents_label()`
en `has_files()`. Labels: "3 bestanden", "2 submappen", "1 bestand · 1 submap",
of "Geen bewerkbare bestanden" — nooit "0 bestanden". `summarize` wordt alleen
aangeroepen voor mappen die het zoekfilter overleven.

**Weergave.** Nieuw `templates/folders.html` met het raster; in `listing.html`
is het oude `<ul class="mappen">`-blok vervangen door één include. De hele kaart
is één `<a>`. CSS-blok onderaan `app.css`:
`repeat(auto-fill, minmax(min(100%, 15rem), 1fr))` — meeschalend zonder
breekpunt, en op een telefoon valt de kolom vanzelf op 100%. Alleen bestaande
Nocturne-tokens. `has_editable_extension` loopt nu via `editable_extension`,
zodat `.MP3` en `.mp3` één formaat zijn.

**Bewuste beperking.** Het formaat op een kaart komt van de extensie. Een
bestand dat `track.mp3` heet maar geen MP3 is, telt op de kaart mee en valt pas
weg zodra je de map opent. Dat is de prijs van AC #5 (geen bestand openen) en
staat zo in de README.

**Tests.** Zes unit-tests in `fs::` (formaten en tellingen, alleen submappen,
lege map, symlink naar buiten, symlink naar binnen, onleesbaar pad), zes in
`browse::` (labels, enkelvoud, gecombineerd, lege map, alleen submappen,
200 mappen onder 1s) en vijf integratietests (kaart met aantal, formaten en
link; map zonder bestanden; kaarten in het HTMX-fragment; "geen bestand
geopend" via een nep-MP3; 200 mappen onder 2s).

**Bij de merge opgelost:** de worktree vertrok van 5e43409 en kende task-35 dus
niet. Conflict in `app.css` (filterblok tegenover een hernoemde sectiekop, beide
behouden) en zes nieuwe tests die nog de oude signatuur `listing(&library, X,
"")` gebruikten; omgezet naar `&Filter::default()`.

**Kwaliteitspoort na de merge:** fmt, clippy `--all-targets` en 486 tests groen.
<!-- SECTION:NOTES:END -->
