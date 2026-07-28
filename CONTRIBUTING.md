# Contributing to Ephor

This repo ships **two deliverables in two languages**:

- **`client/`** — `@vulos/relay-client`, the browser JS/TS SDK (WebRTC
  peer-to-peer data channels, signaling, presence, offline queue).
- **`cmd/vulos-relayd`, `cmd/vulos-relay-agent`, `tunnel/`** — the self-hosted
  Go reverse-tunnel service (the sovereign `frp`/ngrok/Cloudflare Tunnel
  replacement: relay server, agent, rendezvous, autoscale, billing/cost
  metering).

Alongside them, the in-progress **Rust workspace** (`crates/`, 14 crates) is the
broker/coordinator reference implementation.

All three are built, tested, and CI-gated independently — see `.github/workflows/ci.yml`
(`client`, `go` and `rust` jobs). A PR that only touches one side only needs that
side's checks to pass, but keep all three green on `main`/`dev`.

One crate is deliberately outside that net and says so on every CI run:
`crates/broker-billing-patala`, the optional patala-backed settlement adapter, is
`exclude`d from the Rust workspace because it path-depends on the sibling `patala`
repo, which CI does not check out. If you touch it, run it yourself:
`cargo test --manifest-path crates/broker-billing-patala/Cargo.toml`.

## Code of Conduct

We follow the [Contributor Covenant v2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).

## Dev Environment Setup

### Client SDK (`client/`)

Requirements: Node.js 20+ and npm 10+.

```bash
cd client
npm ci
npm run build
npm test
```

The SDK targets browser environments (WebSocket, BroadcastChannel, WebRTC).
Unit tests run under Vitest with jsdom. There's also a real-browser boot-guard
suite (`npm run test:e2e`, Playwright/Chromium) that builds the library and
imports it through its actual `exports` map, catching build-output breakage
the jsdom unit tests can't see (Diwan imports the built `dist-lib/`, not the
source).

### Relay service (Go: `cmd/`, `tunnel/`)

Requirements: Go 1.23+ (repo currently builds with the 1.25 toolchain — see `go.mod`).

```bash
go build ./...
go vet ./...
go test ./... -race
```

Run from the repo root — the Go module (`github.com/vul-os/ephor`) covers
`cmd/vulos-relayd`, `cmd/vulos-relay-agent`, and everything under `tunnel/`
(server, agent, rendezvous, pubcache, autoscale, cost/billing, direct). Several
packages have `_e2e_test.go` suites (drain, shutdown, revocation, demand-signal,
billing) that spin up real in-process servers — expect these to take longer
than a typical unit test run.

## Branch and PR Conventions

- Branch off `main`. Name: `feat/description`, `fix/description`, `chore/description`.
- One logical change per PR.
- PRs require at least one approving review.
- Squash-merge to keep history linear.

## Commit Message Style

Conventional Commits welcome, not required:

```
feat(signaling): retry WebSocket connection with exponential backoff
fix(endpoints): fall back to same-origin when both cloud and LAN are down
fix(rendezvous): serve CORS on the announce/resolve endpoints
chore: upgrade vitest
```

## Testing Expectations

Before opening a PR, run the suite(s) for whatever you touched:

```bash
# client/ changes
cd client && npm test

# cmd/ or tunnel/ changes
go build ./... && go vet ./... && go test ./... -race
```

Changes to endpoint failover, signaling, or the offline queue (client) require
tests. Changes to the tunnel protocol, rendezvous, autoscale, or cost/billing
(Go) require tests. Security-relevant paths on either side (auth token
handling, session isolation, SSRF guards) require explicit test coverage.

## Scope: What We Say Yes and No To

### Yes
- Bug fixes and security improvements, either side.
- Client: endpoint failover and probe logic, signaling reliability and
  reconnect strategies, offline queue and service-worker bootstrap.
- Go: tunnel reliability, rendezvous protocol conformance, autoscale/cost
  correctness, agent hardening.
- Tests and documentation.

### No — frozen invariants
- **No `.tsx` files in `client/`.** JSX only, or plain JS. (The Go side is
  plain Go — this rule is client-specific.)
- **No Google SSO / OAuth.**
- **No Stripe billing.**
- **No Rust rewrites.**
- Features that require centralized account-linking or billing coordination
  belong in a control plane an operator runs, not in this relay — this repo
  stays control-plane-optional and fully self-hostable on its own.
- New external dependencies (npm or Go module) without prior issue discussion.

## Licensing

Ephor — both the `@vulos/relay-client` SDK and the Go tunnel service —
is MIT-licensed. Contributions inherit MIT. No CLA required.
