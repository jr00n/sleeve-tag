"""Voegt een COMM-frame toe aan een MP3 met een ID3v2.4-tag.

Aangeroepen vanuit genereer-fixtures.sh. Staat apart omdat het script anders
drie lagen quoting diep zou moeten nesten.

Gebruik: python3 voeg-comm-frame-toe.py <bestand.mp3> <commentaartekst>
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


def decodeer(vier: bytes) -> int:
    return (vier[0] << 21) | (vier[1] << 14) | (vier[2] << 7) | vier[3]


def main() -> None:
    pad = Path(sys.argv[1])
    tekst = sys.argv[2]

    data = pad.read_bytes()
    if data[:3] != b"ID3":
        raise SystemExit(f"{pad}: geen ID3v2-tag gevonden")
    if data[3] != 4:
        raise SystemExit(f"{pad}: verwacht ID3v2.4, kreeg 2.{data[3]}")

    omvang = decodeer(data[6:10])
    kop = data[:10]
    body = data[10 : 10 + omvang]
    rest = data[10 + omvang :]

    # COMM-payload: encoding 3 (UTF-8), taalcode, lege beschrijving met
    # nulterminator, dan de tekst zelf.
    inhoud = b"\x03" + b"dut" + b"\x00" + tekst.encode("utf-8")
    frame = b"COMM" + synchsafe(len(inhoud)) + b"\x00\x00" + inhoud

    # De volgorde van frames is vrij, dus vooraan invoegen mag.
    nieuwe_body = frame + body
    pad.write_bytes(kop[:6] + synchsafe(len(nieuwe_body)) + nieuwe_body + rest)


if __name__ == "__main__":
    main()
