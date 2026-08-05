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
      ...tseslint.configs.recommended,
    ],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: 'module',
      globals: { ...globals.browser, ...globals.node },
    },
    rules: {
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      // Best-effort teardown/cleanup (`try { pc.close() } catch {}`) appears 9x
      // across call/rtc.ts and call/fabricSignaling.ts: closing an already-closed
      // RTCPeerConnection/BroadcastChannel/track is expected, not exceptional, and
      // there is nothing useful to do with the error at a cleanup call site. This
      // is the option the rule ships for exactly that idiom, not a blanket
      // disable — empty if/for/while blocks elsewhere still error.
      'no-empty': ['error', { allowEmptyCatch: true }],
    },
  },

  // src/__tests__/**/*.ts — vitest suite. `globals: true` in vitest.config.js
  // means describe/it/expect etc. are ambient, not imported.
  {
    files: ['src/__tests__/**/*.ts'],
    languageOptions: {
      globals: { ...globals.browser, ...globals.node, ...globals.vitest },
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
