import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import svelte from 'eslint-plugin-svelte'
import svelteParser from 'svelte-eslint-parser'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  // dist is the Vite build output; node_modules/.tmp holds svelte-check's
  // tsBuildInfo caches (see tsconfig.app.json's tsBuildInfoFile).
  globalIgnores(['dist', 'node_modules']),

  // src/**/*.ts (brands/, lib/, main.ts) AND the <script> block inside every
  // .svelte file share one typescript-eslint setup so both get the same
  // TS-aware rules, in one place, applied consistently. pier is the only
  // Svelte repo in the fleet, so there is no React plugin here — the app has
  // no React.
  {
    files: ['src/**/*.ts', '**/*.svelte', 'vite.config.ts'],
    extends: [
      js.configs.recommended,
      ...tseslint.configs.recommended,
    ],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: 'module',
      globals: { ...globals.browser },
    },
    rules: {
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    },
  },

  // .svelte components AND the `.svelte.ts`/`.svelte.js` universal-reactivity
  // modules (router.svelte.ts, theme.svelte.ts): eslint-plugin-svelte's base
  // config (svelte:base:setup-for-svelte-script) already routes both
  // extensions through svelte-eslint-parser as the top-level parser, so this
  // block's files list mirrors that exactly and layers typescript-eslint's
  // parser in underneath for the actual TS content.
  ...svelte.configs['flat/recommended'],
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.svelte'],
      },
    },
  },

  // Shell.svelte's three `{@html}` sites inline `brand.markSvg` — a
  // build-time-embedded string from src/brands/<VITE_BRAND>.ts, run through
  // scopeSvgIds() (src/lib/brand-mark.ts) to namespace element ids, never
  // user input or network/DB content. svelte/no-at-html-tags exists to catch
  // XSS from untrusted content, which this isn't; downgraded to warn (not
  // disabled) here specifically, scoped to this one file, as a documented
  // judgement call rather than a blanket suppression.
  {
    files: ['src/lib/components/Shell.svelte'],
    rules: {
      'svelte/no-at-html-tags': 'warn',
    },
  },

  // Node-side tooling configs (Vite, svelte.config.js) and scripts/ run under
  // Node, not the browser. scripts/ is out of scope for this task but must
  // still parse. vite.config.ts itself is real TS (imports `type Plugin`
  // etc.) so it's handled by the typescript-eslint block above instead.
  {
    files: ['*.config.js', 'svelte.config.js', 'scripts/**/*.{js,mjs}'],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: 'module',
      globals: { ...globals.node },
    },
  },
])
