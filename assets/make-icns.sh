#!/bin/sh
# Regenerates assets/AppIcon.icns from assets/icon.svg (macOS only: swiftc + iconutil).
set -eu
cd "$(dirname "$0")"
tmp=$(mktemp -d)
swiftc -o "$tmp/render" render_svg.swift
set="$tmp/AppIcon.iconset"
mkdir "$set"
for s in 16 32 128 256 512; do
  "$tmp/render" icon.svg "$set/icon_${s}x${s}.png" "$s"
  "$tmp/render" icon.svg "$set/icon_${s}x${s}@2x.png" "$((s * 2))"
done
iconutil -c icns "$set" -o AppIcon.icns
rm -rf "$tmp"
