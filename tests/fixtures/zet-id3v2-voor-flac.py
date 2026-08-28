"""Zet een ID3v2.4-tag vóór een FLAC-bestand.

Aangeroepen vanuit genereer-fixtures.sh. Dit hoort niet in een FLAC — de
standaard kent alleen Vorbis-comments — maar oudere rippers doen het toch, en op
de echte bibliotheek staan hele albums die er zo uitzien. Zonder zo'n bestand is
niet te testen dat Sleeve dat blok opruimt in plaats van er twee tags naast
elkaar op na te houden.

ffmpeg kan dit niet maken: de flac-muxer kent geen `-write_id3v2`. Vandaar dat
het blok hier met de hand voor het bestand wordt geplakt, net zoals
voeg-comm-frame-toe.py dat voor een MP3 doet.

Gebruik: python3 zet-id3v2-voor-flac.py <bestand.flac> <titel> <artiest>
"""

import sys
from pathlib import Path


def synchsafe(waarde: int) -> bytes:
    """Codeert een lengte als vier bytes van zeven bits, zoals ID3v2.4 vereist."""
    return bytes(
        (
            (waarde >> 21) & 0x7F,
            (waarde >> 14) & 0x7F,
            (waarde >> 7) & 0x7F,
            waarde & 0x7F,
        )
    )


def tekstframe(naam: str, tekst: str) -> bytes:
    """Bouwt één tekstframe: encoding 3 (UTF-8), daarna de tekst zelf."""
    inhoud = b"\x03" + tekst.encode("utf-8")
    return naam.encode("ascii") + synchsafe(len(inhoud)) + b"\x00\x00" + inhoud


def main() -> None:
    pad = Path(sys.argv[1])
    titel = sys.argv[2]
    artiest = sys.argv[3]

    data = pad.read_bytes()
    if data[:4] != b"fLaC":
        raise SystemExit(f"{pad}: dit is geen FLAC-bestand")

    body = tekstframe("TIT2", titel) + tekstframe("TPE1", artiest)

    # ID3v2.4.0, geen vlaggen, dan de omvang van de body.
    kop = b"ID3" + b"\x04\x00" + b"\x00" + synchsafe(len(body))

    pad.write_bytes(kop + body + data)


if __name__ == "__main__":
    main()
