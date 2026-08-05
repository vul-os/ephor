import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  // dist-lib/dist-harness are build output; test-results is Playwright's own
  // report directory. None of these is source we own.
  globalIgnores(['dist-lib', 'dist-harness', 'node_modules', 'test-results', 'coverage']),

  // src/**/*.ts — the TypeScript-migrated SDK source. No React/JSX rules:
  // pier-client has no React of its own (react is an optional peerDependency
  // consumed by callers, not authored here).
  {
    files: ['src/**/*.ts'],
    extends: [
      js.configs.recommended,
      ...tseslint.configs.recommendedTypeChecked,
    ],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: 'module',
      globals: { ...globals.browser, ...globals.node },
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      // Best-effort teardown/cleanup (`try { pc.close() } catch {}`) appears 8x
      // across call/rtc.ts and call/fabricSignaling.ts: closing an already-closed
      // RTCPeerConnection/BroadcastChannel/track is expected, not exceptional, and
      // there is nothing useful to do with the error at a cleanup call site. This
      // is the option the rule ships for exactly that idiom, not a blanket
      // disable — empty if/for/while blocks elsewhere still error. (A 9th
      // site, rtc.ts's `this._audioCtx?.close()`, was moved OFF this idiom
      // during the type-aware lint pass: AudioContext.close() is async, so a
      // sync try/catch never actually caught its rejection — it now uses
      // `.catch()` instead, see rtc.ts's leave().)
      'no-empty': ['error', { allowEmptyCatch: true }],
    },
  },

  // src/__tests__/**/*.ts — vitest suite. `globals: true` in vitest.config.js
  // means describe/it/expect etc. are ambient, not imported.
  //
  // Type-aware rules downgraded to `warn` HERE ONLY (src/ stays `error`),
  // measured on the real recommendedTypeChecked run before any fix landed.
  // Each is a mock/fixture-boundary pattern, not a defect:
  //   require-await (151)          — `vi.fn(async () => ({...}))` mocks must be
  //                                   async to match the real fetch/WebSocket
  //                                   signature being stubbed even when the
  //                                   fake body never itself awaits.
  //   no-unnecessary-type-assertion (77) — almost all `arr[i]!` on fixture
  //                                   arrays; client deliberately did NOT
  //                                   enable noUncheckedIndexedAccess (see
  //                                   tsconfig.app.json comment in console for
  //                                   the parallel decision), so indexed access
  //                                   is already non-optional and the `!` is a
  //                                   no-op. Same tsconfig tradeoff already
  //                                   accepted for src/, just visible here at
  //                                   volume because fixtures index arrays a lot.
  //   no-unsafe-member-access (90), no-unsafe-assignment (46),
  //   no-unsafe-argument (6), no-unsafe-return (2) — `JSON.parse()` on
  //                                   captured WebSocket/fetch mock frames and
  //                                   `String()`/template-literal stringifying
  //                                   of a mocked `RequestInit.body` (typed
  //                                   `BodyInit`, a union `no-base-to-string`
  //                                   can't resolve to a safe `toString`).
  //   no-base-to-string (11)       — same mocked-`RequestInit.body` cause.
  // No-floating-promises / no-misused-promises / unbound-method measured 0 in
  // this directory and are NOT touched here — they stay `error` everywhere.
  {
    files: ['src/__tests__/**/*.ts'],
    languageOptions: {
      globals: { ...globals.browser, ...globals.node, ...globals.vitest },
    },
    rules: {
      '@typescript-eslint/require-await': 'warn',
      '@typescript-eslint/no-unnecessary-type-assertion': 'warn',
      '@typescript-eslint/no-unsafe-member-access': 'warn',
      '@typescript-eslint/no-unsafe-assignment': 'warn',
      '@typescript-eslint/no-unsafe-argument': 'warn',
      '@typescript-eslint/no-unsafe-return': 'warn',
      '@typescript-eslint/no-base-to-string': 'warn',
    },
  },

  // signaling.js and rendezvousSignaling.js are DELIBERATELY unconverted JS —
  // see kotva-client.pin.json: pier speaks a wider signaling protocol than
  // kotva's core (extra signal kinds, toId: null broadcast, extra data/identity
  // fields) so these stay JS with a hand-written .d.ts sidecar rather than
  // being forced into kotva's narrower TS types. Lint as plain JS; do not
  // convert.
  {
    files: ['src/**/*.js'],
    extends: [js.configs.recommended],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: 'module',
      globals: { ...globals.browser, ...globals.node },
    },
    rules: {
      'no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    },
  },

  // signaling.js / rendezvousSignaling.js are not part of tsconfig.json's
  // `include` (only src/**/*.ts is), so the typed block above never resolves
  // type information for them. tsconfig.pinned-js.json is a standalone,
  // eslint-only TS program covering just these two files (allowJs+checkJs) —
  // that's what actually makes type-aware rules (specifically
  // no-floating-promises / no-misused-promises) run against them.
  // `parserOptions.project` (classic mode) is used here instead of
  // `projectService`'s `allowDefaultProject`: the latter was tried first and
  // measured to be order-dependent across a full `eslint .` run — sometimes
  // "was not found by the project service", sometimes fine, depending on
  // what else in the same run touched the project service first. `project`
  // pointed at a dedicated tsconfig had no such flakiness across 10
  // consecutive full-repo runs. Verified empirically: without this block
  // these two get ZERO type-aware findings, not because they're clean, but
  // because nothing typed them. Findings are reported in the branch notes,
  // not fixed — both files are kotva-client.pin.json byte-pinned (see the
  // exclusion block below).
  {
    files: ['src/signaling.js', 'src/rendezvousSignaling.js'],
    extends: [...tseslint.configs.recommendedTypeChecked],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.pinned-js.json'],
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },

  // The 8 modules kotva-client.pin.json byte-hashes against kotva's
  // bindings/js/src, enforced by `npm run kotva:check`. A fix here can only
  // land upstream in kotva and then be re-synced via `npm run kotva:pin` —
  // editing the copy in this repo would itself be the drift the pin exists to
  // catch. Rules are turned off per the specific finding ESLint actually
  // reported, not blanket-suppressed:
  //   prefer-const                        — chunkProof.ts (3: loop-local
  //                                          `let`s never reassigned)
  //   no-unused-vars                      — rendezvousSignaling.js (2: unused
  //                                          bindings)
  //   no-useless-assignment                — signaling.js (1: dead store)
  //   @typescript-eslint/no-explicit-any   — rendezvous.ts (2: `any` usages)
  //
  // Added when type-aware linting was turned on (see the two typed blocks
  // above). Every finding below was read and is reported in the branch notes
  // — NONE was fixed, because these 8 files cannot be edited:
  //   @typescript-eslint/no-unused-vars    — rendezvousSignaling.js (2): the
  //       SAME 2 bindings the base no-unused-vars above already covers,
  //       re-flagged under the typed rule name because the typed block below
  //       layers recommendedTypeChecked on top of it. Not a new finding.
  //   @typescript-eslint/no-unnecessary-type-assertion
  //     — chunkProof.ts (21), relayBox.ts (4), rendezvous.ts (1): redundant
  //       `!`/`as X` where the target already has that type.
  //   @typescript-eslint/no-unsafe-assignment / -member-access / -return
  //     — rendezvous.ts (3/2/2 — JSON.parse'd fetch responses used untyped);
  //       signaling.js / rendezvousSignaling.js (41+6 / 10+20 / 1 — plain JS
  //       with no JSDoc types, so typescript-eslint's default-project
  //       fallback resolves most locals as `any` or unresolvable).
  //   @typescript-eslint/no-unsafe-argument / no-unsafe-call / require-await
  //     — signaling.js (12) / rendezvousSignaling.js (29): same plain-JS
  //       cause. no-unsafe-call (1, signaling.js:659) and require-await (1,
  //       signaling.js:326, the async WS 'open' listener discussed in the
  //       branch notes) are single occurrences of the same root cause.
  //   @typescript-eslint/no-floating-promises / no-misused-promises — each of
  //     the 5 findings was read individually (see branch notes), NOT
  //     rubber-stamped off: signaling.js:335 is a GENUINE BUG —
  //     `this._buildJoinPayload().then((join) => this._send(join))` has no
  //     rejection handler, so a signing failure (WebCrypto ECDSA sign, inside
  //     _buildJoinPayload → signFrame) on the very first join silently drops
  //     it and the peer never announces itself. signaling.js:326/341 are the
  //     same `ws.addEventListener('open'/'message', async () => ...)` shape —
  //     326 is this exact bug surfacing through the DOM listener-return-type
  //     check; 341's `await this._processSignal(...)` is unguarded at the
  //     call site too and worth the same scrutiny upstream.
  //     rendezvousSignaling.js:202/203/310 are NOT bugs: `_loopBoard()` /
  //     `_loopInbox()` are `while (!this._stopped)` background loops that
  //     wrap every iteration in try/catch and never let the outer promise
  //     reject; `connect()` (called from the :310 setTimeout) wraps its
  //     entire body in try/catch and calls `_scheduleReconnect()` on failure
  //     instead of rejecting. A line-scoped eslint-disable-next-line comment
  //     is not possible without editing the pinned bytes, so all 5 are
  //     silenced here, file-wide across the pinned set, with the per-line
  //     reasoning kept here instead of inline.
  {
    files: [
      'src/chunkProof.ts',
      'src/relayBox.ts',
      'src/prekeys.ts',
      'src/rendezvous.ts',
      'src/secureTransport.ts',
      'src/errors.ts',
      'src/signaling.js',
      'src/rendezvousSignaling.js',
    ],
    rules: {
      'prefer-const': 'off',
      'no-unused-vars': 'off',
      'no-useless-assignment': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-unused-vars': 'off',
      '@typescript-eslint/no-unnecessary-type-assertion': 'off',
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/no-unsafe-member-access': 'off',
      '@typescript-eslint/no-unsafe-return': 'off',
      '@typescript-eslint/no-unsafe-argument': 'off',
      '@typescript-eslint/no-unsafe-call': 'off',
      '@typescript-eslint/require-await': 'off',
      '@typescript-eslint/no-floating-promises': 'off',
      '@typescript-eslint/no-misused-promises': 'off',
    },
  },

  // Root-level Node tooling configs (Vite/Vitest/Playwright) and the parity
  // script. scripts/ is out of scope per the task, but the config itself must
  // still parse cleanly if it's ever touched.
  {
    files: ['*.config.js', '*.config.mjs'],
    languageOptions: {
      globals: { ...globals.node },
    },
  },

  // Playwright e2e suite + its React harness (a CONSUMER of the built
  // package, not source we lint for library-code rules).
  {
    files: ['e2e/**/*.{js,jsx}'],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: 'module',
      globals: { ...globals.browser, ...globals.node },
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  },
])
