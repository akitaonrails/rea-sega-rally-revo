#!/bin/sh
# look-sizes.sh — the size each look gives the game (REA_LOOK_SIZES), as
# SRRLauncher resolves it: Original 1280x720, Modern the screen, Steam Deck
# 1280x800; this run's --look/--resolution first, then launcher.cfg's.
# srr-play starts no gamescope (header), so this is the size the helper
# would hand it; it must agree with config::Look::preset.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export HOME=$tmp
REA_FUNCTIONS_ONLY=1 REA_PLAY_HERE=$HERE/.. . "$HERE/../srr-play"
REA_TEST=1
REA_SCREEN=2560x1440
REA_CFG=$tmp/launcher.cfg

[ -n "$REA_LOOK_SIZES" ] || { echo "FAIL: srr-play declares no REA_LOOK_SIZES"; exit 1; }

size() { rea_one_run "$@"; rea_game_size; printf '%sx%s\n' "$REA_W" "$REA_H"; }
expect() { # WANT DESCRIPTION ARGS...
  _want=$1 _what=$2; shift 2
  _got=$(size "$@")
  [ "$_got" = "$_want" ] || { echo "FAIL: $_what: $_got, want $_want"; exit 1; }
}

# What the window saves: the look and its size (Form::save always writes
# WIDTHxHEIGHT). launcher.cfg's resolution is what the game gets.
for case in "original 1280x720" "modern 2560x1440" "steam-deck 1280x800"; do
  set -- $case
  printf 'look = %s\nresolution = %s\n' "$1" "$2" > "$REA_CFG"
  expect "$2" "launcher.cfg look = $1"
done

# This run's --look sets its size over the file's (Settings::merged), any
# spelling of the look; this run's --resolution wins over both.
printf 'look = original\nresolution = 1280x720\n' > "$REA_CFG"
expect 1280x800 "--look steam-deck over the file" --no-window --look steam-deck
expect 1280x800 "--look Steam-Deck over the file" --look Steam-Deck
expect 2560x1440 "--look=modern over the file" --look=modern
expect 1280x720 "--look original" --look original
expect 1024x768 "--resolution over --look" --look steam-deck --resolution 1024x768
# A hand-written `look =` without a resolution is only a label to
# SRRLauncher (the screen's size); the helper would size it by the look.
# Moot while srr-play starts no gamescope (REA_GAMESCOPE=off).
echo "look-sizes.sh: ok"
