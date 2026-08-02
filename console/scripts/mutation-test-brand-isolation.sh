#!/usr/bin/env bash
#
# mutation-test-brand-isolation.sh — proves check-brand-isolation.sh can fail.
#
# A gate that has only ever been read is not evidence. This breaks the guarded
# property one way at a time, asserts the gate goes RED, restores the tree, and
# asserts it goes GREEN again. Every arm covers a DIFFERENT path into the
# shipped output, because a gate can be alive on one and blind on the others:
#
#   A  a brand name in a shared component        → the JS bundle
#   B  a brand name in the shared stylesheet     → the CSS bundle
#   C  a brand name in the default brand's data  → index.html + the JS bundle
#   D  the PRE-RENAME name in a shared component → the second needle
#   E  the default favicon equal to the Vulos one→ the favicon comparison
#   F  an empty dist/                            → the coverage assertion
#
# Arms A–E mutate tracked files. The trap restores them on any exit, including
# a failure or a Ctrl-C, so an interrupted run cannot leave the tree dirty.

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONSOLE="$(dirname "$HERE")"
cd "$CONSOLE"

GATE="scripts/check-brand-isolation.sh"
TOUCHED=(src/lib/components/Shell.svelte src/app.css src/brands/pier.meta.ts src/brands/vulos.meta.ts)
BACKUP="$(mktemp -d)"

restore() {
  for f in "${TOUCHED[@]}"; do
    cp "$BACKUP/$(basename "$f")" "$f"
  done
}
trap 'restore; rm -rf "$BACKUP"' EXIT INT TERM

for f in "${TOUCHED[@]}"; do cp "$f" "$BACKUP/$(basename "$f")"; done

pass=0
fail=0

# Run the gate; expect it to fail. $1 = arm label.
expect_red() {
  echo
  echo "════════ $1 ════════"
  if bash "$GATE" >/tmp/brand-gate.out 2>&1; then
    printf '\033[31mMUTATION SURVIVED\033[0m — the gate PASSED with the defect present.\n'
    tail -6 /tmp/brand-gate.out | sed 's/^/  | /'
    fail=$((fail + 1))
  else
    printf '\033[32mgate went RED as required\033[0m\n'
    grep -E 'FAIL|hit ' /tmp/brand-gate.out | head -6 | sed 's/^/  | /'
    pass=$((pass + 1))
  fi
}

expect_green() {
  echo
  echo "════════ $1 ════════"
  if bash "$GATE" >/tmp/brand-gate.out 2>&1; then
    printf '\033[32mgate is GREEN on the restored tree\033[0m\n'
    tail -4 /tmp/brand-gate.out | sed 's/^/  | /'
    pass=$((pass + 1))
  else
    printf '\033[31mgate is RED on a clean tree\033[0m — the revert did not take.\n'
    tail -12 /tmp/brand-gate.out | sed 's/^/  | /'
    fail=$((fail + 1))
  fi
}

expect_green "BASELINE — unmutated tree must be GREEN"

# ── A: brand name in a SHARED component ─────────────────────────────────────
perl -0pi -e 's/Coordinator control plane/Coordinator control plane, a Vulos service/' \
  src/lib/components/Shell.svelte
expect_red 'A — "Vulos" in a shared component (Shell.svelte topbar text)'
restore

# ── B: brand name in the SHARED stylesheet ──────────────────────────────────
# A custom property, not a comment: minification strips CSS comments, so a
# comment-only mutation would prove nothing about what actually ships.
perl -0pi -e 's/  --font-mono:/  --vulos-brand-leak: 1;\n  --font-mono:/' src/app.css
expect_red 'B — "vulos" in the shared stylesheet (app.css custom property)'
restore

# ── C: brand name in the DEFAULT brand's own data ───────────────────────────
perl -0pi -e "s{docsUrl: 'https://github.com/vul-os/kotva/blob/main/coordinator/CONTRACT.md'}{docsUrl: 'https://vulos.org/docs/'}" \
  src/brands/pier.meta.ts
expect_red 'C — a vulos.org link in the default brand file'
restore

# ── D: the PRE-RENAME name (second needle) ──────────────────────────────────
perl -0pi -e 's/Coordinator control plane/Ephor control plane/' src/lib/components/Shell.svelte
expect_red 'D — "Ephor" (the pre-rename name) in a shared component'
restore

# ── E: the default favicon made identical to the Vulos one ──────────────────
# Both brands pointed at the same mark AND the same accent, so the emitted
# favicons are byte-identical while no brand NAME leaks — this isolates the
# favicon comparison from the string scan.
perl -0pi -e "s{markPath: 'src/brands/assets/vulos-mark.svg'}{markPath: '../brand/pier.svg'}" src/brands/vulos.meta.ts
perl -0pi -e "s/accent: '#0f6a6c'/accent: '#c89a56'/" src/brands/vulos.meta.ts
expect_red 'E — default favicon byte-identical to the Vulos build''s'
restore

# ── F: the coverage assertion ───────────────────────────────────────────────
# The one arm that cannot be triggered by mutating the app: it guards against
# the SCAN breaking rather than the OUTPUT breaking. Exercised on a copy of the
# gate whose `find` is pointed at an empty directory — the build and every other
# assertion still run normally, so the ONLY thing that changes is that the scan
# reads nothing. If a zero-file scan reported PASS, every green above would be
# worthless.
echo
echo "════════ F — a scan that reads zero files must not read as a pass ════════"
# The copy MUST live beside the original: the gate derives the console directory
# from its own $BASH_SOURCE, so a copy in /tmp would cd to the wrong place and
# die before it ever reached the assertion under test. An earlier revision of
# this arm did exactly that and reported a green that meant nothing — which is
# why the branch below insists on the specific failure message.
EMPTY_GATE="$HERE/.mutation-empty-scan.sh"
mkdir -p .brand-check/empty
sed 's|find "$DIST_DEFAULT" -type f -print0|find ".brand-check/empty" -type f -print0|' \
  "$GATE" > "$EMPTY_GATE"
# Guard against the sed silently matching nothing, which would make this arm
# test the unmutated gate and always "pass".
if cmp -s "$GATE" "$EMPTY_GATE"; then
  printf '\033[31mARM BROKEN\033[0m — the mutation sed matched nothing; this arm proves nothing.\n'
  fail=$((fail + 1))
elif bash "$EMPTY_GATE" >/tmp/brand-gate.out 2>&1; then
  printf '\033[31mMUTATION SURVIVED\033[0m — a scan of zero files reported PASS.\n'
  fail=$((fail + 1))
else
  # Must be RED for the COVERAGE reason, not because the build fell over.
  if grep -q 'scanned 0 files' /tmp/brand-gate.out; then
    printf '\033[32mgate went RED as required\033[0m\n'
    grep -E 'FAIL' /tmp/brand-gate.out | head -3 | sed 's/^/  | /'
    pass=$((pass + 1))
  else
    printf '\033[31mARM BROKEN\033[0m — gate failed, but not on the coverage assertion:\n'
    tail -8 /tmp/brand-gate.out | sed 's/^/  | /'
    fail=$((fail + 1))
  fi
fi
rm -rf .brand-check "$EMPTY_GATE"

expect_green 'REVERTED — tree restored, gate must be GREEN again'

echo
if [ "$fail" -eq 0 ]; then
  printf '\033[32mPASS\033[0m %d/%d mutation arms behaved correctly.\n' "$pass" "$((pass + fail))"
else
  printf '\033[31mFAIL\033[0m %d of %d mutation arms did NOT behave correctly.\n' "$fail" "$((pass + fail))"
  exit 1
fi
