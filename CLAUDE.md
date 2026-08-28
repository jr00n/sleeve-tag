# Sleeve (`sleeve-tag`) — werkafspraken

Sleeve is een web-based tag editor voor MP3/FLAC die als één Docker-container op
een UGREEN NAS draait. `PRD.md` is leidend voor scope en eisen; dit bestand legt
de conventies vast waar code zich aan houdt.

## Architectuurregels

- **Alle tag-I/O loopt via `tags::`.** `lofty` wordt uitsluitend binnen die
  module aangeroepen; nergens anders in de codebase. De rest van de applicatie
  werkt alleen met het genormaliseerde tagmodel uit PRD §7 en weet niet of een
  bestand ID3v2-frames of Vorbis-comments bevat. Deze regel wordt afgedwongen
  door `tests/architecture.rs`.
- **Schrijven van tags gaat via `tags::write`.** Die begint bij de tag die al in
  het bestand staat, zodat niet-gemodelleerde velden blijven bestaan, en gaat
  door `atomic::replace`. Een veld dat leeg is, wordt verwijderd en niet als
  lege waarde opgeslagen; `Tags::normalized` legt die regel op één plek vast.
  Verandert er niets, dan wordt het bestand niet aangeraakt.
- **Alle padvertaling loopt via `fs::`.** Een door de gebruiker aangeleverd pad
  wordt daar gecanonicaliseerd en gecontroleerd tegen `MUSIC_ROOT`; handlers
  bouwen nooit zelf een pad op. Dat geldt ook voor het opsommen van een map:
  `fs::Library::list_directory` controleert elke gevonden entry opnieuw, zodat
  een lijst nooit iets toont wat de app niet mag openen.
- **Alle pixelbewerking loopt via `art::`.** Decoderen, verkleinen en
  encoderen van album art gebeurt daar; `tags::` levert alleen de ruwe bytes uit
  het bestand en raakt de afbeelding verder niet aan — ook het uitlezen van de
  afmetingen gaat langs `art::`. `art::` opent zelf geen bestanden: in en uit
  gaan bytes. Deze regel wordt afgedwongen door `tests/architecture.rs`.
- **Een hoes schrijven gaat via `tags::write_art`.** Die wisselt gericht de
  front cover en laat de rest van de tag staan: tekstuele velden, maar ook
  andere afbeeldingen. Verandert er niets, dan wordt het bestand niet
  aangeraakt. Ook deze route loopt door `atomic::replace`, met dezelfde
  hervalidatie.
- **Een aangeleverde hoes gaat door `art::prepare`.** Die valideert op de bytes
  zelf (alleen JPEG en PNG), verkleint alleen wat boven `MAX_ART_SIZE` uitkomt,
  en hercodeert alleen wat verkleind is. Past de afbeelding al, dan gaan de
  bytes ongewijzigd het bestand in — geen kwaliteitsverlies, en een PNG blijft
  een PNG. Bij verkleinen wordt het JPEG, tenzij er werkelijk doorzichtige
  pixels in zitten.
- **De hoesweergave (`cover::`) beschrijft alleen.** Ze krijgt de `ArtInfo` die
  `tags::` uit het bestand las en maakt daar tekst van; ze opent geen bestanden
  en raakt geen pixels aan.
- **Wat de UI toont, wordt in `browse::` opgebouwd.** Die module brengt `fs::`
  en `tags::` samen tot een weergavemodel; handlers renderen dat model en
  bevatten zelf geen sorteer-, filter- of opmaaklogica. Paden die naar de
  browser gaan zijn altijd relatief aan `MUSIC_ROOT`.
- **De albumweergave (`batch::`) stelt alleen voor.** Ze krijgt een `Listing` en
  een verstuurd formulier binnen, opent geen bestanden en schrijft niets — ook
  `batch::preview` niet. Wat ermee gebeurt, beslist de gebruiker in de
  voorbeeldweergave; dat is de enige route waarlangs een batch wordt
  weggeschreven, en `actie=opslaan` is het enige verzoek dat schrijft.
- **Een batch gaat bestand voor bestand.** Elk bestand wordt vlak voor het
  schrijven opnieuw ingelezen en het plan wordt op die verse inhoud toegepast.
  Een fout bij één bestand stopt de rest niet en wordt per bestand gemeld; een
  fout in de invoer zelf houdt de hele batch tegen, want half uitvoeren van een
  plan dat niet klopt is erger dan niets doen. In een gedeeld veld betekent
  leeg daar "ongemoeid laten" en niet "verwijderen" — het veld wordt nooit
  voorgevuld, en wissen is een aparte, expliciete keuze. Titel en tracknummer
  zijn geen gedeeld veld maar een override per bestand; die wint van wat de
  gedeelde velden voor datzelfde bestand zouden doen, en een fout in één rij
  houdt alleen die rij tegen.
- **Een hulpactie vult alleen invoervelden.** Hernummeren, artiest → albumartiest
  en hoofdletters normaliseren zetten een voorstel in het formulier en verder
  niets: geen bestand gaat open, geen tag wordt geschreven. Wat een actie
  voorstelt is met de hand aan te passen en met "Invoer leegmaken" in één klik
  terug te draaien. Een voorstel dat gelijk is aan wat er al staat, wordt niet
  ingevuld.
- **Hoofdletterlogica staat in `casing::`.** Die module kent geen tags en geen
  bestanden: in en uit gaat tekst. Ze raadt, en wat ze raadt hoort zichtbaar en
  terug te draaien te zijn — vandaar dat de uitkomst een voorstel in een veld is
  en nooit een schrijfactie.
- **Schrijven is atomisch, en loopt via `atomic::replace`.** Naar een tijdelijk
  bestand in dezelfde map, hervalideren door opnieuw in te lezen, en pas dan
  hernoemen over het origineel. Bij een fout blijft het origineel onaangetast.
  Die volgorde staat vast in `atomic::replace`, niet bij de aanroeper: een
  schrijfactie die er zelf omheen gaat, heeft geen van die garanties. Eigenaar,
  groep en rechten van het origineel gaan mee; lukt dat niet, dan gaat de
  schrijfactie niet door.
- **Een nieuw bestand aanmaken gaat via `atomic::place`.** Dat gebeurt op één
  plek: de losse `cover.jpg` in de albummap. Dezelfde volgorde als bij
  `replace` — tijdelijk bestand in dezelfde map, eigenaar, groep en rechten van
  een track uit die map overnemen, en pas dan hernoemen. Over een bestand dat er
  al staat gaat er niets heen zonder dat de aanroeper dat expliciet toestaat, en
  identieke inhoud raakt het bestand niet aan. Het schrijven van die `cover.jpg`
  gebeurt ná het embedden en met een eigen regel in het rapport: gaat het mis,
  dan blijft wat er wél geschreven is gewoon staan. Het bestand heet altijd
  `cover.jpg` en is altijd JPEG — één vaste naam vraagt om één vast formaat, dus
  een PNG wordt daarvoor door `art::as_jpeg` gehaald terwijl het embedded
  origineel PNG blijft.
- **De signalering (`checks::`) constateert alleen.** Ze krijgt het
  genormaliseerde tagmodel binnen, opent geen bestanden en stelt geen correcties
  voor. Wat er met een gesignaleerd probleem gebeurt, beslist de gebruiker.
- **De startcontrole (`startup::`) toetst en past niets toe.** Het proces draait
  als niet-root en kan zijn eigen UID dus niet wisselen: `PUID`/`PGID` worden
  door de container-runtime gezet (`user:` in compose), en `startup::check`
  stelt bij start alleen vast of dat klopt en of `MUSIC_ROOT` schrijfbaar is.
  Het enige bestand dat ze aanraakt is haar eigen sonde, en die wordt in
  dezelfde functie weer opgeruimd. Een verkeerde uitkomst wordt gemeld, niet
  gerepareerd, en laat de app niet stoppen — bladeren werkt op een read-only
  share gewoon. Een fout in de configuratie zelf blijft wél fataal.
- **Niets ongevraagd wijzigen.** Geen achtergrondjobs, geen opschoonacties, geen
  velden aanraken die de gebruiker niet zelf heeft ingevuld.

## Conventies

- Rust stable, edition 2024. De toolchain ligt vast in `rust-toolchain.toml`.
- Code en identifiers in het Engels; doc-comments en commentaar in het Nederlands,
  net als de UI.
- Configuratie komt uitsluitend uit omgevingsvariabelen. In de container is
  `MUSIC_ROOT` altijd `/music`; het host-pad van de share is de app onbekend.

## Kwaliteitspoort

`cargo fmt --check`, `cargo clippy -- -D warnings` en `cargo test` moeten groen
zijn voordat werk als afgerond geldt.

## Tests

Tests draaien **nooit** tegen de echte muziekbibliotheek. Ze kopiëren fixtures
uit `tests/fixtures/` naar een tempdir en werken daar, via
`testfixtures::copy_to_tempdir(...)`. Een test die `MUSIC_ROOT` op een echt
bibliotheekpad zet, of die rechtstreeks tegen een fixture in de repo werkt, is
per definitie fout.

Integratietests delen hun procesbesturing via `tests/common/mod.rs`: de binary is
een langlopende server, dus wachten op `Command::output()` laat een test hangen.


<!-- BACKLOG.MD MCP GUIDELINES START -->
<!-- backlog.md-instructions-version: 1.50.1 -->

<CRITICAL_INSTRUCTION>

## BACKLOG WORKFLOW INSTRUCTIONS

This project uses Backlog.md MCP for all task and project management activities.

**CRITICAL GUIDANCE**

- If your client supports MCP resources, read `backlog://workflow/overview` to understand when and how to use Backlog for this project.
- If your client only supports tools or the above request fails, call `backlog.get_backlog_instructions()` to load the tool-oriented overview. Use the `instruction` selector when you need `task-creation`, `task-execution`, or `task-finalization`.

- **First time working here?** Read the overview resource IMMEDIATELY to learn the workflow
- **Already familiar?** You should have the overview cached ("## Backlog.md Overview (MCP)")
- **When to read it**: BEFORE creating tasks, or when you're unsure whether to track work

These guides cover:
- Decision framework for when to create tasks
- Search-first workflow to avoid duplicates
- Links to detailed guides for task creation, execution, and finalization
- MCP tools reference

You MUST read the overview resource to understand the complete workflow. The information is NOT summarized here.

</CRITICAL_INSTRUCTION>

<!-- BACKLOG.MD MCP GUIDELINES END -->
