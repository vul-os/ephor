/**
 * vitest.config.js — test runner config for pier-client.
 *
 * Split from vite.config.lib.js so the library build (`npm run build`) and the
 * test runner (`npm test`) don't share each other's transform pipeline — the
 * lib build externalises react/xlsx, but the test runner needs them
 * resolved in-process.
 *
 * `testTimeout`: chunkProof.test.js walks every chunk index of every tree shape
 * up to n = 64 — 2080 proofs, deliberately exhaustive rather than sampled — and
 * builds a 4096-leaf tree. That is tens of seconds of real BLAKE3 work on a
 * laptop (10 s for the file alone, 31 s under the full suite's parallel load)
 * and it blows vitest's 5 s default, which is a limit on the CLOCK and not on
 * the assertions: the walk terminates and every one of the 2080 proofs
 * verifies. The exhaustive walk is the point of the test, so the budget is
 * raised rather than the coverage cut.
 *
 * The value matches kotva's bindings/js/vitest.config.js, which carries the
 * same two cases byte-identical (they are part of the hash-pinned substrate)
 * and hit the same wall for the same reason.
 */

import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: true,
    include: [
      'src/**/*.test.{js,jsx,ts,tsx}',
      'src/__tests__/**/*.test.{js,jsx,ts,tsx}',
    ],
    testTimeout: 120_000,
  },
})
