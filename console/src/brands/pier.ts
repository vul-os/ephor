import type { Brand } from './types';
import { pierMeta } from './pier.meta';
// The mark's only copy lives on disk at `pierMeta.markPath` (../brand/pier.svg,
// a path other tooling in this repo treats as a contract). vite.config.ts
// resolves this virtual id from that field, so the path is written once.
import markSvg from 'virtual:brand-mark/pier';

export const brand: Brand = { ...pierMeta, markSvg };
