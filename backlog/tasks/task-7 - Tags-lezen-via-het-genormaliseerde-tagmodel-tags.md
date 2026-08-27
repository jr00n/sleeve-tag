---
id: TASK-7
title: 'Tags lezen via het genormaliseerde tagmodel (tags::)'
status: Done
assignee:
  - claude
created_date: '2026-08-26 22:23'
updated_date: '2026-08-27 05:40'
labels: []
milestone: m-1
dependencies:
  - TASK-1
  - TASK-4
documentation:
  - PRD.md
priority: high
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
De frontend werkt uitsluitend met een genormaliseerd tagmodel; de backend vertaalt van en naar het containerformaat. Deze taak levert de leeszijde van die vertaling voor MP3 (ID3v2) en FLAC (Vorbis comments).

De veldafbeelding staat in PRD.md §7: title/TIT2/TITLE, artist/TPE1/ARTIST, album_artist/TPE2/ALBUMARTIST, album/TALB/ALBUM, track+track_total/TRCK `n/total`/TRACKNUMBER+TRACKTOTAL, disc+disc_total/TPOS/DISCNUMBER+DISCTOTAL, year/TDRC/DATE, genre/TCON/GENRE, composer/TCOM/COMPOSER, comment/COMM/COMMENT, en art via APIC type 3 respectievelijk METADATA_BLOCK_PICTURE type 3.

Naast de gemodelleerde velden moeten de ruwe, aanwezige tags opvraagbaar zijn (nodig voor de geavanceerde weergave), evenals technische eigenschappen: duur en formaat. Multi-value velden worden in het MVP als één string behandeld. Alle `lofty`-aanroepen blijven binnen deze module.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Een leesfunctie geeft voor een MP3 en voor een FLAC het volledige genormaliseerde tagmodel terug, inclusief duur en formaat
- [x] #2 Ontbrekende velden komen terug als leeg/afwezig, niet als lege string die van een echt lege tag te onderscheiden is
- [x] #3 Gecombineerde velden (TRCK/TPOS `n/total`) worden correct gesplitst naar nummer en totaal
- [x] #4 Aanwezigheid, formaat, afmetingen en bytegrootte van embedded front cover art zijn opvraagbaar zonder de hele afbeelding te hoeven decoderen wanneer alleen de metadata nodig is
- [x] #5 Er is een aparte functie die alle ruwe aanwezige tags (ID3-frames respectievelijk Vorbis-comments) als sleutel-waardelijst teruggeeft
- [x] #6 Een bestand dat geen geldig MP3/FLAC blijkt geeft een duidelijke fout in plaats van een panic
- [x] #7 Tests draaien tegen de fixtures uit tests/fixtures/ en dekken beide formaten, met en zonder tags en met embedded art
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
## Uitgangssituatie (onderzocht 2026-08-27)

`src/tags/mod.rs` bevat alleen `is_supported_format` uit de padtaak. De fixtures dekken alle varianten die nodig zijn. De
identifiers in het project zijn zojuist naar het Engels omgezet; dit werk volgt die conventie.

## Model

```
Track          — alles wat over één bestand bekend is
├── tags: Tags       (het genormaliseerde model uit PRD §7)
├── format: Format   (Mp3 | Flac)
├── duration         (uit de audio-eigenschappen)
└── art: Option<ArtInfo>
```

`Tags` gebruikt overal `Option`: een veld dat niet in het bestand staat is `None`. Een tag die bestaat maar leeg of
alleen witruimte is, wordt ook `None` — het PRD zegt dat een leeg veld "verwijderd" betekent, dus een lege tag is geen
betekenisvolle waarde. Dat is precies acceptatiecriterium #2.

`track`/`track_total` en `disc`/`disc_total` zijn aparte `Option<u32>`. Voor MP3 zit dat in één frame (`n/total`), voor
FLAC in twee velden; lofty normaliseert dat al, maar de tests leggen vast dat beide formaten hetzelfde resultaat geven.

`year` blijft een `String`: `TDRC` en `DATE` mogen een volledige datum bevatten, en die informatie mag niet stilzwijgend
worden weggegooid door hem als getal te parsen.

## Album art

`ArtInfo { mime, width, height, bytes }`. De afmetingen komen uit de header van de afbeelding — `image` kan die lezen
zonder de pixels te decoderen. Dat is wat acceptatiecriterium #4 bedoelt met "zonder de hele afbeelding te decoderen":
de maplijst toont straks per bestand of er art is en hoe groot, en mag daarvoor geen dertig JPEG's uitpakken.

Er komt een aparte functie voor de ruwe bytes, zodat het thumbnail-endpoint straks de afbeelding kan serveren zonder het
hele tagmodel op te bouwen.

## Ruwe tags

`read_raw_tags` geeft een lijst sleutel-waardeparen met de originele naam zoals die in het bestand staat (ID3-frame-ID of
Vorbis-veldnaam). Binaire waarden zoals art worden samengevat in plaats van uitgeschreven — een APIC-frame van 40 kB in
een HTML-tabel helpt niemand.

## Fouten

`TagError` met thiserror, met een variant voor "kan niet gelezen worden" en voor "geen ondersteund formaat". Het pad zit
niet in de melding, om dezelfde reden als bij `PathError`.

## Tests

Tegen de fixtures: beide formaten met volledige tags (alle velden gecontroleerd), beide zonder tags (alles `None`, geen
lege strings), beide met art (mime, afmetingen en omvang), de ID3v1-varianten, een JPEG die zich voordoet als MP3
(duidelijke fout, geen panic) en een niet-bestaand pad. Plus een test die vastlegt dat MP3 en FLAC voor dezelfde
tagwaarden hetzelfde model opleveren — dat is de kern van "de frontend weet niet welk formaat eronder ligt".
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
De comment bleek in geen van beide formaten leesbaar met de bestaande fixtures, en de oorzaak was in beide gevallen ffmpeg — niet lofty.

Voor MP3: ffmpeg schrijft een comment altijd als `TXXX` met beschrijving 'comment', nooit als het `COMM`-frame dat het PRD noemt en dat Picard en iTunes gebruiken. lofty laat een TXXX-frame zonder bekende beschrijving helemaal weg uit de tag-items, dus de waarde was onzichtbaar. Ook `-metadata COMMENT=` levert TXXX op; ffmpeg kan het simpelweg niet. Het genereerscript plakt daarom zelf een COMM-frame aan de MP3-fixtures (tests/fixtures/voeg-comm-frame-toe.py). Zonder die stap zou de fixture iets testen wat in een echte bibliotheek nauwelijks voorkomt.

Voor FLAC: ffmpeg schrijft het Vorbis-veld `DESCRIPTION`, terwijl Picard `COMMENT` gebruikt. Beide komen in het wild voor, dus het model leest nu Comment met DESCRIPTION als terugval. Dat is geen concessie aan de fixture maar aan de werkelijkheid.

lofty 0.25 wijkt op drie punten af van wat ik verwachtte: `map_key` neemt één argument, `get_string` neemt de ItemKey by value, en `Tag` heeft geen `year()`. Het jaar komt nu uit RecordingDate met Year als terugval.

`year` is bewust een String en geen getal: TDRC en DATE mogen een volledige datum bevatten, en die informatie mag niet sneuvelen doordat de app er een jaartal van maakt. Bij het schrijven wordt dat weer relevant.

De afmetingen van embedded art komen uit `image::ImageReader::into_dimensions`, dat alleen de header leest. Dat is wat acceptatiecriterium 4 bedoelt: de maplijst toont straks per bestand of er art is en hoe groot, en mag daarvoor geen dertig JPEG's decoderen.

Een MP3 met alleen een ID3v1-tag heeft geen primaire tag in lofty; `primary_tag` valt daarom terug op `first_tag`. Zonder die terugval zou zo'n bestand als volledig ongetagd worden getoond, terwijl de waarden er wel zijn.

`tags` heeft nu dezelfde tijdelijke `allow(dead_code)` als `fs`: de mapbrowser is de eerste gebruiker. Beide regels horen in die taak te verdwijnen.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Wat er is gebouwd

De leeszijde van het genormaliseerde tagmodel uit PRD §7. De rest van de applicatie werkt vanaf nu met `Track` en weet niet of eronder ID3v2-frames of Vorbis-comments liggen.

## Model

- **`Tags`** — de twaalf velden uit §7, allemaal `Option`. Een veld dat niet in het bestand staat is `None`; een tag die bestaat maar leeg is óók, want het PRD behandelt leeg als "verwijderd".
- **`Track`** — tags plus formaat, duur en art-informatie.
- **`ArtInfo`** — mime, afmetingen en bytegrootte, gelezen uit de header van de afbeelding zonder de pixels te decoderen.
- **`RawTag`** — sleutel-waardeparen met de originele naam (`TIT2`, `ALBUMARTIST`), voor de geavanceerde weergave. Binaire waarden worden samengevat, niet uitgeschreven.

Losse functies voor de ruwe coverbytes en de ruwe taglijst, zodat een thumbnail-endpoint straks niet het hele model hoeft op te bouwen.

## Wat de tests boven water haalden

De comment was in géén van beide formaten leesbaar, en beide keren lag het aan ffmpeg, niet aan lofty:

- **MP3**: ffmpeg schrijft een comment altijd als `TXXX`, nooit als het `COMM`-frame dat het PRD noemt en dat Picard en iTunes gebruiken. lofty laat zo'n TXXX-frame zonder bekende beschrijving helemaal weg uit de tag-items. Ook `-metadata COMMENT=` helpt niet — ffmpeg kan het niet. Het genereerscript plakt nu zelf een COMM-frame aan de MP3-fixtures.
- **FLAC**: ffmpeg schrijft `DESCRIPTION` waar Picard `COMMENT` schrijft. Beide komen in het wild voor, dus het model leest allebei.

Zonder die correctie zou de fixture iets testen wat in een echte bibliotheek nauwelijks voorkomt, en zou het schrijfpad in fase 2 op een verkeerd uitgangspunt bouwen.

## Tests

66 groen (was 52); achttien nieuwe, waaronder:

- het volledige model uit een getagde MP3, veld voor veld
- **MP3 en FLAC leveren hetzelfde model** — de kern van "de frontend weet niet welk formaat eronder ligt"
- `TRCK`/`TPOS` (één frame, `n/total`) en de FLAC-varianten (twee velden) geven hetzelfde resultaat
- bestanden zonder tags: alles `None`, geen lege strings
- art-metadata voor beide formaten, en de ruwe coverbytes beginnen met de JPEG-signatuur
- ruwe tags dragen de originele sleutelnamen; binaire waarden zijn samengevat
- een MP3 met alleen ID3v1 levert zijn waarden op; bij tegenstrijdigheid wint ID3v2
- een JPEG en een ontbrekend bestand geven een fout, geen panic

## Openstaand

`tags` heeft dezelfde tijdelijke `allow(dead_code)` als `fs`. De mapbrowser is voor beide de eerste gebruiker; die taak hoort de regels op te ruimen.
<!-- SECTION:FINAL_SUMMARY:END -->
