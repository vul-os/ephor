import type { BrandMeta } from './types';

/**
 * Pier — the DEFAULT brand.
 *
 * Pier is the broker (coordinator) reference implementation of the KOTVA
 * standard. It is not a Vulos product, so nothing here points at Vulos: the
 * documentation and support links go to the standard's own repository, and
 * `scripts/check-brand-isolation.sh` fails the build if the string "vulos" (or
 * the old name) survives anywhere in this brand's compiled output.
 *
 * Bronze #C89A56 is the value the mark is drawn in.
 */
export const pierMeta: BrandMeta = {
  name: 'Pier',
  wordmark: 'Pier',
  tagline: 'Operator Console',
  description:
    'Pier Operator Console — run a KOTVA coordinator: descriptor, pricing, prepaid billing, keys, and COORD-1..8 conformance.',

  accent: '#c89a56',
  // Bronze is mid-light, so ink on it must be near-black, not white.
  accentContrast: '#0c0c0c',
  accentFill: '#c89a56',
  accentFillContrast: '#14100a',

  storagePrefix: 'pier',
  faviconHref: '/favicon.svg',
  docsUrl: 'https://github.com/vul-os/kotva/blob/main/coordinator/CONTRACT.md',
  supportContact: 'https://github.com/vul-os/kotva/issues',

  markPath: '../brand/pier.svg',
};
