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
