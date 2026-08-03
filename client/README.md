# pier-client

> <img src="../docs/assets/vulos-logo.png" height="14" alt="VulOS"> Part of **[VulOS](https://vulos.org)** — the open, self-hostable web OS &amp; app suite. This is the client SDK for **Pier**, the suite's connectivity fabric. Runs standalone, or as an app hosted by the Vulos OS.

MIT-licensed JS client for the Pier peer-fabric relay. It runs in the browser
and is consumable by any web surface that speaks the peering contract.

This package runs in the browser and talks to the **host application's peering
backend** (e.g. the Vulos OS `/api/peering/*` endpoints) over HTTP / WebSocket.
It does not bundle a server.

## Part of VulOS

**Pier** is the connectivity fabric of the [VulOS](https://vulos.org)
suite — open, self-hostable products (OS, Diwan, Board, Files, Pier, llmux),
each usable alone and hosted as apps by the **Vulos OS** (the shell). This SDK is
consumed directly by the suite's web surfaces (the Vulos OS shell, Diwan);
the OS surfaces Pier-powered features but never imports product code. The package
has no Vulos-specific runtime dependency — it **runs standalone** against any
backend that implements the peering contract, **and** slots into the OS-hosted suite.

## Install

Published to npm:

```bash
npm install pier-client
```

Or, for local development against a checkout, as a `file:` dependency pointing
at this package's directory:

```jsonc
"pier-client": "file:../pier/client"
```

## Subpath exports

| Subpath                          | Module                                                  |
| -------------------------------- | ------------------------------------------------------- |
| `pier-client`            | root barrel — re-exports everything                     |
| `pier-client/endpoints`  | cloud↔LAN endpoint failover (`selectEndpoint`, etc.)    |
| `pier-client/offlineBootstrap` | one-call offline-first shell bootstrap            |
| `pier-client/signaling`  | `SignalingClient` over `/api/peering/stream` WebSocket  |
| `pier-client/fabric`     | `FabricClient` — WebRTC mesh + relay-circuit fallback (opt-in `rendezvousBaseUrl`) |
| `pier-client/rendezvous` | `RendezvousClient` — open announce/resolve/signal/mailbox + ICE against any relayd |
| `pier-client/presence`   | `PresenceManager` + `usePresence` React hook            |
| `pier-client/call`       | `createCall` — P2P mesh audio/video call                |
| `pier-client/useLiveCursors` | live-cursors React hook (`peerColor`)               |
| `pier-client/roundTripCheck` | round-trip fixture runner (`runRoundTripChecks`)    |

Both ESM (`.js`) and CJS (`.cjs`) bundles are emitted into `dist-lib/` by the
vite-lib build (`npm run build`). `react` and `xlsx` are declared as optional
peer dependencies so consumers dedupe them.

## Security model

Pier is the suite's connectivity fabric, and this client is a trust-boundary
participant. Two properties matter:

**Transport of the credential.** The client holds a short-lived Bearer JWT (the
box/app session token). It is attached to the signaling WebSocket (as a
`Sec-WebSocket-Protocol` token, never the URL) and to the ICE / relay HTTP
calls (`Authorization: Bearer …`). The client **refuses to attach the token to
a plaintext transport**: `wss://` / `https://` are required, and `ws://` /
`http://` are permitted only to a loopback host for local dev. A
`SignalingClient` / `FabricClient` constructed with a token over an insecure
remote URL throws at construction (`code: 'INSECURE_TOKEN_TRANSPORT'`) rather
than leaking the credential. The endpoint-selection layer applies the matching
rule to its credentialed health probe (an https allowlist — see
`endpoints.js`). A **tokenless** client may use `ws://` freely: signaling
frames are ECDSA-signed, so there is no credential to protect.

**Content-blindness of the two peer-fabric paths.** Application data never
flows to the relay server in the clear:

- **WebRTC P2P (preferred).** Data rides a browser `RTCDataChannel` (DTLS-SRTP)
  established directly between peers. The relay/signaling server sees only the
  offer/answer/ICE metadata, never the payload. The signed SDP pins the DTLS
  fingerprint, so a MITM signaling server cannot substitute its own transport.
- **Relay-circuit fallback (content-blind).** When P2P cannot be established,
  payloads are deposited via the relay HTTP API **sealed end-to-end**
  (XChaCha20-Poly1305 keyed by an X25519 ECDH / X3DH shared secret). The relay
  server stores and forwards ciphertext only. The forward-secret **v2 (X3DH)**
  path is preferred; a peer that has cryptographically committed to v2 support
  can never be silently downgraded to the non-forward-secret v1 path (a
  stripped signed-prekey fails closed). If no recipient encryption key is
  known, the deposit is **skipped rather than sent in the clear**.

## Migration compatibility — `configure()`

`endpoints.js` previously used a per-surface localStorage key
(`vulos.os.endpoints.v1`, `vulos.office.endpoints.v1`). The shared module
defaults to `vulos.relay-client.endpoints.v1` but exposes a `configure()`
seam so consumers can preserve their existing user state during migration:

```js
import { configure } from 'pier-client/endpoints'

// vulos OS:
configure({ lsKeyPrefix: 'vulos.os.endpoints.v1' })

// diwan:
configure({ lsKeyPrefix: 'vulos.office.endpoints.v1' })
```

## OS-specific extensions — `tierHint`

`offlineBootstrap.bootstrapOffline()` accepts an optional `tierHint` callback
so the OS-specific MEET-OS-01 Pro-tier injection (and any future per-surface
tier hint) can be wired in without OS-specific logic leaking into the shared
package. Consumers that don't supply one get `undefined` from
`currentTierHint()` — the shared package is a no-op for them.

## Consuming `kotva-client` — the substrate modules in `src/`

Eight modules in `src/` are **not this package's code**. They are the KOTVA
substrate protocols, and their source of truth is
[`kotva/bindings/js`](https://github.com/vul-os/kotva/tree/main/bindings/js),
published as `kotva-client`:

`chunkProof` · `relayBox` · `prekeys` · `signaling` · `rendezvous` ·
`rendezvousSignaling` · `secureTransport` · `errors`

They moved because `chunkProof.js` was a second implementation of a kotva spec
section — `substrate/FEEDS.md § 5.3`, whose Go half is Pier's own
`tunnel/pubcache` — sitting in a product repo with no shared owner. Products
consume the substrate; they do not re-implement it.

**Do not edit these files here.** `npm test` runs
`scripts/check-kotva-parity.mjs` first, which fails if any of them drifts by a
byte from the pinned tag in `kotva-client.pin.json`. Fix upstream in kotva, cut
a new `bindings/js` tag, then re-sync:

```
npm run kotva:pin -- ../../kotva     # re-copy + re-hash, then update tag/commit/version by hand
```

What stayed here is `FabricClient` (`fabric.js`) and the transport/UX glue
around it — `endpoints`, `health`, `presence`, `offlineBootstrap`, `regionPop`,
`roundTripCheck`, `useLiveCursors`, `call/*`. `FabricClient` orchestrates the
substrate but is not substrate: it hardcodes `/api/peering/*` paths, carries a
billing meter, and labels its data channel `diwan-fabric`.

### Why a hash pin and not a dependency

The end state is an ordinary pinned dependency,
`"kotva-client": "0.1.0"`, and the switch is one line in `package.json` plus
deleting the eight files and this gate. It is not that today for one verified
reason: **npm has no subdirectory support for git dependencies.** npm 11.6.2
clones the repo and reads `package.json` at the *clone root*, which kotva does
not have — so the Cargo convention this repo uses for the `kotva-*` crates
(`{ git = "…/kotva", tag = "core-v0.2.0" }`, which works because Cargo scans the
repo for the crate) has no npm equivalent. The tag exists —
`bindings/js/v0.1.0` — but a tag alone is not consumable by npm.

**What unblocks it:** publishing `kotva-client` from `kotva/bindings/js`, either
to the npm registry (`npm publish`, then pin `"kotva-client": "0.1.0"`) or as a
GitHub Release asset (`npm pack`, attach the `.tgz` to the `bindings/js/v0.1.0`
release, then pin the immutable release-asset URL). Both are release actions on
the kotva repo, not code changes here.

A `file:../../kotva/bindings/js` path dependency was deliberately not used. This
repo already records what that pattern costs, in `crates/pier-cli/Cargo.toml`:
the `kotva-depot` path dep makes `cargo build` at the root require a sibling
kotva checkout. The hash pin needs no checkout and no network, so the gate runs
identically in CI, offline, and on a fresh clone.

## License

MIT — see [LICENSE](./LICENSE).
