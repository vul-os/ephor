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

  // The 8 modules kotva-client.pin.json byte-hashes against kotva's
  // bindings/js/src, enforced by `npm run kotva:check`. A fix here can only
  // land upstream in kotva and then be re-synced via `npm run kotva:pin` —
  // editing the copy in this repo would itself be the drift the pin exists to
  // catch. Rules are turned off per the specific finding ESLint actually
  // reported, not blanket-suppressed. Re-measured after re-pinning to kotva
  // main @ db96b9f (see kotva-client.pin.json): that sync incidentally
  // narrowed rendezvous.ts's two `Promise<any>` return types to
  // `Promise<unknown>` with explicit casts at the JSON-parse boundary, and
  // rewrote chunkProof.ts's three reassigned-`let` destructures as fresh
  // `const`s — both fixed findings this list used to carry (`no-explicit-any`
  // / `no-unsafe-assignment` / `no-unsafe-member-access` / `no-unsafe-return`
  // for rendezvous.ts, `prefer-const` for chunkProof.ts). prekeys.ts,
  // rendezvous.ts, secureTransport.ts, and errors.ts now report ZERO findings
  // — verified via `npx eslint <file>` on each — so only the two files below
  // still need an exemption:
  //   @typescript-eslint/no-unnecessary-type-assertion
  //     — chunkProof.ts (21), relayBox.ts (4): redundant `!`/`as X` where the
  //       target already has that type.
  //
  // signaling.ts and rendezvousSignaling.ts (adopted from kotva's real
  // TypeScript source in place of the former hand-written JS transliteration
  // + .d.ts sidecar) are likewise NOT in this files list: `npx eslint
  // src/signaling.ts src/rendezvousSignaling.ts` under the plain
  // `src/**/*.ts` block above (recommendedTypeChecked, no overrides) reports
  // zero findings — verified empirically, not assumed — so they need no
  // exemption either. The signaling.js:335 unhandled-join-rejection bug this
  // list used to document (a signFrame failure silently dropped the join
  // frame) is fixed upstream as of kotva commit 86b5bbe: _buildJoinPayload()
  // .then() now has a .catch() that dispatches an 'error' CustomEvent instead.
  {
    files: [
      'src/chunkProof.ts',
      'src/relayBox.ts',
    ],
    rules: {
      '@typescript-eslint/no-unnecessary-type-assertion': 'off',
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
