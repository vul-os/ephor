/**
 * vite.config.lib.js — library build for pier-client.
 *
 * Produces dist-lib/ with ESM + CJS bundles, one entry per subpath.
 * Externalizes react / xlsx so consumers can dedupe
 * (they're declared as optional peerDependencies in package.json).
 *
 * Usage: vite build --config vite.config.lib.js
 */

import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

const dir = import.meta.dirname

// NOTE: errors / signaling / rendezvous / rendezvousSignaling / chunkProof stay
// .js on purpose — they are kotva-client substrate modules pinned byte-for-byte
// by scripts/check-kotva-parity.mjs (see kotva-client.pin.json). They ship real
// types via hand-written .d.ts sidecars instead of being converted to .ts.
const entries = {
  index:             resolve(dir, 'src/index.ts'),
  errors:            resolve(dir, 'src/errors.js'),
  endpoints:         resolve(dir, 'src/endpoints.ts'),
  health:            resolve(dir, 'src/health.ts'),
  offlineBootstrap:  resolve(dir, 'src/offlineBootstrap.ts'),
  signaling:         resolve(dir, 'src/signaling.js'),
  fabric:            resolve(dir, 'src/fabric.ts'),
  rendezvous:        resolve(dir, 'src/rendezvous.js'),
  rendezvousSignaling: resolve(dir, 'src/rendezvousSignaling.js'),
  chunkProof:        resolve(dir, 'src/chunkProof.js'),
  presence:          resolve(dir, 'src/presence.ts'),
  call:              resolve(dir, 'src/call/index.ts'),
  useLiveCursors:    resolve(dir, 'src/useLiveCursors.ts'),
  roundTripCheck:    resolve(dir, 'src/roundTripCheck.ts'),
  regionPop:         resolve(dir, 'src/regionPop.ts'),
}

export default defineConfig({
  plugins: [react()],
  define: {
    'import.meta.env.VITE_BUILD_TARGET': JSON.stringify('lib'),
  },
  build: {
    lib: {
      entry: entries,
      formats: ['es', 'cjs'],
      fileName: (format, entryName) =>
        format === 'es' ? `${entryName}.js` : `${entryName}.cjs`,
    },
    outDir: 'dist-lib',
    emptyOutDir: true,
    sourcemap: false,
    rollupOptions: {
      external: [
        'react',
        'react-dom',
        'react/jsx-runtime',
        'xlsx',
      ],
      output: {
        // Keep the upstream `@license` / `@preserve` banners in the bundled
        // output. MIT, BSD and ISC all require the copyright notice to travel
        // with every copy, and a bundle IS a copy. Vite 8 bundles with
        // rolldown, whose minifier drops every comment unless this is set —
        // note that `esbuild.legalComments` does NOT do it, because in Vite 8
        // the esbuild options only reach the CSS pipeline.
        comments: { legal: true },
        exports: 'named',
        globals: {
          react: 'React',
          'react-dom': 'ReactDOM',
          'react/jsx-runtime': 'ReactJSXRuntime',
          xlsx: 'XLSX',
        },
      },
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['src/**/*.test.{js,jsx}', 'src/__tests__/**/*.test.{js,jsx}'],
  },
})
