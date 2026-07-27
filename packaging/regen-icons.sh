#!/usr/bin/env bash
# Regenerate hicolor PNG sizes from the master SVG.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
svg="$root/assets/icons/mor-image-prompt-atelier.svg"
name=mor-image-prompt-atelier

for size in 16 22 24 32 48 64 128 256 512; do
  dir="$root/assets/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$dir"
  rsvg-convert -w "$size" -h "$size" "$svg" -o "$dir/${name}.png"
  echo "wrote $dir/${name}.png"
done
