import type { BrandMeta } from './types';

/**
 * Vulos Cloud — the first ALTERNATE brand, and the proof that the seam works.
 *
 * It has no privileges the next brand won't have: it is one more file in this
 * directory, selected with `VITE_BRAND=vulos`, changing only the accent, the
 * mark, the wordmark, the links and the storage namespace. Layout, geometry,
 * spacing, typography and every `--bg-*` / `--border-*` / `--text-*` token are
 * shared with every other brand.
 *
 * Teal #0f6a6c is the Vulos accent, verbatim from vulos-static/src/theme.css.
 */
export const vulosMeta: BrandMeta = {
  name: 'Vulos Cloud',
  wordmark: 'Vulos Cloud',
  tagline: 'Operator Console',
  description:
    'Vulos Cloud Operator Console — run a KOTVA coordinator: descriptor, pricing, prepaid billing, keys, and COORD-1..8 conformance.',

  accent: '#0f6a6c',
  // Deep teal is dark, so ink on it must be near-white — the opposite of bronze.
  // This is exactly why the contrast colours are brand fields and not derived.
  accentContrast: '#f2fbfb',
  accentFill: '#0f6a6c',
  accentFillContrast: '#f2fbfb',

  storagePrefix: 'vulos-cloud',
  faviconHref: '/favicon.svg',
  docsUrl: 'https://vulos.org/docs/',
  supportContact: 'hello@vulos.org',

  markPath: 'src/brands/assets/vulos-mark.svg',
};
