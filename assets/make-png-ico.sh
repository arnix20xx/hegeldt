#!/bin/sh
# Regenerates assets/icon-256.png (Linux) and assets/AppIcon.ico (Windows) from assets/icon.svg.
# macOS only (swiftc renders the SVG); python3 writes the ICO container around PNGs.
set -eu
cd "$(dirname "$0")"
tmp=$(mktemp -d)
swiftc -o "$tmp/render" render_svg.swift
for s in 16 24 32 48 64 128 256; do
  "$tmp/render" icon.svg "$tmp/$s.png" "$s"
done
cp "$tmp/256.png" icon-256.png
python3 - "$tmp" AppIcon.ico <<'PY'
import struct, sys
tmp, out = sys.argv[1], sys.argv[2]
sizes = [16, 24, 32, 48, 64, 128, 256]
pngs = [open(f"{tmp}/{s}.png", "rb").read() for s in sizes]
head = struct.pack("<HHH", 0, 1, len(sizes))
offset = 6 + 16 * len(sizes)
entries = b""
for s, p in zip(sizes, pngs):
    entries += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(p), offset)
    offset += len(p)
open(out, "wb").write(head + entries + b"".join(pngs))
PY
rm -rf "$tmp"
