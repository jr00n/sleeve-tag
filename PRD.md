# PRD — Sleeve (`sleeve-tag`)

Web-based tag editor voor MP3/FLAC op de UGREEN NAS

**Status:** concept v0.1
**Eigenaar:** Jeroen
**Doel van dit document:** startpunt voor de bouw met Claude Code. De `CLAUDE.md` (werkafspraken, conventies, mapstructuur) wordt later afgeleid uit dit PRD.

---

## 0. Naamgeving

- **Weergavenaam:** Sleeve — in de UI, paginatitel, favicon en documentatie.
- **Technische naam:** `sleeve-tag` — voor de Git-repository, het Docker-image (`<namespace>/sleeve-tag`), de compose-servicenaam, de Cargo-crate (`sleeve-tag`, binary `sleeve-tag`) en de containerhostnaam.
- Reden voor het onderscheid: er bestaat een commerciële macOS-app "Sleeve" (now-playing-widget); de technische naam voorkomt verwarring in zoekresultaten en registries.

## 1. Samenvatting

Sleeve is een lichte, in Rust geschreven web-based applicatie om metadata (tags) en album art van MP3- en FLAC-bestanden te bekijken en te bewerken. De app draait als één Docker-container op een UGREEN NAS (UGOS, Linux) met de muziekshare als gemount volume, en wordt gebruikt via de browser op laptop, tablet of telefoon.

De app staat volledig los van welke mediaserver dan ook. De bibliotheek wordt in de praktijk gelezen door Navidrome op dezelfde NAS, maar de tag editor kent geen afhankelijkheid daarvan: hij schrijft standaardconforme tags en mag de bibliotheek nooit beschadigen.

## 2. Probleem en motivatie

- Bestaande tag editors (MusicBrainz Picard, Kid3, Mp3tag) zijn desktop-applicaties; ze vereisen dat de share via SMB gemount wordt en werken niet vanaf een tablet of telefoon.
- Mediaservers (Navidrome, Jellyfin) tonen tags maar bieden geen schrijffunctie.
- Er ontbreekt een eenvoudige, altijd-beschikbare manier om "even snel" een album te corrigeren op de plek waar de bestanden staan.

## 3. Doelen

1. Tags van individuele bestanden en hele albums (mappen) via de browser bekijken en bewerken.
2. Album art tonen, uploaden, vervangen en in alle tracks van een album embedden.
3. Draaien als Docker-container op de UGREEN NAS zonder handmatige tussenstappen na `docker compose up`.
4. Nooit dataverlies: elke schrijfactie is gevalideerd en herstelbaar.

## 4. Non-goals (MVP)

- Geen mediaspeler, geen streaming, geen bibliotheekbeheer à la Lidarr.
- Geen ondersteuning voor andere formaten dan MP3 en FLAC (M4A/OGG/WAV zijn expliciet later).
- Geen multi-user, rollen of accounts. Toegang wordt op netwerkniveau afgeschermd (alleen bereikbaar binnen het LAN en via Tailscale).
- Geen koppeling met een mediaserver in het MVP; een optionele notificatie is een latere uitbreiding (§11).
- Geen automatische, ongevraagde wijzigingen aan de bibliotheek (geen achtergrond-"opschoonjobs").
- Geen hernoemen/verplaatsen van bestanden in het MVP.

## 5. Gebruiker en context

- Eén gebruiker (beheerder van de bibliotheek), technisch onderlegd.
- Gebruik: incidenteel, sessies van enkele minuten tot een uur, vaak op tablet of telefoon vanaf de bank.
- Bibliotheek: mappenstructuur op de NAS, typisch `Artiest/Album/track.ext`, maar niet gegarandeerd consistent.
- Navidrome leest dezelfde share en pikt wijzigingen zelf op via zijn periodieke scan; de tag editor hoeft daar niets voor te doen.

## 6. Functionele eisen

### 6.1 Navigatie en overzicht
- FR-1 Mapbrowser over het gemounte muziekvolume, startend bij de geconfigureerde root. Navigeren is beperkt tot deze root (geen path traversal).
- FR-2 Per map: lijst van MP3/FLAC-bestanden met de belangrijkste tags (tracknr, titel, artiest, album, duur, formaat) en een thumbnail van de embedded art.
- FR-3 Zoeken/filteren binnen de huidige map op bestandsnaam of titel.
- FR-4 Visuele markering van bestanden met ontbrekende of inconsistente tags (bijv. geen album art, album verschilt binnen dezelfde map).

### 6.2 Tags bekijken en bewerken (per bestand)
- FR-5 Formulier met de kernvelden: titel, artiest, albumartiest, album, tracknummer + totaal, discnummer + totaal, jaar, genre, componist, commentaar.
- FR-6 Opslaan schrijft de tags in het bestand en toont daarna de opnieuw ingelezen waarden ter bevestiging.
- FR-7 Een "geavanceerd"-weergave toont alle aanwezige ruwe tags (ID3-frames / Vorbis-comments), alleen-lezen in het MVP.

### 6.3 Batch-bewerking (per album/map)
- FR-8 Meerdere bestanden in een map selecteren (of alles) en gedeelde velden in één keer zetten: albumartiest, album, jaar, genre, disc.
- FR-9 Per-bestand overrides in dezelfde weergave voor titel en tracknummer (tabel met inline bewerken).
- FR-10 Hulpacties: tracknummers automatisch nummeren op basis van huidige sortering; "artiest → albumartiest kopiëren"; hoofdletters normaliseren (optioneel, met preview).
- FR-11 Vóór opslaan een diff-preview: welke bestanden krijgen welke wijzigingen. Opslaan gebeurt bestand-voor-bestand met foutrapportage per bestand; één fout blokkeert de rest niet.

### 6.4 Album art
- FR-12 Huidige embedded art tonen (groot) inclusief afmetingen, formaat en bestandsgrootte.
- FR-13 Nieuwe art uploaden (JPEG/PNG) vanuit de browser en embedden in één bestand of in alle geselecteerde bestanden van het album.
- FR-14 Optie om de art ook als `cover.jpg` in de albummap weg te schrijven (Navidrome en vrijwel alle spelers pakken dit op).
- FR-15 Optie om te grote art automatisch te verkleinen naar een configureerbare maximale resolutie (standaard 1000×1000) en JPEG-kwaliteit.
- FR-16 Embedded art verwijderen.


## 7. Tagmodel

De frontend werkt uitsluitend met een genormaliseerd model. De backend vertaalt van/naar het containerformaat.

| Veld (intern)   | ID3v2.4 (MP3)          | Vorbis comment (FLAC)         |
|-----------------|------------------------|-------------------------------|
| title           | TIT2                   | TITLE                         |
| artist          | TPE1                   | ARTIST                        |
| album_artist    | TPE2                   | ALBUMARTIST                   |
| album           | TALB                   | ALBUM                         |
| track / track_total | TRCK (`n/total`)   | TRACKNUMBER / TRACKTOTAL      |
| disc / disc_total   | TPOS (`n/total`)   | DISCNUMBER / DISCTOTAL        |
| year            | TDRC                   | DATE                          |
| genre           | TCON                   | GENRE                         |
| composer        | TCOM                   | COMPOSER                      |
| comment         | COMM                   | COMMENT                       |
| art             | APIC (type 3, front)   | METADATA_BLOCK_PICTURE (type 3) |

Regels:
- MP3 wordt altijd weggeschreven als ID3v2.4 (UTF-8); dit is het formaat dat Navidrome en moderne spelers het best ondersteunen. Bestaande ID3v1-tags worden verwijderd of gesynchroniseerd, nooit inconsistent achtergelaten.
- Onbekende/niet-gemodelleerde tags blijven ongewijzigd bewaard; de app overschrijft alleen velden die de gebruiker aanraakt.
- Lege invoer in een veld betekent "veld verwijderen", en dit wordt in de diff-preview expliciet getoond.
- Multi-value velden (meerdere artiesten/genres) worden in het MVP als één string behandeld; splitsen is een latere uitbreiding.

## 8. Technische eisen

### 8.1 Stack
- **Taal:** Rust (stable toolchain via `rustup`, edition 2024). Motivatie: één statische binary, minimale image en geheugenfootprint op de NAS, en een typesysteem dat het tagmodel afdwingt.
- **Tag I/O:** `lofty` — pure Rust, leest en schrijft ID3v2 (MP3) en Vorbis-comments/pictures (FLAC) inclusief embedded art. Alle bestandsmutaties lopen uitsluitend via één eigen module (`tags::`) die het genormaliseerde model uit §7 vertaalt; nergens anders in de code wordt `lofty` direct aangeroepen.
- **Web:** `axum` (op `tokio`) als HTTP-framework, `tower-http` voor statische bestanden en logging-middleware.
- **Templates/UI:** serverside templates via `askama` (compile-time gecontroleerd) + HTMX vanaf een lokaal meegeleverde JS-file. Geen node-toolchain, geen aparte frontend-build. Alternatief (SPA) alleen als de UI-complexiteit dat later rechtvaardigt.
- **Afbeeldingen:** `image`-crate voor het decoderen, verkleinen en opnieuw encoderen (JPEG/PNG) van album art.
- **Overig:** `serde` voor modellen/JSON, `tracing` + `tracing-subscriber` voor logging, `clap` of `envy` voor configuratie uit omgevingsvariabelen, `anyhow`/`thiserror` voor foutafhandeling.
- **Tests:** `cargo test` met unit-tests per module en integratietests tegen een tijdelijke map. Fixtures (1 seconde stilte als MP3 en FLAC, plus varianten met bestaande tags/art) worden eenmalig gegenereerd met ffmpeg en ingecheckt onder `tests/fixtures/`; tests kopiëren fixtures naar een tempdir en draaien nooit tegen de echte bibliotheek.

### 8.2 Ontwikkelworkflow (MacBook) en build
- Ontwikkelen gebeurt native op macOS (Apple Silicon): `cargo run` met een lokale testmap als `MUSIC_ROOT`, `cargo watch -x run` (of `bacon`) voor automatisch herbouwen, rust-analyzer in de editor. Docker is tijdens ontwikkelen niet nodig.
- Kwaliteitspoort: `cargo fmt --check`, `cargo clippy -- -D warnings` en `cargo test` moeten groen zijn voor elke fase-afronding.
- Productie-image voor de NAS (`linux/amd64`; te bevestigen met `uname -m` op de UGREEN) via een multi-stage Dockerfile: build-stage op `rust:<stable>-slim`, runtime-stage op `gcr.io/distroless/static` of `alpine` met een statisch gelinkte binary (`x86_64-unknown-linux-musl`). Bouwen vanaf de Mac met `docker buildx build --platform linux/amd64`; als alternatief cross-compilen met `cargo zigbuild`.
- Distributie: in eerste instantie `docker save | ssh nas docker load`; in fase 5 een GitHub Action die bij een versie-tag het image naar GHCR pusht, zodat de NAS het via `docker compose pull` binnenhaalt.

### 8.3 Container en deployment
- Eén image (statische binary in een minimale runtime-image), gestart via `docker compose`.
- Volumes: muziekshare op `/music` (read-write), optioneel `/config` voor instellingen/logs.
- Het proces draait als niet-root; `PUID`/`PGID` zijn via env instelbaar en worden bij start toegepast (entrypoint of `user:` in compose), zodat geschreven bestanden dezelfde eigenaar/groep krijgen als de rest van de share op de UGREEN. Op deze NAS is dat `PUID=1000` en `PGID=10`; de meegeleverde `docker-compose.yml` gebruikt deze waarden als standaard.
- Configuratie via omgevingsvariabelen: `MUSIC_ROOT`, `PORT`, `PUID`, `PGID`, `MAX_ART_SIZE`, `LOG_LEVEL`, `BACKUP_ON_WRITE`.
- Healthcheck-endpoint (`/healthz`) voor Docker.
- Voorbeeld `docker-compose.yml` wordt meegeleverd, met commentaar gericht op UGOS.

### 8.4 Data-integriteit en veiligheid
- Schrijven gebeurt atomisch: naar een tijdelijk bestand in dezelfde map, valideren door opnieuw in te lezen, en pas dan hernoemen over het origineel. Bij een fout blijft het origineel onaangetast.
- Optionele backup: bij `BACKUP_ON_WRITE=true` wordt een `.bak` naast het bestand geplaatst (standaard uit, om de share niet te vervuilen).
- Alle paden worden gecanonicaliseerd (`std::fs::canonicalize`) en gecontroleerd tegen `MUSIC_ROOT`; symlinks buiten de root worden geweigerd.
- Alleen bestanden met extensie `.mp3`/`.flac` én herkend containerformaat worden bewerkbaar getoond.
- Geen inbound-toegang van buiten het tailnet/LAN; de app biedt zelf geen authenticatie in het MVP en documenteert dit expliciet.

### 8.5 Niet-functionele eisen
- Een map met 30 tracks laadt in < 1 s op de NAS (tags worden lazy en per map gelezen, geen bibliotheek-index in het MVP).
- Responsive UI: bruikbaar op een telefoonscherm (batch-tabel mag horizontaal scrollen).
- Logging naar stdout in leesbaar formaat; elke schrijfactie wordt gelogd met pad en gewijzigde velden.
- Image-grootte onder de 30 MB; geheugengebruik in rust onder de 30 MB.

## 9. Fasering

| Fase | Inhoud | Klaar wanneer |
|------|--------|---------------|
| 0 | Cargo-project `sleeve-tag`, `CLAUDE.md`, axum "hello world" met askama-template, fixtures onder `tests/fixtures/`, multi-stage Dockerfile, `cargo fmt`/`clippy`/`test` groen | `cargo run` toont een pagina op de Mac; `docker buildx` levert een werkend `linux/amd64`-image |
| 1 | Mapbrowser + tags lezen (FR-1 t/m FR-4, FR-7) | Echte share read-only doorbladeren op de NAS |
| 2 | Per-bestand bewerken en atomisch opslaan (FR-5, FR-6, §8.3) | Wijziging op een testkopie correct teruggelezen door de app én door een onafhankelijke tool (bijv. `ffprobe` of Navidrome) |
| 3 | Batch-bewerking met diff-preview (FR-8 t/m FR-11) | Compleet album in één keer corrigeren |
| 4 | Album art (FR-12 t/m FR-16) | Hoes uploaden, embedden, `cover.jpg` schrijven |
| 5 | PUID/PGID-afronding, `docker-compose.yml` voor UGOS, documentatie | Productie op de UGREEN met de echte share |

Elke fase eindigt met werkende tests en een korte handmatige check op de NAS. Fase 6+ (uitbreidingen) pas plannen als fase 5 stabiel draait.

## 10. Acceptatiecriteria MVP

1. Op de UGREEN NAS start de container met `docker compose up -d` en is de UI bereikbaar via `http://<nas>:<port>` en via Tailscale.
2. Een willekeurig album (MP3 én FLAC) kan volledig worden gecorrigeerd — velden, tracknummers, hoes — vanaf een tablet, en de bestanden behouden dezelfde eigenaar/permissies als daarvoor.
3. Na de reguliere scan van Navidrome toont Navidrome de nieuwe metadata en hoes, zonder dat de tag editor daar iets voor hoeft te doen.
4. Het bewust afbreken van de container tijdens een schrijfactie (test) laat geen beschadigd of half geschreven bestand achter.
5. `cargo test`, `cargo clippy -- -D warnings` en `cargo fmt --check` slagen zonder toegang tot de echte bibliotheek.

## 11. Latere uitbreidingen (buiten MVP)

- MusicBrainz-lookup: album opzoeken op basis van bestaande tags of bestandsnamen en velden automatisch voorstellen.
- Cover Art Archive: hoes ophalen op basis van MusicBrainz release-ID.
- Bestanden en mappen hernoemen volgens een configureerbaar patroon (`{albumartist}/{album}/{track:02} - {title}`).
- Extra formaten: M4A/AAC, OGG, Opus.
- Multi-value tags, ReplayGain-velden, lyrics.
- Bibliotheekbrede rapportage: albums zonder hoes, inconsistente albumartiesten, dubbele tracks.
- Optionele mediaserver-notificatie na opslaan: voor Navidrome via de Subsonic-API (`startScan`), voor andere servers via een configureerbare webhook. Alleen actief als geconfigureerd; de app blijft zonder deze koppeling volledig functioneel.
- Eenvoudige authenticatie (basic auth of een tokencookie) voor het geval de app ooit buiten het tailnet wordt ontsloten.

## 12. Open vragen

- Exacte pad van de muziekshare op de UGREEN (nodig voor de volume-mount in de `docker-compose.yml`). UID/GID zijn bekend: 1000/10.
- Is er een voorkeur voor JPEG-only bij het embedden van art (kleiner) of moet PNG behouden blijven?
- Gewenste standaardsortering in de mapweergave: op bestandsnaam of op tracknummer uit de tags?
