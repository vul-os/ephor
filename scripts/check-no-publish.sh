#!/usr/bin/env bash
# Enforce the PUBLISH RULE (workspace Cargo.toml): this repo publishes NOTHING to crates.io.
#
# The published, semver-stable artefacts are the `kotva-*` crates in the kotva repo, which this repo
# CONSUMES. Spec-defined wire objects must have exactly one published home — otherwise two
# implementations can encode the same object differently and both believe they are conformant — and
# a crates.io name can be yanked but never released, so publishing from a deliberately replaceable
# implementation permanently reserves names it should not hold.
#
# Checks, in order of what has actually gone wrong in this suite:
#   1. Every crate manifest disables publishing (directly or by workspace inheritance).
#   2. The workspace itself declares `publish = false`, so a NEW crate inherits the rule by default
#      rather than silently defaulting to publishable.
#   3. No `cargo publish` invocation exists in CI, scripts, or the Makefile.
#   4. A coverage floor: the crate count is asserted, so a glob that silently matches nothing (or a
#      crate added outside crates/) fails loudly instead of reporting a vacuous pass.
set -euo pipefail
cd "$(dirname "$0")/.."

fail=0
note() { printf '  %s\n' "$*"; }
bad()  { printf 'FAIL: %s\n' "$*"; fail=1; }

# --- 4. coverage floor first, so an empty glob can never look like success -------------------
#
# Discovery is a DEPTH-INDEPENDENT find, not `crates/*/Cargo.toml`. That glob is pinned to exactly
# one level below crates/ and therefore could not match `crates/pier-gateway/fuzz/Cargo.toml` — a
# real, separate crate with its own manifest — no matter how many crates existed. The floor could
# not catch it either: MIN_CRATES equalled the number of top-level crate dirs exactly, so the gate
# was honest about the 15 it saw and silent about the 16th. A nested crate (fuzz targets, xtask,
# examples, integration harnesses) is precisely where a stray `publish = true` would go unnoticed,
# and the gate's stated purpose is that a NEW crate inherits the rule by DEFAULT.
#
# `-prune` on target/ keeps vendored/build manifests out; they are not ours to govern.
shopt -s nullglob
mapfile -t manifests < <(find crates -name target -prune -o -name Cargo.toml -print | sort)
count=${#manifests[@]}
# 15 workspace/excluded crate dirs + crates/pier-gateway/fuzz. Raise this when a crate is added.
MIN_CRATES=16
if (( count < MIN_CRATES )); then
  bad "found $count crate manifests, expected at least $MIN_CRATES — the search matched nothing, or a crate moved"
  echo "(refusing to report a pass over an empty or truncated set)"
  exit 1
fi
note "checking $count crate manifests"

# --- 1. every member disables publishing ----------------------------------------------------
for f in "${manifests[@]}"; do
  if ! grep -qE '^publish[[:space:]]*=[[:space:]]*false|^publish\.workspace[[:space:]]*=[[:space:]]*true' "$f"; then
    bad "$f does not disable publishing (needs 'publish = false' or 'publish.workspace = true')"
  fi
  # `publish = true` anywhere is the design change the rule forbids, even if another line disables it.
  if grep -qE '^publish[[:space:]]*=[[:space:]]*true' "$f"; then
    bad "$f sets 'publish = true' — publishing from this repo is a design change, not a release step"
  fi
done

# --- 2. the workspace default itself --------------------------------------------------------
if ! grep -qE '^publish[[:space:]]*=[[:space:]]*false' Cargo.toml; then
  bad "workspace Cargo.toml lacks 'publish = false' in [workspace.package] — a new crate would default to publishable"
fi

# --- 3. no publish automation ---------------------------------------------------------------
# Capture into a variable and test emptiness, rather than branching on a pipeline's exit status.
# The earlier version of this check branched on `grep ... | grep -v ...` directly and was DEAD: it
# passed a `Makefile` path that does not exist in this repo, BSD grep returns 2 on a missing-file
# error even when it found matches elsewhere, and `set -o pipefail` propagated that 2 — so the `if`
# was false and a real `cargo publish` in CI sailed through while the offending line was printed to
# the console. Only mutation testing surfaced it. Scan just the paths that exist.
scan_paths=()
for d in .github scripts; do [[ -e "$d" ]] && scan_paths+=("$d"); done
for g in Makefile Justfile; do [[ -f "$g" ]] && scan_paths+=("$g"); done
if (( ${#scan_paths[@]} == 0 )); then
  bad "nothing to scan for publish automation — expected at least scripts/ to exist"
else
  hits=$(grep -rn --include='*.yml' --include='*.yaml' --include='*.sh' --include='Makefile' \
           --include='Justfile' -E 'cargo[[:space:]]+publish' "${scan_paths[@]}" 2>/dev/null \
         | grep -v 'check-no-publish' || true)
  if [[ -n "$hits" ]]; then
    bad "a 'cargo publish' invocation exists — remove it:"
    printf '%s\n' "$hits" | sed 's/^/    /'
  fi
fi

if (( fail )); then
  echo
  echo "PUBLISH RULE violated. If a crate here genuinely needs external consumers, the fix is to"
  echo "move it to the kotva repo as a kotva-* crate, not to publish it from this one."
  exit 1
fi
echo "OK: $count crates, none publishable; workspace default is publish = false; no publish automation."
