# Sleeve

Web-based tag editor voor MP3- en FLAC-bestanden, geschreven in Rust. Sleeve
draait als één Docker-container op een NAS met de muziekshare als gemount
volume, en wordt gebruikt via de browser op laptop, tablet of telefoon.

De weergavenaam is **Sleeve**; `sleeve-tag` is de technische naam (crate,
binary, Docker-image, containerhostnaam).

Sleeve staat volledig los van welke mediaserver dan ook: het schrijft
standaardconforme tags en gaat ervan uit dat een mediaserver als Navidrome de
wijzigingen zelf oppikt bij zijn periodieke scan.

Zie [PRD.md](PRD.md) voor de volledige functionele en technische eisen.

## Status

In aanbouw. De fasering en openstaande taken staan in `backlog/`.

## Ontwikkelen op macOS

Vereist: een Rust stable toolchain via [rustup](https://rustup.rs). De
`rust-toolchain.toml` in de repo zorgt dat de juiste toolchain en componenten
automatisch worden gebruikt.

```sh
# Bouwen
cargo build

# Draaien tegen een lokale testmap — nooit tegen de echte bibliotheek
MUSIC_ROOT=~/muziek-test cargo run

# Automatisch herbouwen tijdens ontwikkelen (optioneel)
cargo watch -x run
```

De UI staat daarna op <http://localhost:8080>. Start vanuit de projectroot: de
statische bestanden worden relatief aan de werkdirectory geserveerd.

`MUSIC_ROOT` wijst tijdens ontwikkelen naar een testmap op de Mac. In de
container is `MUSIC_ROOT` altijd `/music`; het pad van de share op de NAS is
uitsluitend de linkerkant van de volume-mount.

## Configuratie

Alle configuratie komt uit omgevingsvariabelen. Dezelfde waarden zijn ook als
CLI-flag beschikbaar (`--music-root`, `--port`, …), wat handig is bij lokaal
ontwikkelen; `sleeve-tag --help` toont ze.

| Variabele | Standaard | Betekenis |
|---|---|---|
| `MUSIC_ROOT` | — (verplicht) | Pad naar de muziekbibliotheek. Moet bestaan en een map zijn. In de container altijd `/music`. |
| `PORT` | `8080` | Poort waarop de webserver luistert. |
| `PUID` | `1000` | UID waaronder bestanden worden weggeschreven. |
| `PGID` | `10` | GID waaronder bestanden worden weggeschreven. |
| `MAX_ART_SIZE` | `1000x1000` | Maximale resolutie van embedded album art. Ook `1000` is geldig; verkleinen behoudt de beeldverhouding. |
| `LOG_LEVEL` | `info` | Logniveau voor `tracing`. Een lege waarde valt terug op `info`. |
| `BACKUP_ON_WRITE` | `false` | Plaatst bij elke schrijfactie een `.bak` naast het bestand. Accepteert `true`/`false`, `1`/`0`, `yes`/`no`, `on`/`off`. |

Een ontbrekende of ongeldige waarde laat de app bij start stoppen met een
melding die de variabele bij naam noemt — een verkeerd ingestelde container
faalt dus meteen, en niet pas bij de eerste schrijfactie. De effectieve
configuratie wordt bij start gelogd.

## Kwaliteitspoort

Deze drie commando's moeten groen zijn voordat werk als afgerond geldt:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Tests draaien altijd tegen tijdelijke mappen met ingecheckte fixtures en raken
de echte muziekbibliotheek nooit aan.

## Testfixtures

Onder `tests/fixtures/` staan kleine audiobestanden (één seconde stilte, samen
zo'n 84 KB) die de tagvarianten dekken waar de code mee om moet gaan:

| Fixture | Bijzonderheid |
|---|---|
| `untagged.mp3` / `untagged.flac` | geen enkele tag |
| `tagged.mp3` / `tagged.flac` | volledige tagset uit het tagmodel |
| `tagged-with-art.mp3` / `tagged-with-art.flac` | idem, plus embedded front cover |
| `id3v1-only.mp3` | uitsluitend een ID3v1-tag, geen ID3v2 |
| `id3v1-inconsistent.mp3` | ID3v1 en ID3v2 met verschillende waarden |
| `cover.jpg` / `cover.png` | losse afbeeldingen voor het testen van uploads |

De laatste twee MP3-varianten bestaan omdat het PRD eist dat een ID3v1-tag nooit
inconsistent achterblijft; zonder zo'n bestand is die regel niet te testen.

Twee dingen doet ffmpeg anders dan gangbare taggers, en daar corrigeert het
script voor: het schrijft een comment als `TXXX` in plaats van `COMM` (het script
plakt daarom zelf een `COMM`-frame aan de MP3's), en het gebruikt in FLAC het
veld `DESCRIPTION` waar Picard `COMMENT` schrijft (de tagmodule leest beide).

Gebruik ze via `testfixtures::copy_to_tempdir(...)`, dat een kopie in een
wegwerpmap zet. Rechtstreeks tegen een fixture in de repo werken is fout: een
schrijftest zou het origineel dan wijzigen.

Opnieuw genereren (alleen nodig bij een nieuwe variant):

```sh
tests/fixtures/genereer-fixtures.sh   # vereist ffmpeg
```

## Productie-image bouwen

De NAS draait `linux/amd64`, de ontwikkelmachine is Apple Silicon. Het image
wordt daarom expliciet voor dat platform gebouwd:

```sh
docker buildx build --platform linux/amd64 -t sleeve-tag:dev .
```

De build-stage draait daarbij geëmuleerd; reken op enkele minuten voor een
schone build. Een wijziging in alleen de broncode hergebruikt de laag met de
gecompileerde dependencies en is daarna een stuk sneller.

Het resultaat is een statisch gelinkte binary (`x86_64-unknown-linux-musl`) in
een distroless-image: geen shell, geen package manager, geen Rust-toolchain.
Het image is ongeveer 6,5 MB.

### Als de build wordt afgeschoten

Onder emulatie is elk `rustc`-proces fors zwaarder dan native. Op een builder met
weinig geheugen wordt een crate dan door de OOM-killer gestopt, zichtbaar als
`signal: 9, SIGKILL` halverwege het compileren. De Dockerfile beperkt daarom het
aantal parallelle jobs tot twee. Heeft de builder ruim geheugen, dan mag dat
omhoog:

```sh
docker buildx build --platform linux/amd64 --build-arg BUILD_JOBS=8 -t sleeve-tag:dev .
```

### Naar de NAS brengen

Zolang er nog geen image in een registry staat, gaat het handmatig:

```sh
docker save sleeve-tag:dev | ssh <nas> docker load
```

Vanaf de release-workflow haalt de NAS het image op met `docker compose pull`.

### Podman in plaats van Docker

Podman leest dezelfde Dockerfile en kent dezelfde vlaggen; vervang `docker` door
`podman` (zonder `buildx`):

```sh
podman build --platform linux/amd64 -t sleeve-tag:dev .
```

## Projectstructuur

| Module | Verantwoordelijkheid |
|--------|----------------------|
| `config` | Configuratie uit omgevingsvariabelen |
| `fs` | Padvalidatie en containment binnen `MUSIC_ROOT`; de enige plek die een gebruikerspad naar een filesystem-pad vertaalt |
| `tags` | Genormaliseerd tagmodel en alle tag-I/O (de enige plek die `lofty` gebruikt) |
| `art` | Album art decoderen, verkleinen en encoderen (de enige plek die pixels aanraakt) |
| `browse` | Weergavemodel van één map: paden en tags samengebracht tot wat de templates tonen |
| `web` | Axum-router, handlers en askama-templates |

Daarnaast: `templates/` met de askama-templates en `static/` met de assets.

## Mapbrowser

De startpagina is de wortel van `MUSIC_ROOT`; elke map eronder heeft een eigen
URL onder `/map/`, bijvoorbeeld `/map/Artiest/Album`. Het pad in de URL is altijd
relatief aan `MUSIC_ROOT` — het absolute pad van de NAS komt niet in de
interface of in een link terecht. Boven de wortel navigeren kan niet: `fs::`
weigert zo'n pad met een 403.

Per map worden de submappen getoond en de bestanden waarvan de tags te lezen
zijn, met tracknummer, titel, artiest, album, duur en formaat. Bestanden worden
gesorteerd op het tracknummer uit de tags, met de bestandsnaam als terugval
wanneer een tracknummer ontbreekt; bestanden zonder nummer staan achteraan. Dat
beantwoordt het open punt over sortering uit PRD §12.

Het zoekveld filtert binnen de huidige map op bestandsnaam of titel, en op de
naam van submappen. Met JavaScript ververst HTMX tijdens het typen alleen de
lijst (de server geeft dan het fragment `templates/listing.html` terug, herkend
aan de `HX-Request`-header); zonder JavaScript is het een gewone GET naar
dezelfde URL met `?q=`, met hetzelfde resultaat als hele pagina.

Er is bewust geen bibliotheek-index: de tags worden per map gelezen op het moment
dat de pagina wordt opgevraagd. Dat lezen is blokkerende I/O en gebeurt daarom in
`spawn_blocking`, buiten de async-runtime.

### Album art in de lijst

De embedded hoes van een bestand komt van `/art/<pad>`:

| URL | Antwoord |
|---|---|
| `/art/<pad>` | de hoes ongewijzigd, met het MIME-type zoals het in het bestand staat |
| `/art/<pad>?size=thumb` | een JPEG van hoogstens 160 px per as |
| een bestand zonder hoes | `404` met een leesbare melding |

De maplijst vraagt de thumbnail-variant op. Dertig volledige hoezen van elk een
halve megabyte naar een telefoon sturen voor een vakje van veertig pixels zou de
pagina onbruikbaar maken; het verkleinen gebeurt bij het verzoek, want er is in
het MVP bewust geen cache-laag. De afbeeldingen worden lazy geladen en hebben
vaste afmetingen, zodat de lijst compleet op het scherm staat voordat de eerste
hoes binnen is en er daarna niets verschuift. Bestanden zonder hoes krijgen een
placeholder en doen geen verzoek dat toch niets zou opleveren.

De antwoorden dragen `Cache-Control: no-cache`: na een latere schrijfactie mag
de browser geen oude hoes blijven tonen.

## Frontend zonder build-stap

De UI wordt serverside gerenderd met askama plus HTMX. Er is bewust geen
node-toolchain en geen frontend-build: `cargo build` levert alles.

Alle assets worden lokaal meegeleverd, zodat de app werkt op een NAS zonder
internetverbinding. Een test controleert dat de pagina naar geen enkele externe
host verwijst.

| Bestand | Herkomst |
|---|---|
| `static/htmx.min.js` | htmx 2.0.10, opgehaald van unpkg |
| `static/app.css` | eigen stijl, telefoon-eerst |
| `static/favicon.svg` | eigen |

Bij het bijwerken van htmx: vervang het bestand, noteer de nieuwe versie hier, en
draai de tests.
