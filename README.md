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
| `PUID` | `1000` | UID waaronder bestanden worden weggeschreven. Zie [Rechten en eigenaarschap](#rechten-en-eigenaarschap). |
| `PGID` | `10` | GID waaronder bestanden worden weggeschreven. Zie [Rechten en eigenaarschap](#rechten-en-eigenaarschap). |
| `MAX_ART_SIZE` | `1000x1000` | Maximale resolutie van embedded album art. Ook `1000` is geldig; verkleinen behoudt de beeldverhouding. |
| `ART_QUALITY` | `85` | JPEG-kwaliteit (1–100) waarmee een verkleinde hoes wordt gecodeerd. Daarboven lopen de bytes hard op zonder zichtbaar verschil, daaronder worden vlakken korrelig. |
| `MAX_UPLOAD_MB` | `10` | Bovengrens aan een geüploade afbeelding, in megabytes. Ruim boven wat een hoes nodig heeft, ruim onder wat een NAS met weinig geheugen plat legt. |
| `LOG_LEVEL` | `info` | Logniveau voor `tracing`. Een lege waarde valt terug op `info`. De tagbibliotheek staat standaard op `error`; zie [Eén bestand, één tagblok](#één-bestand-één-tagblok). |
| `BACKUP_ON_WRITE` | `false` | Plaatst bij elke schrijfactie een `.bak` naast het bestand. Accepteert `true`/`false`, `1`/`0`, `yes`/`no`, `on`/`off`. |

Een ontbrekende of ongeldige waarde laat de app bij start stoppen met een
melding die de variabele bij naam noemt — een verkeerd ingestelde container
faalt dus meteen, en niet pas bij de eerste schrijfactie. De effectieve
configuratie wordt bij start gelogd.

## Rechten en eigenaarschap

Sleeve schrijft in de muziekshare van de NAS. Bestanden die daar met een andere
eigenaar of groep terechtkomen dan de rest van de share, zijn voor Navidrome of
voor de gebruiker zelf ineens onbruikbaar. Op deze UGREEN is dat UID `1000` en
GID `10`.

### Het proces draait niet als root

Het image zet `USER 1000:10`, dus ook een kale `docker run` start al als
niet-root. In compose wordt dat overschreven met de waarden uit de omgeving:

```yaml
user: "${PUID:-1000}:${PGID:-10}"
environment:
  PUID: 1000
  PGID: 10
```

`user:` en de twee omgevingsvariabelen horen dezelfde waarden te hebben:
`user:` bepaalt wat er werkelijk gebeurt, `PUID`/`PGID` zijn wat de app
verwacht. Lopen ze uiteen, dan zegt de app dat bij start (zie hieronder).

### Waarom `user:` en geen entrypoint-script

De gangbare aanpak in NAS-images is een entrypoint dat als root start, met
`chown`/`usermod` de gewenste UID en GID inregelt en dan naar die gebruiker
afdaalt. Dat kan hier niet, en dat is een bewuste keuze:

- De runtime is `gcr.io/distroless/static-debian12`. Er zit geen shell in, geen
  `chown` en geen `su-exec`. Zo'n script terugbrengen betekent een shell — en
  daarmee een grotere aanvalsoppervlakte — het image in halen.
- Zo'n script móet als root beginnen om zijn werk te kunnen doen. Precies wat
  de eis "draait niet als root" wil uitsluiten. Met `user:` is het proces geen
  moment root geweest.
- Docker kan het gewoon zelf. Een entrypoint zou een taak overnemen die de
  runtime al betrouwbaar uitvoert.

Het `chown`-deel van zo'n script is hier bovendien niet nodig: Sleeve maakt
geen eigen datamap aan, en bij het vervangen van een bestand nemen eigenaar,
groep en rechten van het origineel over (zie
[Schrijven zonder iets kwijt te raken](#schrijven-zonder-iets-kwijt-te-raken)).

### Wat de app bij start controleert

Een niet-root proces kan zijn eigen UID niet veranderen. `PUID`/`PGID` worden
dus niet dóór de app toegepast maar dóór de runtime; de app (`startup::`)
toetst bij start of dat ook zo is uitgepakt. Ze zet daarvoor één sondebestand
in `MUSIC_ROOT` neer, leest de eigenaar ervan terug en ruimt het meteen op. Dat
is bewust geen controle op de mode-bits: die liegen zodra er ACL's op de share
staan of de map setgid is, en juist op een NAS is dat eerder regel dan
uitzondering. Een bestand echt aanmaken beantwoordt beide vragen op de enige
manier die telt — mág er geschreven worden, en wie is straks de eigenaar.

| Situatie | Logregel | Gevolg |
|---|---|---|
| Schrijfbaar, uid/gid gelijk aan `PUID`/`PGID` | `INFO … MUSIC_ROOT is schrijfbaar` met `uid` en `gid` | — |
| Schrijfbaar, uid/gid wijken af | `WARN … PUID/PGID` met beide waarden | De app draait door; controleer `user:` in compose |
| Niet schrijfbaar | `ERROR … MUSIC_ROOT is niet schrijfbaar; opslaan zal mislukken` | De app draait door: bladeren en tags bekijken werkt gewoon |

Een read-only share laat de container dus niet omvallen. Dat is opzet: een
draaiende UI met een duidelijke melding is makkelijker te diagnosticeren dan een
container in een herstartlus, en het onderscheid met een echte
configuratiefout — die de app wél meteen laat stoppen — blijft zo zichtbaar.
Wat de app níet doet, is wachten met melden tot de eerste bewerking wordt
opgeslagen.

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
| `untagged.mp3` / `untagged.flac` | geen gemodelleerde tags; de FLAC draagt wel een `ENCODER`-veld dat ffmpeg altijd schrijft |
| `tagged.mp3` / `tagged.flac` | volledige tagset uit het tagmodel |
| `tagged-with-art.mp3` / `tagged-with-art.flac` | idem, plus embedded front cover |
| `id3v1-only.mp3` | uitsluitend een ID3v1-tag, geen ID3v2 |
| `id3v1-inconsistent.mp3` | ID3v1 en ID3v2 met verschillende waarden |
| `id3-in-flac.flac` | een ID3v2-blok vóór de Vorbis-comments, met een andere titel |
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

### Draaien op de NAS

De meegeleverde `docker-compose.yml` is bedoeld om zonder aanpassingen te
werken; alles wat per NAS verschilt staat in een `.env` ernaast, die niet in
Git zit.

```sh
cp .env.example .env       # en zet MUSIC_HOST_PATH op het pad van de share
docker compose up -d
docker compose logs -f sleeve-tag
```

De UI staat daarna op `http://<nas>:8080`, op het LAN en via Tailscale. Sleeve
heeft zelf geen login: de afscherming gebeurt op netwerkniveau, dus deze poort
hoort nooit naar internet open te staan.

`MUSIC_HOST_PATH` is de enige verplichte waarde. Het is ook het enige
NAS-specifieke gegeven in de opstelling: het staat links van de dubbele punt in
de volume-mount, en in de container heet de map altijd `/music`. Wat de rest
van de variabelen doet, staat in [Configuratie](#configuratie) en als
commentaar in `.env.example`.

De container draait als `${PUID}:${PGID}` — zie
[Rechten en eigenaarschap](#rechten-en-eigenaarschap) voor waarom dat via
`user:` gaat en niet via een entrypoint.

### De healthcheck

Compose bevraagt `/healthz`, maar er zit geen `curl` in een distroless-image.
De container roept daarom dezelfde binary nog eens aan, in een tweede
bedrijfsmodus:

```sh
sleeve-tag --health    # exitcode 0 = gezond, 1 = niet gezond
```

Die doet één verzoek aan `127.0.0.1:$PORT/healthz` en zegt verder niets; alleen
de exitcode telt, en dat is precies wat Docker nodig heeft. De modus draait
vóór de configuratie wordt ingelezen en heeft alleen `PORT` nodig — een
healthcheck hoort niet te struikelen over iets anders dan de vraag die hij
stelt. Samen met `restart: unless-stopped` komt een vastgelopen container zo
vanzelf weer terug.

### Naar de NAS brengen

Zolang er nog geen image in een registry staat, gaat het handmatig:

```sh
docker save sleeve-tag:dev | ssh <nas> docker load
```

Met Podman als bouwer moet de volledige naam mee, anders vindt `save` het image
niet:

```sh
podman save localhost/sleeve-tag:dev | ssh <nas> docker load
```

Het image komt aan de andere kant binnen als `localhost/sleeve-tag:dev`. De
compose-file verwacht `sleeve-tag:dev`, dus hernoem het één keer op de NAS:

```sh
ssh <nas> docker tag localhost/sleeve-tag:dev sleeve-tag:dev
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
| `startup` | De startcontrole: is `MUSIC_ROOT` schrijfbaar, en met welke eigenaar en groep |
| `health` | De healthcheck-modus (`--health`): één verzoek aan `/healthz`, alleen een exitcode terug |
| `fs` | Padvalidatie en containment binnen `MUSIC_ROOT`; de enige plek die een gebruikerspad naar een filesystem-pad vertaalt |
| `tags` | Genormaliseerd tagmodel en alle tag-I/O (de enige plek die `lofty` gebruikt) |
| `art` | Album art decoderen, verkleinen en encoderen (de enige plek die pixels aanraakt) |
| `checks` | Signalering van ontbrekende en onderling afwijkende tags; leest en schrijft niets |
| `atomic` | De schrijfstrategie: de inhoud van een bestand vervangen zonder het kwijt te raken, en als enige uitzondering een nieuw bestand neerzetten (`cover.jpg`) |
| `browse` | Weergavemodel van één map: paden en tags samengebracht tot wat de templates tonen |
| `edit` | Het bewerkformulier: vertaling tussen het tagmodel en de tekst in een formulier |
| `batch` | De albumweergave: een selectie bestanden, de gedeelde velden, de overrides per bestand en de hulpacties |
| `casing` | Hoofdlettergebruik van een tagwaarde normaliseren; kent geen tags en geen bestanden |
| `cover` | De hoespagina van één bestand: formaat, afmetingen en grootte als leesbare tekst, en wat een upload opleverde |
| `web` | Axum-router, handlers en askama-templates |

Daarnaast: `templates/` met de askama-templates en `static/` met de assets.

## Schrijven zonder iets kwijt te raken

De bibliotheek op de NAS is niet opnieuw op te bouwen, en de container kan
midden in een schrijfactie worden afgebroken. Elke schrijfactie loopt daarom via
`atomic::replace`, dat de volgorde vastlegt in plaats van hem aan de aanroeper
over te laten:

1. Het origineel wordt gekopieerd naar een tijdelijk bestand **in dezelfde map**
   — alleen dan is het hernoemen straks atomair. Over een filesystem-grens heen
   doet `rename` een kopieeractie, en juist dat moment van halve inhoud moet
   uitgesloten blijven.
2. De aanroeper past dat tijdelijke bestand aan. Het is een exacte kopie, zodat
   tag-I/O een echt audiobestand heeft om mee te beginnen.
3. De aanroeper leest het opnieuw in en keurt het goed. Zegt hij nee, dan gaat
   er niets over het origineel heen.
4. Eigenaar, groep en rechten van het origineel gaan mee. Lukt dat niet, dan
   gaat de schrijfactie niet door: stilletjes de eigenaar van een bestand
   veranderen is precies wat het PRD verbiedt. Op de NAS, met `PUID`/`PGID`
   gelijk aan die van de share, doet dat geval zich niet voor.
5. Bij `BACKUP_ON_WRITE=true` komt er een `<naam>.bak` naast te staan, met de
   inhoud van vóór deze schrijfactie. Standaard staat dat uit, om de share niet
   te vervuilen.

### Eén bestand, één tagblok

Een bestand kan meer dan één tagblok dragen. Een MP3 met ID3v2 én ID3v1 is niet
netjes maar wel gangbaar; een FLAC met een ID3-blok ervóór hoort helemaal niet te
bestaan — de FLAC-standaard kent alleen Vorbis-comments — maar oudere rippers
maken ze bij bosjes.

Sleeve leest en schrijft alleen het blok dat bij het formaat hoort. Het andere
blok blijft dus staan met de oude waarden, en welke van de twee een speler kiest,
is niet te voorspellen. Daarom haalt `tags::` bij het schrijven weg wat er niet
naast hoort: bij een MP3 de ID3v1-tag, bij een FLAC een ID3-blok.

Drie regels waar het zich aan houdt:

- **Alleen wanneer er tóch geschreven wordt.** Verandert er niets aan de tags,
  dan blijft het bestand onaangeraakt — ook met zo'n blok erin. Gigabytes
  herschrijven om iets op te ruimen wat je niet hebt gewijzigd, is precies de
  ongevraagde wijziging die het PRD verbiedt.
- **Nooit stilzwijgend.** Wat er verdwijnt, staat in de melding boven het
  formulier en in het rapport van een batch.
- **Zichtbaar vóór je begint.** De maplijst markeert zo'n bestand met "tagblok
  dat er niet hoort", en op de pagina met ruwe tags staat elk blok apart, met de
  waarschuwing erbij welke er niet in thuishoort.

De tagbibliotheek waarschuwt zelf bij élk inlezen van zo'n bestand. Op een map
met tientallen albums van dezelfde ripper verdringt dat alles wat er wél toe
doet, dus staat die bibliotheek standaard op `error`. Wie de meldingen tóch wil
zien: `LOG_LEVEL=info,lofty=warn`.

### Waarom een groot bestand minuten kost

Stap 1 kopieert het volledige bestand. Bij een album van 30 losse tracks merk je
daar niets van, maar een 2LP-rip als één FLAC van enkele gigabytes is een paar
minuten bezig — en dat is de prijs van de garantie dat het origineel nooit
halverwege kapot is. Komt daar een FLAC zonder `PADDING`-blok bij, dan moet ook
de tagschrijver de hele stream herschrijven; dat blok wordt daarna toegevoegd,
dus een tweede bewerking van hetzelfde bestand is sneller.

Zolang zo'n schrijfactie loopt, toont de knop een spinner en neemt het formulier
geen tweede klik meer aan (`static/app.js`). Zonder JavaScript werkt alles
gewoon — dan blijft alleen de bezig-weergave achterwege.

## Hoe het eruitziet

De vormgeving komt uit het design system **Nocturne**. De kleuren, tonenreeksen,
ruimtematen, hoeken en schaduwen daarvan staan als tokens boven in
`static/app.css`, en dat is de enige plek in het project waar een kleurwaarde
staat; de componentregels eronder noemen alleen tokens. Wie de vormgeving wil
bijstellen, hoeft daarvoor geen enkele component aan te raken.

Twee dingen die daaruit volgen en er anders uitzien dan gewoonlijk: een knop is
een omlijning en geen gevuld vlak — ook de primaire, die zich onderscheidt door
de accentkleur en niet doordat hij is ingekleurd — en een scheidingsregel vloeit
aan beide uiteinden uit in plaats van er droog af te breken.

Sleeve heeft een donkere en een lichte weergave. De kopbalk biedt de keuze en
onthoudt hem in de browser (`localStorage`); zolang er geen keuze is gemaakt,
volgt de pagina de systeemvoorkeur. Een klein script in de `<head>` zet een
bewaarde keuze terug vóór het eerste renderen, zodat de pagina niet eerst in de
andere modus opflitst. Zonder JavaScript blijft de systeemvoorkeur gelden en
staat de schakelaar er niet — een knop die niets doet is erger dan geen knop.

Het lettertype van het systeem is Inter. Dat wordt niet opgehaald: de NAS heeft
geen internetverbinding, dus Inter wordt alleen gebruikt wanneer hij op het
apparaat staat en er wordt anders teruggevallen op de systeemletter. Alles wat
de pagina nodig heeft, komt van de NAS zelf.

## Wat de browser er nog bij doet

`static/app.js` is de enige JavaScript in het project, en alles erin is een
toevoeging: valt het weg, dan werkt elk formulier zoals het altijd deed.

| | |
|---|---|
| **Bezig-weergave** | een knop die schrijft toont een spinner en neemt geen tweede klik meer aan |
| **Een hoes neerslepen** | een JPEG of PNG op het uploadvak slepen vult het bestandsveld, met een miniatuur erbij |
| **Idem op het hoesje** | op de bewerkpagina is het hoesje zelf ook een doel; daar verschijnt dan één knop, "In dit bestand zetten" |
| **Idem voor een selectie** | in de voorbeeldweergave van een batch, voor de bestanden die je hebt aangevinkt |
| **Donker of licht** | de keuze in de kopbalk, onthouden in de browser; zonder keuze geldt de systeemvoorkeur |

Slepen verandert niets aan wat er daarna gebeurt: de vinkjes en de knoppen
bepalen nog steeds wat er met de afbeelding wordt gedaan, en er wordt niets
geschreven voordat je op een knop drukt. Meerdere bestanden tegelijk of iets
anders dan een JPEG of PNG levert meteen een melding op, in plaats van een
mislukte upload.

Om die reden staat de uitnodiging om te slepen `hidden` in de template en haalt
het script hem tevoorschijn: een hint die nergens toe leidt is erger dan geen
hint.

Een hoes voor een selectie hoort bij de **voorbeeldweergave** van een batch en
niet bij de albumtabel zelf. Die tabel post zichzelf bij elk vinkje opnieuw; een
afbeelding van megabytes zou dan bij iedere klik meereizen, en de server kan een
bestandsveld daarna niet terugvullen. De voorbeeldstap leidt rechtstreeks naar
het schrijven, dus daar gaat de afbeelding precies één keer over de lijn — en
kan hij onderweg ook niet verdwijnen. Dat het voorbeeld tóch per bestand kan
zeggen of een hoes wordt *toegevoegd* of *vervangen*, komt doordat dat volgt uit
wat er nu in het bestand zit.

Het hoesje op de bewerkpagina is een snelkoppeling naar de gewone route en geen
tweede manier om te schrijven: het formulier eromheen post naar dezelfde
`/hoes/{pad}` als de hoespagina. Het staat náást het tagformulier en niet erin —
geneste formulieren bestaan niet in HTML, en een hoesactie hoort geen tags mee
te sturen. Voor de hele map of een losse `cover.jpg` blijft de hoespagina de
plek; die keuzes horen niet in een snelkoppeling thuis.
6. Pas dan wordt het tijdelijke bestand over het origineel hernoemd.

Gaat er onderweg iets mis — ook bij een paniek — dan blijft het origineel
byte-voor-byte zoals het was en verdwijnt het tijdelijke bestand. Dat laatste
hangt aan een `Drop`-guard, zodat geen enkel foutpad er zelf aan hoeft te
denken. Het tijdelijke bestand heet `.<naam>.<pid>.sleeve-tmp`: verborgen, zodat
de mapbrowser het overslaat als er onverhoopt toch een blijft liggen.

Elke geslaagde schrijfactie wordt gelogd met het pad en de gewijzigde velden.
Een mislukte hervalidatie krijgt een eigen foutregel: het origineel is dan heel,
maar er is wel zojuist een onbruikbaar bestand geproduceerd.

## Tags wegschrijven

`tags::write` neemt het genormaliseerde model aan en zet het in het bestand. De
regels komen uit PRD §7 en zijn strenger dan ze op het eerste gezicht lijken,
omdat Navidrome dezelfde bestanden leest.

- **Alleen gemodelleerde velden worden aangeraakt.** Er wordt begonnen bij de
  tag die al in het bestand staat, niet bij een lege. Alles wat Sleeve niet
  kent — een `TPUB` van je platenlabel, een `TSRC`, een `ENCODER` — blijft
  gewoon staan. Ook de embedded hoes overleeft een tagwijziging; een test
  bewaakt beide.
- **Leeg betekent verwijderen.** Een veld dat leeg is (of alleen spaties
  bevat) verdwijnt uit het bestand in plaats van als lege waarde achter te
  blijven. `Tags::normalized` legt dat op één plek vast, zodat geen enkel
  formulier er nog aan hoeft te denken.
- **MP3 wordt ID3v2.4 met UTF-8**, ook wanneer het bestand daarvoor iets anders
  had. Een bestaande ID3v1-tag wordt verwijderd: die kan maar dertig tekens per
  veld en zou na een wijziging iets anders zeggen dan ID3v2. Verwijderen maakt
  die tegenstrijdigheid onmogelijk, en dat is veiliger dan hem synchroniseren.
- **Samengestelde velden volgen hun formaat.** ID3v2 krijgt `TRCK` en `TPOS` als
  `nummer/totaal`; Vorbis-comments krijgen `TRACKNUMBER` en `TRACKTOTAL` apart.
- **Het jaar gaat naar `TDRC`/`DATE`**, en een los `YEAR`-veld wordt opgeruimd.
  Commentaar gaat naar `COMMENT`, waarbij `DESCRIPTION` (wat ffmpeg schrijft)
  verdwijnt. Twee plekken met een verschillende waarde is precies de verwarring
  die deze app moet wegnemen.
- **De audio blijft bit-identiek.** Een tagwijziging raakt de audioframes niet;
  een test knipt de tagblokken eraf en vergelijkt wat er overblijft.
- **Verandert er niets, dan gebeurt er niets.** Een bestand herschrijven dat
  gelijk blijft is een ongevraagde wijziging: de wijzigingsdatum verspringt en
  Navidrome gaat er opnieuw naar kijken zonder dat er iets te zien valt.

### Twee omwegen om lofty 0.25.1 heen

Beide zijn met een test vastgelegd, zodat ze opvallen als een nieuwere versie ze
oplost:

- `WriteOptions::remove_others` bestaat wel maar wordt nergens uitgelezen; de
  vlag doet niets. De ID3v1-tag wordt daarom met de hand verwijderd.
- `TagType::remove_from_path` opent het bestand alleen-lezen en probeert er
  vervolgens in te schrijven, wat altijd mislukt. Sleeve opent het bestand zelf
  lees-schrijf en gebruikt `remove_from`.

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

### Signalering van tagproblemen

De lijst wijst zelf aan waar iets mis is, zodat je niet elk bestand hoeft te
openen om dat te ontdekken. Dit is **puur informatief**: Sleeve past nooit
ongevraagd iets aan.

Per bestand verschijnt een label bij een ontbrekende titel, artiest, album of
hoes, en bij een ontbrekend of dubbel tracknummer. Boven de lijst staat wat er
tussen de bestanden onderling niet klopt: meer dan één albumtitel, albumartiest
of jaartal in dezelfde map, hoeveel bestanden geen tracknummer hebben, en welke
tracknummers meer dan eens voorkomen.

Een ontbrekende waarde geldt niet als tegenstrijdigheid — dat is een gebrek van
dat ene bestand. Zonder dat onderscheid zou elke map met één ongetagd bestand
als inconsistent gelden.

De labels zijn zichtbare tekst en geen tooltip: op een telefoon is er geen
hover. De beoordeling loopt over de héle map, ook wanneer er gefilterd wordt —
aan de map verandert niets doordat je zoekt.

### Bewerken

De titel in de maplijst opent `/bewerk/<pad>`: een formulier met de twaalf
kernvelden, gevuld met wat er nú in het bestand staat.

Opslaan schrijft de wijzigingen weg, leest het bestand **opnieuw in**, en toont
die waarden. Dat is het hele punt: de bevestiging komt uit het bestand en niet
uit wat je zojuist intikte, want alleen dan zegt hij iets. Een veld leegmaken
verwijdert die tag — dat staat boven het formulier, want het is het enige gedrag
dat kan verrassen.

De numerieke velden worden gecontroleerd vóórdat er iets naar het bestand gaat.
Bij een fout — in de invoer of tijdens het schrijven — blijft het bestand
onaangetast, staat er een uitleg boven het formulier, en blijven de ingevulde
waarden staan zodat er niets overgetypt hoeft te worden.

Er wordt bewust niet doorverwezen na het opslaan. Een herlaadactie stuurt
hetzelfde formulier nog eens, en dat is ongevaarlijk: `tags::write` raakt het
bestand niet aan wanneer er niets verandert. Dat scheelt een flash-mechanisme om
de bevestiging te bewaren.

### Albumweergave: een selectie in één keer zetten

Bestand voor bestand corrigeren is te traag voor een heel album. "Meerdere
bestanden bewerken" in de maplijst opent `/album/<pad>`: dezelfde bestanden,
maar dan als tabel met een vinkje per rij en de vijf velden die een album deelt
— albumartiest, album, jaar, genre en discnummer.

Bij het openen is alles geselecteerd; met "Alles selecteren" en "Niets
selecteren" is dat in één klik terug te zetten. De selectie, de ingevulde velden
en de wissen-vinkjes zitten in één formulier en gaan samen mee met elk verzoek.
Het aanpassen van de selectie laat de invoer dus staan, en andersom.

Het invoerveld wordt **nooit voorgevuld** met de huidige waarde. Daardoor
betekent leeg altijd hetzelfde: dit veld blijft in elk bestand zoals het is.
Wissen is een aparte keuze, met een vinkje naast het veld. Wat er nú in de
selectie staat, is als tekst onder het veld te lezen: één gedeelde waarde, leeg,
of "verschillend" met de waarden die voorkomen. Onder het formulier staat per
veld wat er bij het opslaan zou gebeuren.

Titel, tracknummer en albumartiest horen bij het bestand en zijn daarom **in de
tabel zelf** in te tikken: één invoerveld per rij, met de huidige waarde als
grijze tekst erin. Dezelfde regel geldt er: leeg laten verandert niets. Een
override is voor dat ene bestand bedoeld en wint daarom van wat de gedeelde
velden ervoor zouden doen. De ingetikte waarden blijven staan bij het wisselen
van selectie; wat in een niet-geselecteerde rij staat, zegt erbij dat het niet
wordt opgeslagen. Een onleesbaar tracknummer wordt bij die rij gemeld en houdt
alleen die rij tegen — de andere rijen en de gedeelde velden blijven bruikbaar.

Albumartiest staat zowel bij de gedeelde velden als in de tabel, en dat is geen
vergissing: hij is meestal voor het hele album gelijk, maar de hulpactie hieronder
zet er per bestand een eigen waarde in.

#### Hulpacties

Drie correcties die met de hand te veel werk zijn, en één om ze terug te draaien:

| Knop | Wat hij doet |
|------|--------------|
| Hernummeren | Nummert de selectie opeenvolgend, in de volgorde van de tabel — niet in die van de bestaande tracknummers, want juist die kloppen niet |
| Artiest → albumartiest | Zet per bestand de artiest als albumartiest klaar; een bestand zonder artiest wordt overgeslagen |
| Hoofdletters normaliseren | Stelt een leesbare schrijfwijze voor van titel en albumartiest per bestand, en van album en genre wanneer de hele selectie er dezelfde waarde heeft |
| Invoer leegmaken | Haalt alles wat er ingevuld of voorgesteld is weer weg; de selectie blijft staan |

Een hulpactie **vult alleen invoervelden**. Er gaat geen bestand open en er wordt
niets geschreven: wat de actie voorstelt staat daarna gewoon in de velden, is met
de hand aan te passen, en gaat met "Invoer leegmaken" in één klik weer weg. Een
voorstel dat gelijk is aan wat er al staat, wordt niet ingevuld — dat is geen
voorstel.

Het normaliseren zelf zit in `casing`. Elk woord krijgt een hoofdletter, behalve
de kleine woorden (`de`, `van`, `the`, `at`, …) middenin. Wat er níét gebeurt is
minstens zo belangrijk: een korte reeks kapitalen blijft een afkorting (`DJ`,
`BBC`, `R.E.M.`, `AC/DC`), en een woord dat zijn eigen hoofdletters draagt blijft
staan (`McCartney`, `iPhone`, `d'Angelo`). Vijf letters of meer in kapitalen is
geen afkorting maar geschreeuw, en wordt wél omgezet.

#### Voorbeeld, opslaan en het resultaat

"Voorbeeld en opslaan" leidt naar de **voorbeeldweergave**: per bestand welke
velden veranderen, met de oude waarde doorgestreept en de nieuwe ernaast. Een
veld dat verdwijnt staat er met zoveel woorden bij als verwijdering — dat is de
ingrijpendste wijziging die een batch kan maken. Bestanden waar niets mee gebeurt
staan er ook in, met de mededeling dat ze niet worden aangeraakt: dát er niets
gebeurt is de helft van wat een voorbeeld moet vertellen.

Dit is de **enige route** waarlangs een batch wordt weggeschreven. De hele
formulierstaat gaat als verborgen velden mee, dus er kan niets anders opgeslagen
worden dan wat er te zien is; wie iets wil wijzigen, gaat eerst met "Annuleren"
terug naar het formulier, en op dat moment is er nog niets geschreven. Klopt de
invoer niet, dan verschijnt de opslaanknop niet: een plan met een fout erin wordt
niet half uitgevoerd.

Opslaan gaat **bestand voor bestand**. Elk bestand wordt vlak voor het schrijven
opnieuw ingelezen, zodat het plan op de werkelijke inhoud wordt toegepast en niet
op een leesronde van een minuut geleden. Een fout bij één bestand stopt de rest
niet: na afloop staat er per bestand of het is bijgewerkt (met welke velden),
ongemoeid is gebleven, of niet opgeslagen kon worden en waarom. De tabel eronder
komt uit een verse leesronde en toont dus wat er werkelijk in de bestanden staat.

Elke andere POST naar deze pagina schrijft **niets**: de selectie bijwerken, een
hulpactie uitvoeren, een veld invullen — die bouwen de pagina alleen opnieuw op.
Een integratietest controleert dat de bestanden daarbij byte voor byte
onaangeroerd blijven, en een tweede voert een batch uit op een map waarin één
bestand niet schrijfbaar is: de rest wordt bijgewerkt, dat ene blijft heel, en de
reden staat erbij.

De tabel is breder dan een telefoonscherm en scrollt daarom horizontaal binnen
zijn eigen rand; de kolom met de bestandsnaam blijft daarbij staan, want zonder
die naam is niet te zien wat je aanvinkt.

### Hoesweergave: wat er werkelijk in zit

Een thumbnail van veertig pixels verraadt niet of de hoes eronder 300×300 of
3000×3000 is, en of daar een halve megabyte in gaat zitten. `/hoes/<pad>`,
bereikbaar door op de hoes of de link "hoes" op de bewerkpagina te klikken, toont
de afbeelding zo groot als het scherm toelaat, met het formaat (JPEG, PNG, …),
de afmetingen in pixels en de bestandsgrootte erbij. Is de hoes niet vierkant,
dan staat dat er ook: vrijwel elke speler toont hem in een vierkant vak, en daar
wordt hij dan bijgesneden of uitgerekt.

Een bestand zonder hoes levert geen 404 op maar dezelfde pagina met de
mededeling dat er niets in zit — en met het formulier om er een toe te voegen.

#### Een hoes plaatsen of verwijderen

Op dezelfde pagina staat een uploadveld voor een JPEG of PNG, met twee knoppen:
**alleen dit bestand**, of **alle tracks in deze map**. Wat er gebeurt staat in
de knop zelf, zodat er geen keuzevakje bij hoeft dat je over het hoofd kunt zien.
Zit er al een hoes in, dan staan dezelfde twee knoppen er ook om hem te
verwijderen.

Het schrijven gaat bestand voor bestand via dezelfde atomische route als de
tags: naar een tijdelijk bestand, hervalideren door de hoes terug te lezen, en
pas dan hernoemen. Alleen de afbeelding verandert — de tekstuele tags blijven
zoals ze zijn, en andere afbeeldingen dan de front cover blijven staan. Een fout
bij één bestand houdt de rest niet tegen; na afloop staat er per bestand of het
is bijgewerkt, ongemoeid is gebleven, of waarom het niet lukte. De pagina toont
daarna de **opnieuw ingelezen** situatie: wat je ziet, zit werkelijk in het
bestand.

Zit dezelfde hoes er al in, of valt er niets te verwijderen, dan wordt het
bestand niet aangeraakt: een herschrijving die niets verandert is een
ongevraagde wijziging.

#### Ook als `cover.jpg` in de albummap

Onder het uploadveld staat een vinkje **ook als `cover.jpg` in de albummap
zetten**. Navidrome en vrijwel elke andere speler pakken dat bestand op, ook
wanneer de embedded hoes ontbreekt of afwijkt. Dit is de enige plek waar Sleeve
een nieuw bestand in de bibliotheek aanmaakt, en dus ook de enige plek met een
vinkje: zonder dat vinkje komt er niets in de map bij.

Wat de map in gaat is altijd JPEG en heet altijd `cover.jpg`. Eén vaste naam
vraagt om één vast formaat, dus een geüploade PNG wordt hiervoor omgezet — het
embedded origineel blijft ongewijzigd PNG. Een JPEG gaat ongewijzigd de map in.

Staat er al een `cover.jpg`, dan zegt de pagina dat, met de omvang erbij, en
komt er een tweede vinkje om hem te **vervangen**. Zonder dat tweede vinkje
blijft het bestaande bestand staan en meldt het rapport waarom er niets gebeurd
is. Die bevestiging moet vóór het versturen gegeven worden: na een POST is de
bestandsinvoer van de browser leeg, dus een "weet je het zeker?"-scherm achteraf
zou betekenen dat je de afbeelding opnieuw moet kiezen.

Het schrijven loopt via `atomic::place`: naar een tijdelijk bestand in dezelfde
map, met eigenaar, groep en rechten van de track ernaast, en pas dan hernoemen.
Een afgebroken actie laat dus nooit een half bestand achter, en het resultaat
past bij de rest van de share. Het gebeurt ná het embedden en krijgt een eigen
regel in het rapport: gaat het schrijven van `cover.jpg` mis, dan blijft de hoes
die al in de tracks staat gewoon staan.

De feiten komen uit dezelfde leesronde als de tags: er wordt alleen de header van
de afbeelding gelezen, niet de pixels. Dat is ook wat de maplijst gebruikt om te
signaleren dat de tracks in een map **verschillende hoezen** hebben — vergeleken
op type, afmetingen en omvang, want twee hoezen die daarin gelijk zijn, zijn in
de praktijk dezelfde afbeelding. Bestanden zonder hoes tellen daarbij niet mee;
die hebben hun eigen melding.

### Wat er met een nieuwe hoes gebeurt

Een aangeleverde afbeelding gaat door `art::prepare`, en daar gebeuren drie
dingen — niet meer:

1. **Valideren.** Het formaat wordt uit de bytes zelf geraden, niet uit een
   bestandsnaam of een `Content-Type`: een `.jpg` die in werkelijkheid een zip
   is, hoort niet in iemands muziekbibliotheek te belanden. Alleen JPEG en PNG
   komen erdoor, en alleen tot `MAX_UPLOAD_MB` — die grens geldt op de ruwe
   bytes, dus vóór er iets wordt uitgepakt.
2. **Verkleinen**, maar alleen wat boven `MAX_ART_SIZE` uitkomt. Een 3000×3000
   scan in elk van de twaalf tracks blaast een album op. De beeldverhouding
   blijft behouden en er wordt nooit vergroot.
3. **Hercoderen**, en alleen wanneer er verkleind is — naar JPEG met kwaliteit
   `ART_QUALITY`, tenzij het origineel werkelijk doorzichtige pixels bevat: die
   zouden zwart worden.

Past de afbeelding al binnen de grenzen, dan komen de bytes **ongewijzigd** het
bestand in: geen hercodering, geen kwaliteitsverlies, en een PNG blijft een PNG.
Dat is het antwoord op de open vraag uit PRD §12 — er wordt alleen omgezet
wanneer er toch al iets moet gebeuren.

Een decoder krijgt daarbij een geheugengrens mee. Een afbeelding van 20000×20000
past in een paar honderd kilobyte gecomprimeerde data maar vraagt bij het
uitpakken meer dan een gigabyte; op een NAS met weinig geheugen is dat het
verschil tussen een foutmelding en een gestorven container.

### Geavanceerde weergave: alle ruwe tags

`/tags/<pad>`, bereikbaar vanaf de bewerkpagina, toont per bestand alles wat er
werkelijk in staat, inclusief velden die het genormaliseerde model niet kent:
ID3v2-frames voor MP3 (`TIT2`, `TPE1`, …) en Vorbis-comments voor FLAC
(`TITLE`, `ARTIST`, …), telkens met hun oorspronkelijke sleutelnaam. Embedded art wordt samengevat als type en
grootte; de afbeeldingsdata zelf komt er niet in.

Deze weergave is **alleen-lezen** en dat is geen tijdelijke beperking: ruwe
frames bewerken is geen doel van het MVP. De pagina bevat daarom geen formulier,
geen invoerveld en geen knop; een test bewaakt dat.

Eén ding om te weten bij het lezen: een samengesteld ID3v2-frame als `TRCK`
(`3/12`) staat als één frame in het bestand, maar wordt als nummer én totaal
gelezen. Beide delen verschijnen dan als twee regels met dezelfde sleutel. De
pagina zegt dat er zelf bij.

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
