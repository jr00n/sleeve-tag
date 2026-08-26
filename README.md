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

## Projectstructuur

| Module | Verantwoordelijkheid |
|--------|----------------------|
| `config` | Configuratie uit omgevingsvariabelen |
| `fs` | Padvalidatie en containment binnen `MUSIC_ROOT` |
| `tags` | Genormaliseerd tagmodel en alle tag-I/O (de enige plek die `lofty` gebruikt) |
| `web` | Axum-router, handlers en askama-templates |
