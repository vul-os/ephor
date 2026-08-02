# Pier Rust workspace

Pier is the **broker (coordinator) reference implementation** of the KOTVA standard — the
single project that implements [`coordinator/CONTRACT.md`](https://github.com/vul-os/kotva).
This workspace is the all-Rust rewrite; the Go reverse-tunnel relay + JS client SDK are
**preserved untouched** alongside it until the port is proven (HANDOVER §Guardrails-3).

Why "Pier": a pier is infrastructure that serves vessels it does not own. Many independent
piers exist; you use whichever one you reach, and you leave. That is exactly the standing of a
KOTVA coordinator — accountable, swappable, never load-bearing — and it completes the family:
kotva = anchor -> pier -> depot. (The previous name meant a Spartan magistrate who oversaw the
kings: authority, which is the one thing a coordinator must not be.) Pier is not a Vulos
product; anyone may run one, and a third party's pier is theirs, not Vulos-operated.

## Layout

Every coordinator-kind crate declares exactly one content-visibility pair — `VisibilityClass`
(`blind` / `blind-routing` / `terminating`) × `AssuranceLevel` (`structural` / `attested` /
`declared`), CONTRACT §3 — checked by `pier-conformance`'s COORD-1..8 harness, not merely
asserted in prose.

| Crate | Kind / role | Status |
|---|---|---|
| `pier-economics` | Shared model: the content-visibility property, the coordinator-kinds table (CONTRACT §5), and the real `kotva-core`-signed descriptor / tariff / usage-receipt shapes (§2.1, §6). | **built** — real Ed25519 signing over deterministic CBOR, pinned to `kotva-core@core-v0.2.0` |
| `pier-conformance` | The `Coordinator` trait + the COORD-1..8 checklist harness (CONTRACT §7). | **built** — harness + tests |
| `pier-billing` | CONTRACT §6 made concrete: kind-agnostic metering, a signed `TariffSchedule`, signed usage receipts (`ReceiptLog`), and a no-token `SettlementRail`/`StakeVerifier` seam with mock reference adapters. | **built** — 55 tests |
| `pier-admin` | Kind-agnostic operator HTTP API (axum) composing pier-economics/pier-billing/pier-conformance: descriptor+tariff sign, metering/receipts view, quota/key mgmt, live `/conformance`. | **built** — 29 tests, `pier-admin` reference binary |
| `pier-gateway` | The mail *adapter* — legacy SMTP/IMAP/POP3 bridge (spec §7). The one `terminating` kind. Folded out of envoir; pins `kotva-core@core-v0.2.0`. | **built** — 320 tests incl. conformance + runtime COORD-1/5 discharge |
| `pier-relay` | Mesh `relay` kind, `blind`/`structural` — real libp2p 0.56 Circuit Relay v2 server; forwards NAT'd-peer ciphertext, holds no decrypting key. | **built** — 12 tests, real two-peer loopback relay test |
| `pier-reachability-adapter` | REACH kind, `blind-routing` — SNI-passthrough public reach for box services; box terminates TLS. Replaces the Go L7-terminating proxy. REACH-2 key-auth done. | **built** — 32 tests. **Open:** control-channel transport not yet Noise-encrypted (see CHANGELOG "Honest limits") |
| `pier-media-relay` | `blind-routing`/`structural` — orchestrates an external SFU (coturn/LiveKit sidecar) for real-time media; SFrame-sealed payload, routing metadata visible (RFC 9605). | **built** — 24 tests |
| `pier-indexer` | Search/discovery over a public plaintext corpus; query channel `terminating`/`declared` (attested-TEE option documented). `Gate::DerivedViewOnly` (§4 carve-out). | **scaffold** — posture + signed descriptor only, 8 tests |
| `pier-labeler` | Opt-in, subscribable moderation labels — CONTRACT §4's own named carve-out example. `Gate::DerivedViewOnly`. | **scaffold** — 7 tests |
| `pier-matcher` | Real-time supply/demand matching (rides, delivery); `terminating`/`declared` default, attested-TEE option. `Gate::DerivedViewOnly`. | **scaffold** — 8 tests |
| `pier-arbiter` | Dispute resolution (staked jury) over disclosed evidence; `terminating`/`declared`. `Gate::NoDeliveryPath`; stake is a `pier-billing::StakeVerifier` seam, no field in the descriptor. | **scaffold** — 7 tests |
| `pier-oracle` | Physical-world/real-fact attestation (ORACLE ⊂ ATTEST, DIRECTION §2); `terminating`/`declared`. `Gate::NoDeliveryPath`. | **scaffold** — 7 tests |
| `pier-compute` | Outsourced computation, *provisional* per CONTRACT §5's own table; `terminating`/`declared` default, attested-TEE "blind compute" option. `Gate::NoDeliveryPath`. | **scaffold** — 8 tests |

"scaffold" means: a real `pier_conformance::Coordinator` implementation and a real
`kotva-core`-signed descriptor exist and are tested, but the kind's own function (ranking,
labeling, matching, arbitration, attestation, compute) is future work, disclosed in each crate's
`//!` docs — not silently stubbed.

### The `kotva-core` carve (done)

`kotva-core` + `kotva-mail` are carved out of envoir and live in the kotva repo (`crates/`),
tag-pinned at **`core-v0.2.0`**. Every crate above that signs or verifies (`pier-economics` and
everything built on it) uses the real crate — there is no stub seam left. The wire is
byte-identical to envoir's own conformance vectors. envoir itself is now node-only: the mail
gateway and its conformance/fuzz moved here to `crates/pier-gateway`.

## The `kotva-core` pin (the isango guardrail)

Substrate types (MOTE, envelope, identity/naming, PUB, SYNC, signing/DS-tags, CBOR, crypto) come
from **`kotva-core`**, pinned by **tag** in the workspace `Cargo.toml` — never tracked at HEAD or
by path (HANDOVER §Guardrails-1). `pier-economics` re-exports the pieces every other crate in
this workspace needs (`IdentityKey`, `Cbor`, the signed descriptor/tariff/usage-receipt shapes) so
individual kind crates don't each carry their own `kotva-core` dependency.

## No token

Settlement and stake ride existing assets only, verified on-rail (DIRECTION §5) — there is no
protocol token anywhere in this workspace, and `Descriptor` structurally cannot carry a stake
field or a price rank (CONTRACT §2.1). `pier-billing::SettlementRail`/`StakeVerifier` are the
seams; both default fail-closed with a single mock reference adapter each.

## Build

```sh
cargo build
cargo test                    # 556 tests
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

These are what the `Rust Workspace` CI job runs. `pier-billing-patala` is **`exclude`d** from
this workspace (it path-depends on the sibling `patala` repo, absent from a fresh checkout), so
none of the above — `--workspace` included — reaches it; build it with
`cargo test --manifest-path crates/pier-billing-patala/Cargo.toml` when `patala` is checked out
next to `pier`. It is the one crate CI does not verify.

The Go tree (`go build ./...`) is unaffected — `Cargo.toml` and `go.mod` coexist at the root.
