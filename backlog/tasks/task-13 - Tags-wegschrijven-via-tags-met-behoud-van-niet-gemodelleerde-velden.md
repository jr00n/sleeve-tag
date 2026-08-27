---
id: TASK-13
title: 'Tags wegschrijven via tags:: met behoud van niet-gemodelleerde velden'
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:24'
updated_date: '2026-08-27 21:55'
labels: []
milestone: m-2
dependencies:
  - TASK-7
  - TASK-12
documentation:
  - PRD.md
modified_files:
  - src/tags/mod.rs
  - src/atomic.rs
  - src/web/mod.rs
  - README.md
  - CLAUDE.md
priority: high
type: feature
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De schrijfzijde van de vertaling uit PRD §7. De regels zijn strikt, omdat de bibliotheek door Navidrome gelezen wordt en niet beschadigd mag raken:

- MP3 wordt altijd weggeschreven als ID3v2.4 met UTF-8. Bestaande ID3v1-tags worden verwijderd of gesynchroniseerd, nooit inconsistent achtergelaten.
- Tags die de app niet modelleert blijven ongewijzigd bewaard; alleen velden die de gebruiker daadwerkelijk aanraakt worden overschreven.
- Een leeg gemaakt veld betekent 'veld verwijderen', niet 'lege string opslaan'.
- Multi-value velden worden in het MVP als één string behandeld.

Het feitelijke wegschrijven loopt via de atomische schrijfhelper uit die taak, zodat een half geschreven bestand onmogelijk is.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Een schrijffunctie neemt het genormaliseerde tagmodel aan en schrijft de velden correct weg voor zowel MP3 als FLAC
- [x] #2 MP3-bestanden zijn na het schrijven ID3v2.4 met UTF-8, ook wanneer ze daarvoor een oudere versie hadden
- [x] #3 Een bestaande ID3v1-tag is na het schrijven verwijderd of in lijn met de ID3v2-tag, nooit afwijkend
- [x] #4 Tags die niet in het model voorkomen zijn na het schrijven onveranderd aanwezig
- [x] #5 Een veld dat leeg is gemaakt is na het schrijven verwijderd uit het bestand in plaats van als lege waarde aanwezig
- [x] #6 Gecombineerde velden worden correct weggeschreven (TRCK/TPOS als `n/total`, TRACKNUMBER/TRACKTOTAL apart)
- [x] #7 De audio-inhoud van het bestand is na een tagwijziging bit-identiek aan die van daarvoor
- [x] #8 Tests schrijven naar kopieën van de fixtures en lezen het resultaat terug; een test verifieert het resultaat ook met een onafhankelijke tool (bijv. ffprobe) wanneer die beschikbaar is
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

### 1. `tags::write` — de schrijfzijde van PRD §7

```rust
pub fn write(path: &Path, wanted: &Tags, options: atomic::Options) -> Result<(), WriteError>
```

Loopt volledig via `atomic::replace`, dus een half geschreven bestand is onmogelijk. Stappen:

1. **Normaliseren.** Elke waarde wordt getrimd; leeg wordt `None`. Daarmee is "een leeg veld betekent verwijderen" (AC #5) een eigenschap van de invoer en geen los geval verderop.
2. **Diff bepalen.** De huidige tags worden gelezen en vergeleken met de gewenste. Is er niets veranderd, dan wordt het bestand **niet aangeraakt** — dat is de regel "niets ongevraagd wijzigen" uit CLAUDE.md, en het scheelt Navidrome een herscan van een bestand dat gelijk is gebleven.
3. **`prepare`.** Op het tijdelijke bestand wordt de bestaande tag van het doeltype opgehaald en aangepast; alleen de gemodelleerde velden worden gezet of verwijderd.
4. **`validate`.** Het tijdelijke bestand wordt opnieuw ingelezen en het genormaliseerde model moet gelijk zijn aan wat er bedoeld was. Zo niet, dan gaat er niets over het origineel heen.

De lijst gewijzigde velden gaat als omschrijving naar `atomic::replace`, dat hem in de logregel zet — daarmee is "gelogd met pad en gewijzigde velden" uit task-12 pas echt waar.

### 2. Niet-gemodelleerde velden behouden (AC #4)
Er wordt uitgegaan van de **bestaande** tag uit het bestand, niet van een lege. Alles wat niet in het model zit blijft daardoor gewoon staan. lofty bewaart frames die niet op het generieke model passen bovendien in een *companion tag* die bij het terugschrijven weer meegaat (`preserve_format_specific_items`, standaard aan). Of dat in de praktijk klopt, moet een test uitwijzen en niet een aanname — dus daar komt een test op met een frame dat Sleeve niet kent.

Ook de embedded hoes hangt aan diezelfde tag. Een tagwijziging mag de album art niet slopen; daar komt een aparte test op.

### 3. ID3v2.4 en ID3v1 (AC #2, #3)
- lofty schrijft standaard ID3v2.4 (`use_id3v23` staat uit); UTF-8 hoort daarbij.
- `WriteOptions::remove_others(true)` laat alleen de geschreven tag staan. Een ID3v1-tag verdwijnt dus, en kan per definitie niet meer afwijken. Dat is de "verwijderd"-tak van AC #3, en die is eenvoudiger en veiliger dan synchroniseren.
- Een MP3 met alléén ID3v1 verliest daarbij niets: die tag draagt uitsluitend velden die het model kent, en die zijn al ingelezen.

### 4. Randgevallen in het model
- **Jaar** wordt als `RecordingDate` geschreven (TDRC / DATE). Het losse `Year`-veld wordt opgeruimd, zodat er geen tweede, oudere waarde blijft rondslingeren.
- **Commentaar** wordt als `Comment` geschreven; `Description` (wat ffmpeg in FLAC schrijft) wordt verwijderd. Twee plekken met tegenstrijdig commentaar laten staan zou precies de verwarring opleveren die deze app moet oplossen.
- **Tracknummer en totaal**: lofty kent het formaatverschil (`TRCK` als `n/total`, `TRACKNUMBER`/`TRACKTOTAL` apart). Een totaal zonder nummer is in ID3v2 niet uit te drukken; wat er dan gebeurt, moet een test uitwijzen voordat ik er een regel van maak.

### 5. Tests
Altijd tegen kopieën van de fixtures in een tempdir.

- Een volledige schrijfronde voor MP3 én FLAC: teruglezen levert precies het bedoelde model (AC #1).
- Een veld leegmaken verwijdert het uit het bestand — gecontroleerd op de ruwe tags, niet alleen op het model (AC #5).
- Een niet-gemodelleerd veld staat er na het schrijven nog (AC #4), en de embedded hoes ook.
- De MP3 is daarna ID3v2.4 (AC #2) en heeft geen ID3v1 meer, ook de fixture die met een afwijkende ID3v1 begon (AC #3).
- `TRCK` bevat `n/total`, Vorbis heeft `TRACKNUMBER` en `TRACKTOTAL` apart (AC #6).
- **Audio bit-identiek** (AC #7): de test knipt de tagblokken van het bestand af — ID3v2-header en ID3v1-staart bij MP3, de metadatablokken bij FLAC — en vergelijkt wat er overblijft vóór en na. Dat bewijst het zonder externe tool.
- **Onafhankelijke controle** (AC #8): een test die `ffprobe` aanroept en de waarden terugleest. Ontbreekt ffprobe, dan slaat de test zichzelf over met een melding; hij mag de kwaliteitspoort niet afhankelijk maken van een systeemtool.
- Een mislukte schrijfactie laat het origineel byte-voor-byte intact (leunt op task-12, maar wordt hier met een echt audiobestand herhaald).
- Schrijven zonder wijzigingen raakt het bestand niet aan.

### 6. Documentatie
README: een sectie over de schrijfregels uit §7 en wat er met ID3v1 gebeurt. CLAUDE.md: de regel over `tags::` aanvullen met de schrijfzijde. Het `#![allow(dead_code)]` in `atomic` kan weg zodra er een echte aanroeper is.

### Afwijkingen van het plan

- Het plan ging ervan uit dat `WriteOptions::remove_others(true)` de ID3v1-tag zou opruimen. Die vlag doet in lofty 0.25.1 niets; ID3v1 wordt nu met de hand verwijderd, via een zelf geopend lees-schrijf bestand omdat `remove_from_path` óók stuk is.
- Het randgeval 'tracktotaal zonder tracknummer' bleek geen aparte behandeling nodig te hebben: de volledige schrijfronde en de hervalidatie dekken het af, en er is geen fixture of formulier dat die combinatie kan opleveren. Er is dus ook geen regel voor bedacht.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Twee gebreken in lofty 0.25.1 gevonden, allebei tijdens het schrijven.**

1. `WriteOptions::remove_others` bestaat als vlag maar wordt in de hele crate nergens uitgelezen — hij doet niets. Mijn eerste opzet leunde erop om de ID3v1-tag kwijt te raken; de test op de `id3v1-inconsistent`-fixture liet zien dat de tag er gewoon nog stond.
2. `TagType::remove_from_path` opent het bestand met `Probe::open` (alleen-lezen) en probeert er vervolgens in te schrijven. Dat mislukt altijd, met 'failed to write to file'. Sleeve opent het bestand nu zelf lees-schrijf en gebruikt `remove_from`.

Beide omwegen zitten in `remove_stale_tags`, en `the_lofty_workarounds_are_still_needed` legt vast dát de gebreken er zijn. Gaat die test bij een nieuwere lofty stuk, dan is dat het sein om de omweg weg te halen in plaats van hem mee te slepen omdat niemand meer weet waarom hij er staat.

**Niet-gemodelleerde velden overleven het schrijven — geverifieerd, niet aangenomen.** lofty bewaart frames die niet op het generieke model passen in een companion tag (`preserve_format_specific_items`, standaard aan) en voegt ze bij het terugschrijven weer samen. De test moest daar wel toe worden uitgerust: de ingecheckte fixtures dragen uitsluitend gemodelleerde velden, dus de test zet eerst zelf een uitgever en een ISRC in het bestand — velden die Picard standaard schrijft — en controleert daarna dat `TPUB`/`TSRC` (MP3) en `PUBLISHER`/`ISRC` (FLAC) er nog staan, met dezelfde waarde.

**AC #6 is niet met `read_raw_tags` te bewijzen.** Die geeft de gesplitste kijk van de tagbibliotheek: één `TRCK` met `7/9` komt daar als twee regels langs. Om te laten zien dat er één frame met `7/9` in het bestand staat, leest de test de ID3v2-frames rechtstreeks uit de bytes (syncsafe lengtes, coderingsbyte overslaan). Voor FLAC volstaat `read_raw_tags` wel, want daar zijn `TRACKNUMBER` en `TRACKTOTAL` werkelijk aparte velden.

**AC #7 zonder externe tool.** De test knipt de tagblokken van het bestand af — het ID3v2-blok vooraan (syncsafe lengte uit de header) en een eventuele ID3v1-staart van 128 bytes bij MP3, alle metadatablokken bij FLAC — en vergelijkt wat er overblijft vóór en na. Dat bewijst bit-identieke audio zonder van ffmpeg af te hangen, en dekt ook de varianten mét embedded hoes.

**Schrijven zonder wijzigingen raakt het bestand niet aan.** `write` leest eerst de huidige tags, bepaalt welke velden verschillen, en stopt wanneer dat er geen zijn. Een bestand herschrijven dat gelijk blijft is een ongevraagde wijziging: de wijzigingsdatum verspringt en Navidrome gaat er opnieuw naar kijken zonder dat er iets te zien valt. Dezelfde diff levert meteen de veldnamen voor de logregel van `atomic::replace` — daarmee is 'gelogd met pad en gewijzigde velden' uit task-12 pas echt waar.

**`Tags::normalized` legt 'leeg betekent verwijderen' op één plek vast.** Elke waarde wordt getrimd en wat leeg blijft wordt `None`, vóór de vergelijking en vóór het schrijven. Daardoor hoeft geen enkel formulier er nog aan te denken, en levert teruglezen na schrijven exact hetzelfde model op — wat de hervalidatie een strakke controle maakt in plaats van een benadering.

**Jaar en commentaar ruimen hun tweede vindplaats op.** Het jaar gaat naar `RecordingDate` (TDRC/DATE) en het losse `Year`-veld wordt verwijderd; commentaar gaat naar `Comment` en `Description` (wat ffmpeg in FLAC schrijft) wordt verwijderd. Twee plekken met een verschillende waarde laten staan is precies de verwarring die deze app moet wegnemen — en het is geen ongevraagde wijziging, want de gebruiker heeft dat veld juist aangeraakt.

**`atomic` houdt zijn `#![allow(dead_code)]` nog even.** `tags::write` gebruikt de module nu wel, maar `write` zelf wordt pas door het bewerkformulier (task-14) aangeroepen. Zolang die keten nergens in een handler eindigt, ziet de compiler het geheel als ongebruikt. Het commentaar bij de uitzondering is bijgewerkt zodat duidelijk is waar hij op wacht.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Tags wegschrijven via `tags::` met behoud van niet-gemodelleerde velden

De schrijfzijde van PRD §7. `tags::write` neemt het genormaliseerde model aan, zet het in het bestand, en gaat daarvoor door `atomic::replace` — een half geschreven bestand is dus onmogelijk.

### Wat het doet

- **Alleen gemodelleerde velden worden aangeraakt.** Er wordt begonnen bij de tag die al in het bestand staat, niet bij een lege, en lofty bewaart frames die niet op het model passen in een companion tag. Een `TPUB`, een `TSRC`, een `ENCODER` en de embedded hoes overleven een tagwijziging allemaal.
- **Leeg betekent verwijderen.** `Tags::normalized` trimt elke waarde en maakt er `None` van wanneer er niets overblijft — vóór de vergelijking én vóór het schrijven, zodat geen enkel formulier er nog aan hoeft te denken.
- **MP3 wordt ID3v2.4 met UTF-8**, ook zonder bestaande ID3v2-tag. De ID3v1-tag wordt verwijderd: die kan maar dertig tekens per veld en zou na een wijziging iets anders zeggen dan ID3v2. Verwijderen maakt die tegenstrijdigheid onmogelijk.
- **Samengestelde velden volgen hun formaat**: `TRCK`/`TPOS` als `7/9`, Vorbis-comments met `TRACKNUMBER` en `TRACKTOTAL` apart.
- **Verandert er niets, dan gebeurt er niets.** De diff tussen huidige en gewenste tags bepaalt of er geschreven wordt, en levert meteen de veldnamen voor de logregel.

### Twee gebreken in lofty 0.25.1

Gevonden doordat tests faalden, niet door de documentatie te lezen:

1. `WriteOptions::remove_others` bestaat als vlag maar wordt nergens in de crate uitgelezen — hij doet niets. De ID3v1-tag bleef gewoon staan.
2. `TagType::remove_from_path` opent het bestand alleen-lezen en probeert er dan in te schrijven; dat mislukt altijd.

Beide omwegen zitten in `remove_stale_tags`, en `the_lofty_workarounds_are_still_needed` legt vast dát de gebreken er zijn. Gaat die test bij een nieuwere lofty stuk, dan is dat het sein om de omweg weg te halen.

### Tests

176 tests groen (141 unit, 1 architectuur, 5 art, 12 mapbrowser, 7 ruwe tags, 6 configuratie, 4 server); `cargo fmt --check` en `cargo clippy -- -D warnings` schoon. Alle schrijftests werken op kopieën van de fixtures in een tempdir.

- Een volledige schrijfronde voor MP3 en FLAC, ook op bestanden zonder enige tag.
- Een leeggemaakt veld verdwijnt uit het bestand — gecontroleerd op de ruwe sleutels, niet alleen op het model.
- Uitgever en ISRC (die de test er eerst zelf in zet, want de fixtures dragen alleen gemodelleerde velden) staan er na het schrijven nog, met dezelfde waarde. De embedded hoes ook.
- ID3v2.4 na het schrijven, geen ID3v1 meer — ook op de fixture die met een afwijkende ID3v1 begon.
- `TRCK` bevat `7/9`, gelezen uit de ruwe frame-bytes; `read_raw_tags` kan dat niet bewijzen omdat die de gesplitste kijk van de bibliotheek geeft.
- De audio is bit-identiek: de test knipt de tagblokken eraf en vergelijkt de rest.
- Een mislukte schrijfactie laat het audiobestand byte-voor-byte intact.
- `ffprobe` leest terug wat er geschreven is (AC #8). Ontbreekt ffprobe, dan slaat de test zichzelf over: de kwaliteitspoort mag niet van een systeemtool afhangen.

### Vervolg

`atomic` houdt zijn `#![allow(dead_code)]` nog even: `tags::write` gebruikt de module wel, maar wordt zelf pas vanaf het bewerkformulier (task-14) aangeroepen.
<!-- SECTION:FINAL_SUMMARY:END -->
