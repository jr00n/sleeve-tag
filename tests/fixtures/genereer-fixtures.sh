#!/usr/bin/env bash
#
# Genereert de testfixtures voor Sleeve.
#
# De fixtures staan ingecheckt in Git; dit script hoeft alleen te draaien als er
# een variant bij moet of als een bestand opnieuw gemaakt moet worden. Draai het
# vanuit deze map:
#
#     ./genereer-fixtures.sh
#
# Alle audio is één seconde stilte, zodat de bestanden klein blijven. De inhoud
# doet er niet toe: de tests gaan over tags, niet over geluid.
#
# Let op: opnieuw genereren levert niet byte-voor-byte dezelfde bestanden op
# tussen ffmpeg-versies. Dat is geen probleem — de tests kijken naar tags, niet
# naar hashes — maar het verklaart wel waarom een hergeneratie een diff geeft.

set -euo pipefail

cd "$(dirname "$0")"

if ! command -v ffmpeg >/dev/null; then
  echo "ffmpeg is nodig om de fixtures te genereren (brew install ffmpeg)" >&2
  exit 1
fi

# De duur staat in de bron zelf (`d=1`) en niet als losse `-t`-optie: `-t` na een
# `-i` geldt voor de volgende input, en bij de varianten met een tweede input
# (de cover) liep `anullsrc` daardoor oneindig door.
STILTE=(-f lavfi -i "anullsrc=r=44100:cl=mono:d=1")

# -bitexact laat encoder- en versietags weg, wat de uitvoer zo reproduceerbaar
# mogelijk maakt. Staat per output, niet in STILTE, om dezelfde reden.
#
# Geen -shortest bij de varianten met een cover: de kortste stream is dan het
# ene coverframe, en de audio zou tot 0,04 seconde worden afgekapt.
FF=(ffmpeg -hide_banner -loglevel error -y)

# Verwijdert een ID3v2-tag aan het begin van een MP3.
#
# Nodig omdat ffmpeg `-write_id3v2 0` negeert zodra er metadata is: zonder deze
# stap zou "id3v1-only" gewoon óók een volledige ID3v2-tag hebben, en zou
# "untagged" met een lege ID3v2-header beginnen.
strip_id3v2() {
  python3 - "$1" <<'PY'
import sys
from pathlib import Path

pad = Path(sys.argv[1])
data = pad.read_bytes()

if data[:3] == b"ID3":
    # De omvang staat in vier synchsafe bytes: zeven bits per byte.
    omvang = (data[6] << 21) | (data[7] << 14) | (data[8] << 7) | data[9]
    pad.write_bytes(data[10 + omvang :])
PY
}

echo "cover-afbeeldingen"
"${FF[@]}" -f lavfi -i "color=c=0x3a6ea5:s=300x300" -frames:v 1 -update 1 -bitexact cover.png
"${FF[@]}" -i cover.png -q:v 4 -bitexact cover.jpg

# Volledige tagset: de velden uit het tagmodel van het PRD.
TAGS=(
  -metadata title="Stilte in D"
  -metadata artist="De Testartiest"
  -metadata album_artist="De Albumartiest"
  -metadata album="Fixtures voor Sleeve"
  -metadata track="3/12"
  -metadata disc="1/2"
  -metadata date="2024"
  -metadata genre="Ambient"
  -metadata composer="De Componist"
  -metadata comment="Gegenereerd voor de tests van Sleeve"
)

echo "MP3 zonder tags"
"${FF[@]}" "${STILTE[@]}" -c:a libmp3lame -q:a 9 -bitexact \
  -write_id3v1 0 -write_id3v2 0 -map_metadata -1 untagged.mp3
strip_id3v2 untagged.mp3

echo "MP3 met volledige tags (ID3v2.4)"
"${FF[@]}" "${STILTE[@]}" -c:a libmp3lame -q:a 9 -bitexact \
  -id3v2_version 4 -write_id3v1 0 "${TAGS[@]}" tagged.mp3

echo "MP3 met embedded album art"
"${FF[@]}" "${STILTE[@]}" -i cover.jpg -c:a libmp3lame -q:a 9 -bitexact \
  -map 0:a -map 1:v -c:v copy -disposition:v attached_pic \
  -metadata:s:v title="Album cover" -metadata:s:v comment="Cover (front)" \
  -id3v2_version 4 -write_id3v1 0 "${TAGS[@]}" tagged-with-art.mp3

echo "MP3 met uitsluitend een ID3v1-tag"
"${FF[@]}" "${STILTE[@]}" -c:a libmp3lame -q:a 9 -bitexact \
  -write_id3v1 1 "${TAGS[@]}" id3v1-only.mp3
strip_id3v2 id3v1-only.mp3

echo "MP3 met een ID3v1-tag die afwijkt van de ID3v2-tag"
# ffmpeg schrijft beide tags altijd met dezelfde waarden. Voor de fixture die de
# opruimregel uit het PRD moet uitdagen ("nooit inconsistent achterlaten") wordt
# de 128 bytes lange ID3v1-tag daarom met de hand aangeplakt, met andere waarden
# dan de ID3v2-tag.
cp tagged.mp3 id3v1-inconsistent.mp3
python3 - <<'PY'
from pathlib import Path

def veld(tekst: str, lengte: int) -> bytes:
    """ID3v1 gebruikt vaste veldlengtes, aangevuld met nulbytes."""
    ruw = tekst.encode("latin-1", errors="replace")[:lengte]
    return ruw + b"\x00" * (lengte - len(ruw))

tag = (
    b"TAG"
    + veld("Oude titel uit ID3v1", 30)
    + veld("Oude artiest", 30)
    + veld("Oud album", 30)
    + veld("1999", 4)
    + veld("afwijkend van ID3v2", 30)
    + bytes([13])  # genre 13 = Pop
)
assert len(tag) == 128, len(tag)

pad = Path("id3v1-inconsistent.mp3")
pad.write_bytes(pad.read_bytes() + tag)
PY

echo "FLAC zonder tags"
"${FF[@]}" "${STILTE[@]}" -c:a flac -bitexact -map_metadata -1 untagged.flac

echo "FLAC met volledige tags"
"${FF[@]}" "${STILTE[@]}" -c:a flac -bitexact "${TAGS[@]}" tagged.flac

echo "FLAC met embedded album art"
"${FF[@]}" "${STILTE[@]}" -i cover.jpg -c:a flac -bitexact \
  -map 0:a -map 1:v -c:v copy -disposition:v attached_pic \
  -metadata:s:v title="Album cover" -metadata:s:v comment="Cover (front)" \
  "${TAGS[@]}" tagged-with-art.flac

echo
echo "Klaar. Totale omvang:"
du -ch ./*.mp3 ./*.flac ./*.png ./*.jpg | tail -1
