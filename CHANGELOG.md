# Changelog

All notable changes to **Pier** — the broker (coordinator) reference implementation of the KOTVA
standard. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning is
[SemVer](https://semver.org/spec/v2.0.0.html).

> **This file was reset on 2026-08-03, and the reset is the first entry rather than a silent gap.**
> The project was renamed Ephor → Pier and has **no releases, no crates.io or npm publications, no
> forks and no dependents** — so nothing downstream was relying on the old entries, which is the
> only thing that makes a reset honest rather than a rewrite. Nothing was deleted: the pre-rename
> history of the JS SDK and the Go relay (0.1.0–0.3.0) moved to [`client/CHANGELOG.md`](client/CHANGELOG.md),
> the component it actually documents, and full history remains in git.

## [Unreleased]

### Changed — the project is now Pier

- **Renamed Ephor → Pier** across the repo, the 15 crates (`pier-*`), the Go module
  (`github.com/vul-os/pier`), the console, the site and the brand. An *ephor* was a Spartan
  magistrate who oversaw the kings — authority, which is precisely what a KOTVA coordinator is
  not: accountable, swappable, never load-bearing. A pier serves vessels it does not own. It
  completes the family: kotva = anchor → pier → depot.
- **New mark**, drawn to the Vulos product-logo standard.
- **`DS-tag` change, WIRE-BREAKING and deliberate:** `EPHOR-REACH-v0/tunnel-auth` →
  `PIER-REACH-v0/tunnel-auth`. A signature made under the old prefix does not verify under the
  new one; both ends must be rebuilt. That is what a domain-separation tag is for.

### Added

- **`crates/pier-cli`** — a flyctl-shaped CLI over the DEPOT control plane. Every verb IS a
  `kotva_depot::Ability`, enforced by a test that walks the built command table in both directions
  with coverage counts, so `terminate` is unrepresentable. **35 of its 37 rows are scaffolds that send
  nothing** — tagged in `--help`, exiting non-zero and naming what is missing. The other 2 are
  local-only (`auth token`, `auth whoami`) and contact no coordinator.
- **`crates/pier-infra-service`** — the `infra-service` coordinator kind, absorbing the retired
  `compute` kind (hosted inference is an `edge-fn` with `artifact-source = operator`, not a kind).
- **Foreign-byte conformance**: `pier-economics` now decodes kotva-coordinator's own frozen
  `DESCRIPTOR_V0` and re-encodes it byte-for-byte, and rejects the retired `"compute"` kind. A
  round-trip test cannot catch two implementations drifting together; only bytes from the other
  side can.
- **A rebrandable console.** `console/src/brands/` with Pier as the default; `check-brand-isolation.sh`
  scans **built output** (not source) so an inlined SVG or a minified string cannot leak another
  brand, with an 8-arm mutation suite.

### Fixed

- **The CLI panicked on every command** while 39 tests were green — `ArgMatches::get_one` on an id
  the subcommand never declared. Every test built its input by hand and jumped over the only
  broken code.
- **Stale branding shipped in raster assets**: the console screenshot on the landing page still
  showed *Ephor*, invisible to every text grep of the rename.
- **The docs site published another repo's screenshots** — a demo harness branded
  `github.com/vul-os/vulos-relay`, at a version this package has never had.

### Security

- **`publish = false` on the whole workspace, enforced.** A spec-defined wire object needs exactly
  one published home; the `kotva-*` crates are it. npm packages must now also declare a posture
  explicitly rather than becoming publishable by omission.

