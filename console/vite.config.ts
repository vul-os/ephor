import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

import type { BrandMeta } from './src/brands/types';
import { pierMeta } from './src/brands/pier.meta';
import { vulosMeta } from './src/brands/vulos.meta';

// ─────────────────────────────────────────────────────────────────────────────
// Branding
//
// This console is rebrandable: `VITE_BRAND` picks which file in src/brands/ the
// whole app reads its identity from, and NOTHING outside src/brands/ names a
// brand. Pier — the broker (coordinator) reference implementation of the KOTVA
// standard — is the default; Vulos Cloud is simply the first alternate.
//
// Only the *metadata* half of a brand is imported here (see src/brands/types.ts):
// Vite bundles this config with esbuild, which cannot resolve the virtual module
// id the brand modules use for their mark, so `*.meta.ts` is deliberately kept
// free of non-plain imports.
// ─────────────────────────────────────────────────────────────────────────────

const HERE = dirname(fileURLToPath(import.meta.url));

const BRANDS: Record<string, BrandMeta> = {
  pier: pierMeta,
  vulos: vulosMeta,
};

const DEFAULT_BRAND = 'pier';

function selectBrand(): { id: string; meta: BrandMeta } {
  const id = process.env.VITE_BRAND ?? DEFAULT_BRAND;
  const meta = BRANDS[id];
  if (!meta) {
    // Fail loudly. A typo'd VITE_BRAND silently falling back to the default
    // would ship the wrong identity to a deployment that asked for another one.
    throw new Error(
      `VITE_BRAND="${id}" is not a known brand. Known: ${Object.keys(BRANDS).join(', ')}. ` +
        `Add src/brands/${id}.ts + src/brands/${id}.meta.ts and register it in vite.config.ts.`,
    );
  }
  return { id, meta };
}

/** Read a brand's mark straight off disk, from the single path its meta names. */
function readMark(meta: BrandMeta): string {
  const path = resolve(HERE, meta.markPath);
  try {
    return readFileSync(path, 'utf8');
  } catch {
    throw new Error(
      `Brand "${meta.name}" declares markPath "${meta.markPath}" (${path}) but it could not be read.`,
    );
  }
}

const VIRTUAL_MARK = 'virtual:brand-mark/';

/**
 * Serves `virtual:brand-mark/<brand>` — the brand's mark SVG as a string.
 *
 * The mark is never copied into a TS file: `markPath` in the brand metadata is
 * the one place the location is written, and both this module and the favicon
 * below are generated from it, so they cannot drift apart.
 */
function brandMark(): Plugin {
  return {
    name: 'brand-mark',
    resolveId(id) {
      if (id.startsWith(VIRTUAL_MARK)) return '\0' + id;
      return null;
    },
    load(id) {
      if (!id.startsWith('\0' + VIRTUAL_MARK)) return null;
      const name = id.slice(('\0' + VIRTUAL_MARK).length);
      const meta = BRANDS[name];
      if (!meta) throw new Error(`virtual:brand-mark/${name}: no such brand.`);
      // The marks carry long design-rationale comments — worth keeping on disk,
      // not worth shipping three copies of into the DOM on every page load.
      // XML comments have no rendering semantics, so dropping them is lossless.
      const svg = readMark(meta).replace(/<!--[\s\S]*?-->/g, '').replace(/\n\s*\n/g, '\n');
      return `export default ${JSON.stringify(svg)};`;
    },
  };
}

/**
 * Emits `/favicon.svg` for the active brand.
 *
 * Source is `faviconPath` when the brand declares one, else `markPath`. A favicon
 * is NOT simply the mark at a smaller size: pier ships `brand/favicon.svg`, whose
 * tile enlarges the mark (96/128 rather than 84/128) so the gap between the
 * pilings and the water survives 16px. This plugin used to derive unconditionally
 * from `markPath`, which shipped the untuned bare mark as the tab icon and left
 * the file built for the job unused.
 *
 * The marks are `currentColor`-filled (one file, both themes), so a favicon
 * derived from one — having no parent to inherit `color` from — gets `color`
 * pinned to the brand accent on the root element. A dedicated favicon file
 * normally carries its own fixed palette and needs no pinning.
 */
function brandFavicon(brandId: string, meta: BrandMeta): Plugin {
  const source = () => {
    if (!meta.faviconPath) return readMark(meta);
    const path = resolve(HERE, meta.faviconPath);
    try {
      return readFileSync(path, 'utf8');
    } catch {
      throw new Error(
        `Brand "${meta.name}" declares faviconPath "${meta.faviconPath}" (${path}) but it could not be read.`,
      );
    }
  };

  const build = () => {
    // Strip design-rationale comments, exactly as brandMark() does. These files
    // carry more prose than drawing — pier's favicon was ~1.3KB of comment in a
    // 1.7KB asset — and XML comments have no rendering semantics, so this is
    // lossless. Omitting it here meant every visitor downloaded the commentary.
    const src = source().replace(/<!--[\s\S]*?-->/g, '').replace(/\n\s*\n/g, '\n');
    // A brand whose icon carries its own fixed palette needs no pinning.
    if (!src.includes('currentColor')) return src;
    return src.replace(/<svg\b/, `<svg style="color:${meta.accent}"`);
  };

  return {
    name: 'brand-favicon',
    // The dev server has no dist/ to emit into, so serve it from memory instead.
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url?.split('?')[0] !== '/favicon.svg') return next();
        res.setHeader('Content-Type', 'image/svg+xml');
        res.end(build());
      });
    },
    generateBundle() {
      this.emitFile({ type: 'asset', fileName: 'favicon.svg', source: build() });
      this.info(`emitted favicon.svg for brand "${brandId}" from ${meta.faviconPath ?? meta.markPath}`);
    },
  };
}

/**
 * Stamps the brand into the HTML <head>: title, description, favicon href, and
 * the four accent custom properties app.css derives its whole accent ladder
 * from. Doing it here rather than in a component means the page is correct
 * before a single byte of JS runs — no flash of a default identity.
 */
function brandHtml(meta: BrandMeta): Plugin {
  return {
    name: 'brand-html',
    transformIndexHtml: {
      order: 'pre',
      handler(html) {
        const tokens = [
          `--brand-accent: ${meta.accent};`,
          `--brand-accent-contrast: ${meta.accentContrast};`,
          `--brand-accent-fill: ${meta.accentFill};`,
          `--brand-accent-fill-contrast: ${meta.accentFillContrast};`,
        ].join(' ');

        return {
          html,
          tags: [
            {
              tag: 'title',
              children: `${meta.name} · ${meta.tagline}`,
              injectTo: 'head',
            },
            {
              tag: 'meta',
              attrs: { name: 'description', content: meta.description },
              injectTo: 'head',
            },
            {
              tag: 'link',
              attrs: { rel: 'icon', type: 'image/svg+xml', href: meta.faviconHref },
              injectTo: 'head',
            },
            {
              // app.css only ever *consumes* these, so stylesheet order is
              // irrelevant — a custom property is substituted at use time.
              tag: 'style',
              attrs: { id: 'brand-tokens' },
              children: `:root { ${tokens} }`,
              injectTo: 'head-prepend',
            },
          ],
        };
      },
    },
  };
}

const { id: brandId, meta: brandMeta } = selectBrand();

// https://vite.dev/config/
export default defineConfig({
  // No shared public/ directory: the only static asset this app had was the
  // favicon, and that is now brand-owned and emitted by brandFavicon().
  publicDir: false,
  resolve: {
    alias: {
      // The one import specifier every component uses for its identity.
      $brand: resolve(HERE, `src/brands/${brandId}.ts`),
    },
  },
  plugins: [svelte(), brandMark(), brandFavicon(brandId, brandMeta), brandHtml(brandMeta)],
});
