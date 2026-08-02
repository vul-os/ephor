#!/usr/bin/env bash
#
# check-brand-isolation.sh — the DEFAULT build must not leak another brand.
#
# Pier is the broker (coordinator) reference implementation of the KOTVA
# standard, not a Vulos product. The concrete danger this gate exists to stop is
# a third-party pier deployment being mistaken for a Vulos-operated service —
# which happens the moment one Vulos string survives into the shipped bundle: an
# inlined mark, a wordmark, a docs link, a localStorage key, a <title>.
#
# It therefore checks BUILT OUTPUT, never source. Source greps are the wrong
# instrument here: an SVG inlined through a virtual module, a string folded into
# a minified chunk, or a value stamped into index.html by a build plugin are all
# invisible to them and all shipped to users.
#
# Three assertions:
#   1. the default target really is pier (built with VITE_BRAND unset);
#   2. no file in its dist/ contains "vulos" or "ephor", case-insensitively —
#      every file, including fonts, read as bytes so nothing is skipped;
#   3. the emitted favicon differs from the one the Vulos brand emits.
#
# Plus a coverage assertion: a named set of files MUST have been scanned. A
# count alone is not enough — a glob that silently matched only the fonts would
# still clear a floor. See scripts/mutation-test-brand-isolation.sh for the
# evidence that this gate actually fails when it should.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONSOLE="$(dirname "$HERE")"
cd "$CONSOLE"

# Every brand name that must NOT appear in the default build's output. "ephor"
# is the name this project shipped under before the rename; a survivor is a
# stale artifact, not merely a cosmetic miss.
FORBIDDEN=(vulos ephor)

# Files that MUST be among those scanned. If any is missing, the build did not
# produce what we think it produces and a clean scan means nothing.
REQUIRED_ARTIFACTS=(
  index.html
  favicon.svg
)
# At least this many files must be scanned. The real build emits ~30 (two
# bundles, a favicon, an index.html and the vendored font faces); anything near
# zero means the glob broke, not that the output got clean.
MIN_FILES_SCANNED=20

fail() { printf '\033[31mFAIL\033[0m %s\n' "$*" >&2; exit 1; }
ok()   { printf '\033[32m  ok\033[0m %s\n' "$*"; }

DIST_DEFAULT="dist"
DIST_VULOS=".brand-check/dist-vulos"

# ── 1. Build the DEFAULT target ─────────────────────────────────────────────
# VITE_BRAND is deliberately UNSET, not set to "pier": this asserts what an
# operator who builds the repo as-is actually gets.
echo "── building the default brand (VITE_BRAND unset) ──"
rm -rf "$DIST_DEFAULT" "$DIST_VULOS"
env -u VITE_BRAND pnpm exec vite build --outDir "$DIST_DEFAULT" >/dev/null
[ -f "$DIST_DEFAULT/index.html" ] || fail "default build produced no $DIST_DEFAULT/index.html"

if ! grep -q '<title>Pier ' "$DIST_DEFAULT/index.html"; then
  fail "default build is not Pier — <title> in $DIST_DEFAULT/index.html says: $(grep -o '<title>[^<]*' "$DIST_DEFAULT/index.html")"
fi
ok "default target is Pier"

# ── 2. Scan every emitted file for a foreign brand ──────────────────────────
echo "── scanning built output ──"
scanned=0
hits=0
scanned_names=""

while IFS= read -r -d '' file; do
  scanned=$((scanned + 1))
  scanned_names="$scanned_names ${file#"$DIST_DEFAULT"/}"
  for needle in "${FORBIDDEN[@]}"; do
    # -a: read as text even for the vendored .woff/.woff2 faces, so binary
    #     assets are genuinely searched rather than silently skipped.
    # LC_ALL=C: byte-wise matching, no locale surprises on the font blobs.
    if LC_ALL=C grep -aiq -- "$needle" "$file"; then
      printf '\033[31m  hit\033[0m %s contains "%s":\n' "${file#"$DIST_DEFAULT"/}" "$needle" >&2
      LC_ALL=C grep -aio -- ".\{0,60\}$needle.\{0,60\}" "$file" | head -5 | sed 's/^/       /' >&2
      hits=$((hits + 1))
    fi
  done
done < <(find "$DIST_DEFAULT" -type f -print0)

# Coverage: a pass is only meaningful if the scan actually read the output.
[ "$scanned" -ne 0 ] || fail "scanned 0 files — the glob is broken, not the output clean"
if [ "$scanned" -lt "$MIN_FILES_SCANNED" ]; then
  fail "scanned only $scanned files, expected at least $MIN_FILES_SCANNED — the build is truncated or the glob is broken"
fi
for want in "${REQUIRED_ARTIFACTS[@]}"; do
  case " $scanned_names " in
    *" $want "*) ;;
    *) fail "coverage gap: $want was never scanned (scanned:$scanned_names)" ;;
  esac
done
# The bundles are content-hashed, so require them by shape rather than by name.
case "$scanned_names" in
  *assets/index-*.js*) ;;
  *) fail "coverage gap: no assets/index-*.js was scanned (scanned:$scanned_names)" ;;
esac
case "$scanned_names" in
  *assets/index-*.css*) ;;
  *) fail "coverage gap: no assets/index-*.css was scanned (scanned:$scanned_names)" ;;
esac
ok "scanned $scanned files, including index.html, favicon.svg and both bundles"

[ "$hits" -eq 0 ] || fail "$hits file/needle pair(s) in the default build name another brand"
ok "no occurrence of: ${FORBIDDEN[*]}"

# ── 3. The favicon must be this brand's, not the other one's ────────────────
echo "── comparing emitted favicons ──"
VITE_BRAND=vulos pnpm exec vite build --outDir "$DIST_VULOS" >/dev/null
[ -s "$DIST_DEFAULT/favicon.svg" ] || fail "default build emitted no (or an empty) favicon.svg"
[ -s "$DIST_VULOS/favicon.svg" ]   || fail "vulos build emitted no (or an empty) favicon.svg"

if cmp -s "$DIST_DEFAULT/favicon.svg" "$DIST_VULOS/favicon.svg"; then
  fail "the default build's favicon is byte-identical to the Vulos build's — the favicon is not brand-owned"
fi
ok "default favicon ($(wc -c < "$DIST_DEFAULT/favicon.svg" | tr -d ' ') bytes) differs from the Vulos one ($(wc -c < "$DIST_VULOS/favicon.svg" | tr -d ' ') bytes)"

rm -rf "$(dirname "$DIST_VULOS")"

printf '\033[32mPASS\033[0m brand isolation: the default build ships Pier and nothing else.\n'
