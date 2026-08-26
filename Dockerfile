# Sleeve (`sleeve-tag`) — productie-image voor de UGREEN NAS.
#
# Bouwen vanaf een Apple Silicon Mac:
#
#     docker buildx build --platform linux/amd64 -t sleeve-tag:dev .
#
# De build-stage draait daarbij geëmuleerd als amd64. Dat is trager dan
# cross-compileren, maar het houdt de Dockerfile vrij van een cross-toolchain.

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
FROM rust:slim AS build

# musl-tools levert de linker voor een statisch gelinkte binary. Zonder statische
# linking is een distroless-runtime zonder libc geen optie.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Onder emulatie is elk rustc-proces fors zwaarder dan native. Met de
# standaardparallellie loopt een build-VM van 2 GB vol en wordt rustc door de
# OOM-killer afgeschoten (zichtbaar als "signal: 9, SIGKILL" bij een willekeurige
# crate). Twee jobs is de veilige ondergrens; verhoog dit als de builder meer
# geheugen heeft.
ARG BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${BUILD_JOBS}

# rust-toolchain.toml eerst, zodat rustup de vastgelegde toolchain ophaalt
# voordat er iets gecompileerd wordt.
COPY rust-toolchain.toml ./
RUN rustup target add x86_64-unknown-linux-musl

# Eerst alleen de manifesten plus een lege binary: deze laag compileert alle
# dependencies en blijft geldig zolang Cargo.toml en Cargo.lock niet wijzigen.
# Een wijziging in de broncode hergebruikt hem, wat een herbouw van tientallen
# crates scheelt.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --target x86_64-unknown-linux-musl \
    && rm -rf src

# Nu de echte broncode. `templates/` moet mee: askama verwerkt de templates
# tijdens het compileren, dus ze zitten daarna in de binary.
COPY src ./src
COPY templates ./templates

# Cargo kijkt naar wijzigingstijden; zonder touch zou de dummy-build als actueel
# gelden en zou de echte main.rs niet opnieuw vertaald worden.
RUN touch src/main.rs \
    && cargo build --release --target x86_64-unknown-linux-musl \
    && strip target/x86_64-unknown-linux-musl/release/sleeve-tag

# ---------------------------------------------------------------------------
# Runtime
# ---------------------------------------------------------------------------
# distroless/static bevat geen shell, geen package manager en geen libc: alleen
# wat een statisch gelinkte binary nodig heeft.
FROM gcr.io/distroless/static-debian12 AS runtime

# De statische assets worden op runtime van schijf geserveerd, relatief aan de
# werkdirectory.
WORKDIR /app

COPY --from=build /build/target/x86_64-unknown-linux-musl/release/sleeve-tag /usr/local/bin/sleeve-tag
COPY static ./static

# In de container is de muziekshare altijd /music; het pad op de NAS is
# uitsluitend de linkerkant van de volume-mount.
ENV MUSIC_ROOT=/music \
    PORT=8080

EXPOSE 8080

# Niet-root, met de UID en GID die op deze NAS gelden. De PUID/PGID-taak maakt
# dit instelbaar via omgevingsvariabelen.
USER 1000:10

ENTRYPOINT ["/usr/local/bin/sleeve-tag"]
