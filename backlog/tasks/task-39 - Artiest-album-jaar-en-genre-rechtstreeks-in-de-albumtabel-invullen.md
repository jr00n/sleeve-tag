---
id: TASK-39
title: 'Artiest, album, jaar en genre rechtstreeks in de albumtabel invullen'
status: Done
assignee: []
created_date: '2026-08-30 07:13'
updated_date: '2026-08-30 12:54'
labels: []
dependencies: []
references:
  - >-
    https://claude.ai/design/p/5afac6eb-4f00-4e4a-9ea9-047921edeb4a?file=Sleeve.dc.html
type: feature
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Het ontwerp in Claude Design (project 5afac6eb-4f00-4e4a-9ea9-047921edeb4a, artboard `Sleeve.dc.html`) laat elke kolom in de bestandstabel bewerken: titel, artiest, album, jaar en genre, plus track- en discnummer.

Sleeve kent nu twee wegen: gedeelde velden voor de hele selectie, en per bestand alleen titel en tracknummer. Een compilatie waarin elke track een andere artiest heeft, is daardoor alleen bestand voor bestand te doen — terwijl de tabel er al staat.

De regel die er al is, blijft gelden: wat per bestand wordt ingetikt, is een override en wint van wat de gedeelde velden voor datzelfde bestand zouden doen.

Buiten scope: nieuwe velden die Sleeve nog niet kent, en het bewerken van ruwe frames.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Artiest, album, jaar en genre zijn per bestand in de tabel zelf in te tikken, net als titel en tracknummer nu.
- [x] #2 Wat er per bestand wordt ingetikt, wint van wat de gedeelde velden voor datzelfde bestand zouden doen.
- [x] #3 Leeg laten betekent in de tabel hetzelfde als nu bij titel en tracknummer: het bestand houdt wat het heeft; wissen blijft een aparte, expliciete keuze.
- [x] #4 Een fout in één rij houdt alleen die rij tegen en niet de rest van de batch, en wordt bij die rij gemeld.
- [x] #5 De tabel blijft op een telefoon bruikbaar: hij scrollt binnen zijn eigen rand en de pagina zelf scrollt niet horizontaal mee.
- [x] #6 Wat er per bestand wordt ingetikt, komt terug in de voorbeeldweergave en wordt daar per veld getoond zoals de andere wijzigingen.
- [x] #7 De overrides per veld zijn met tests gedekt, inclusief het samenspel met een gedeeld veld dat hetzelfde veld raakt.
- [x] #8 README en, waar de regels veranderen, CLAUDE.md zijn bijgewerkt.
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
Gebouwd in een eigen worktree, commit c169ce4, gemerged op main (eb7acd8).

**Eén lijst velden, geen tweede.** `RowField` groeide van 3 naar 7 waarden
(`Track, Title, Artist, AlbumArtist, Album, Year, Genre`, in tabelvolgorde).
Omdat de formulier-parsing, `intents`, `hidden_fields`, `touched_fields`,
`label_of`, `value_of` en `changes_between` allemaal al over `RowField::ALL`
lopen, volgen de override per bestand (AC #1), het winnen van het gedeelde veld
(AC #2, ook van een wissen-vinkje), leeg-is-ongemoeid (AC #3) en de terugkeer in
de voorbeeldweergave (AC #6) daar rechtstreeks uit.

**Fouten per veld in plaats van per rij.** `Row` bestaat niet meer uit losse
benoemde velden maar uit `inputs: Vec<RowInput>` — de tegenhanger van
`SharedInput`. Daardoor kon `problems` van rij- naar veldniveau: een onleesbaar
tracknummer wordt gemeld ónder het veld waarin het is ingetikt en zet
`aria-invalid` alleen daar (AC #4). `AlbumPage.columns` komt uit hetzelfde
`RowField::ALL`, zodat kop en cel niet uit de pas kunnen lopen.

**Het jaar is géén getal — afwijking van de opdracht.** De briefing ging ervan
uit dat het jaar net als het tracknummer numeriek is en per rij kan falen. In
dit codebestand is `Tags::year` overal een `String`: het gedeelde veld Jaar
valideert niet, `edit::` ook niet, en `checks::` behandelt het als tekst. Dat is
bewust, want ID3v2.4 (TDRC) en Vorbis (DATE) kunnen een volledige datum
bevatten. `parse_number` erop loslaten zou een bestaande `2024-05-01`
onbewerkbaar maken, en het rijveld zou strenger zijn dan het gedeelde veld met
dezelfde naam. Het jaar is dus vrij gelaten, met de reden vastgelegd in
`RowField::is_numeric` en een test erbij
(`a_year_may_be_a_full_date_because_the_tag_model_allows_one`). AC #4 wordt
gedekt door het tracknummer.

**Sleutelnaam.** De formuliersleutel van de kolom Album is
`albumtitel:<bestand>` en niet `album:<bestand>`: `album` is al de sleutel van
het gedeelde veld, en die zitten in dezelfde body.

**Weergave.** In `albumform.html` zijn de handgeschreven cellen één
`{% for input in row.inputs %}`-loop geworden en is "Bewerken" naar de laatste
kolom verhuisd zodat de invulbare kolommen aaneengesloten zijn. Breedteklassen
`--tekst/--middel/--kort` komen uit `RowField::size`, zodat een kolombreedte
niet in twee bestanden apart bepaald wordt. `.tabelrand` kreeg `min-width: 0`:
zonder dat mag een flexitem breder worden dan zijn container en scrolt het
venster alsnog horizontaal mee (AC #5).

**Tests.** Vijf nieuwe unit-tests plus vier integratietests (kolommen aanwezig;
rij wint van gedeeld veld, end-to-end tot en met schrijven en herlezen; leeg
laat ongemoeid; fout in één rij naast goede kolommen in dezelfde én een andere
rij).

**Bij de merge opgelost:** de worktree vertrok van 5e43409 en kende task-36 dus
niet; het conflict in `app.css` waren twee blokken die beide achteraan
toevoegden, allebei behouden. `batch.rs` en `albumform.html` mergeden vanzelf.

README kreeg een nieuwe subsectie met de sleuteltabel; CLAUDE.md alleen de regel
"Een batch gaat bestand voor bestand" (opsomming van de override-velden en de
zin over het jaar).

**Kwaliteitspoort na de merge:** fmt, clippy `--all-targets` en 486 tests groen.
<!-- SECTION:NOTES:END -->
