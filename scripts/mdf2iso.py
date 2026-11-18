#!/usr/bin/env python3
"""Convert the data track of a raw Alcohol 120% .mdf disc image to ISO 9660.

Both Heavy Gear discs are stored with 2448-byte raw sectors: 12 sync + 4
header + 2048 user data + 288 EDC/ECC + 96 subchannel bytes. Disc 1 is a
single Mode 1 data track. Disc 2 is mixed-mode: one Mode 1 data track
followed by CD audio tracks, whose sectors carry no sync pattern. An ISO
image is the 2048 user-data bytes of each data sector, so the conversion is
a slice per sector that stops at the first sector without a sync pattern.

The script refuses to guess: a sector that has a sync pattern but is not
Mode 1, or a short read, aborts with an error rather than producing a
corrupt ISO. Trailing audio sectors are counted and reported, not written;
extracting the soundtrack is a separate concern.

Usage: mdf2iso.py <image.mdf> <out.iso>
"""
import sys

SYNC = b"\x00" + b"\xff" * 10 + b"\x00"
RAW_SECTOR = 2448  # 2352 bytes of CD frame + 96 bytes of subchannel data
USER_DATA_OFFSET = 16  # after the 12-byte sync and 4-byte header
USER_DATA_SIZE = 2048


def convert(src_path: str, dst_path: str) -> tuple[int, int]:
    """Return (data sectors written, trailing non-data sectors skipped)."""
    written = 0
    trailing = 0
    with open(src_path, "rb") as src, open(dst_path, "wb") as dst:
        while True:
            sector = src.read(RAW_SECTOR)
            if not sector:
                return written, trailing
            if len(sector) != RAW_SECTOR:
                raise SystemExit(f"short sector {written + trailing}: {len(sector)} bytes")
            if sector[:12] != SYNC:
                trailing += 1
                continue
            if trailing:
                raise SystemExit(f"data sector {written + trailing} after {trailing} audio sectors; "
                                 "multi-session or interleaved layouts are not handled")
            if sector[15] != 1:
                raise SystemExit(f"sector {written} is mode {sector[15]}, expected Mode 1")
            dst.write(sector[USER_DATA_OFFSET:USER_DATA_OFFSET + USER_DATA_SIZE])
            written += 1


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    sectors, trailing = convert(argv[1], argv[2])
    print(f"wrote {sectors} data sectors ({sectors * USER_DATA_SIZE} bytes) to {argv[2]}")
    if trailing:
        print(f"skipped {trailing} trailing audio sectors ({trailing / 75 / 60:.1f} minutes of CD audio)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
