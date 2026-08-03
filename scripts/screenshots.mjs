#!/usr/bin/env node
/**
 * screenshots.mjs — the images the Pier documentation site publishes.
 *
 * Writes site/screenshots/, which site/docs/screenshots.md embeds and
 * site/docs.html renders.
 *
 * Usage (from repo root):
 *   pnpm --dir console build     # produce console/dist first
 *   npm run screenshots
 *
 * Prerequisites: Node.js 20+, `npm ci` in scripts/ (installs Playwright +
 * downloads a headless Chromium binary ~170 MB on first run).
 *
 * ── What this used to capture (harness now deleted) ─────────────────────────
 * This script previously served `demo/index.html` — an interactive demo harness
 * for `@vulos/relay-client`, a DIFFERENT project, inherited wholesale when this
 * repository was carved out of its predecessor. Its two outputs were published
 * on the Pier documentation site as Pier's screenshots, so the "Screenshots"
 * chapter of the Pier docs showed another product's UI, headed
 * "@vulos/relay-client — Interactive Demo" and pointing at box.vulos.org.
 *
 * That is worse than a stale asset. Pier is the broker reference implementation
 * of the KOTVA standard and specifically NOT a Vulos product — the console even
 * ships a gate (console/scripts/check-brand-isolation.sh) whose whole purpose is
 * to stop a Pier deployment being mistaken for a Vulos-operated service. That
 * gate scans built text and cannot see inside a PNG, so two images carrying
 * another brand's name sat on the site untouched by it.
 *
 * The site copies were removed first; `demo/index.html`, `docs/SCREENSHOTS.md`
 * and `docs/screenshots/` (hero.png, architecture.png, README.md) have since
 * been deleted outright, so none of the above exists in the tree any more.
 *
 * So this now captures Pier: the operator console, on the routes a reader of
 * the docs would want to see.
 *
 * ── Framed viewport, not full page ──────────────────────────────────────────
 * Captures were `fullPage: true`, which sizes the image to the DOCUMENT rather
 * than the screen: the old hero.png came out 1280x1399, taller than it was
 * wide, and sat in a page dominating it. Every capture here is a 1440x900
 * (16:10) frame at deviceScaleFactor 2 — 2880x1800 pixels, meant to be
 * presented at half that in CSS.
 */

import { chromium } from '@playwright/test';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { existsSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve, dirname, extname, relative, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const DIST = resolve(ROOT, 'console', 'dist');
const OUT_DIR = resolve(ROOT, 'site', 'screenshots');

const FRAME = { width: 1440, height: 900 };
const PHONE = { width: 390, height: 844 };

/**
 * Each entry becomes one file in site/screenshots/ and is referenced by
 * site/docs/screenshots.md. Keep the two in step.
 */
const SHOTS = [
  { file: 'overview-light.png', route: 'overview', scheme: 'light', ready: 'text=Coordinator posture' },
  { file: 'overview-dark.png', route: 'overview', scheme: 'dark', ready: 'text=Coordinator posture' },
  { file: 'conformance.png', route: 'conformance', scheme: 'light', ready: 'text=checklist' },
  { file: 'billing.png', route: 'billing', scheme: 'light', ready: 'text=Prepaid ledger' },
  { file: 'overview-mobile.png', route: 'overview', scheme: 'light', ready: 'text=Coordinator posture', viewport: PHONE },
];

// ── The build must exist, and must be the Pier one ──────────────────────────

if (!existsSync(DIST)) {
  console.error(
    'console/dist not found. Build the console first:\n' + '  pnpm --dir console build',
  );
  process.exit(1);
}

const indexHtml = await readFile(resolve(DIST, 'index.html'), 'utf8');
// Anchored on the closing tag: index.html carries an authoring comment whose
// text includes the literal "<title>", which an unanchored match would find.
const title = indexHtml.match(/<title>([^<]*)<\/title>/)?.[1] ?? '(none)';
if (!title.startsWith('Pier ')) {
  console.error(
    `console/dist is not the default Pier build — its <title> is "${title}".\n` +
      'These images are published on the Pier documentation site; capturing another\n' +
      'brand would put that brand on it, and no text-scanning gate can see inside a\n' +
      'PNG. Rebuild with VITE_BRAND unset:\n' +
      '  env -u VITE_BRAND pnpm --dir console build',
  );
  process.exit(1);
}

// ── Static server for the built console ─────────────────────────────────────

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
};

const server = createServer(async (req, res) => {
  try {
    const urlPath = decodeURIComponent(req.url.split('?')[0]);
    let filePath = join(DIST, urlPath === '/' ? 'index.html' : urlPath);
    if (!filePath.startsWith(DIST)) throw new Error('bad path');
    if (!existsSync(filePath) || !statSync(filePath).isFile()) {
      filePath = join(DIST, 'index.html'); // SPA fallback (hash routing anyway)
    }
    res.writeHead(200, { 'Content-Type': MIME[extname(filePath)] ?? 'application/octet-stream' });
    res.end(await readFile(filePath));
  } catch (e) {
    res.writeHead(500);
    res.end(String(e));
  }
});

await new Promise((res, rej) => server.listen(0, '127.0.0.1', (e) => (e ? rej(e) : res())));
const baseUrl = `http://127.0.0.1:${server.address().port}`;
console.log(`[screenshots] serving console/dist → ${baseUrl}`);

/**
 * Squeeze a capture to a 256-colour palette, in place, if ImageMagick is here.
 * Console UI is flat fill plus antialiased text, so this is roughly a 55%
 * saving at an RMSE of ~0.3% — invisible at 1:1. Optional: `magick` is not a
 * dependency of this package and a contributor without it must still be able to
 * regenerate these; absent, it no-ops and says so. The result is kept only if
 * it actually came out smaller, so an ImageMagick that inflates or truncates
 * the file cannot silently replace a good capture.
 */
function tryShrink(file) {
  const tmp = file + '.q.png';
  try {
    execFileSync('magick', [file, '-colors', '256', '-depth', '8', tmp], { stdio: 'pipe' });
  } catch {
    return { shrunk: false, reason: 'ImageMagick not available' };
  }
  try {
    const before = statSync(file).size;
    const after = statSync(tmp).size;
    if (after > 0 && after < before) {
      execFileSync('cp', [tmp, file]);
      return { shrunk: true, before, after };
    }
    return { shrunk: false, reason: `no saving (${before} -> ${after} bytes)` };
  } finally {
    rm(tmp, { force: true });
  }
}

await mkdir(OUT_DIR, { recursive: true });
const browser = await chromium.launch();

try {
  for (const shot of SHOTS) {
    const vp = shot.viewport ?? FRAME;
    const context = await browser.newContext({
      viewport: vp,
      colorScheme: shot.scheme,
      deviceScaleFactor: 2,
    });
    const page = await context.newPage();
    page.on('console', (m) => m.type() === 'error' && console.warn('[page error]', m.text()));
    await page.goto(`${baseUrl}/#/${shot.route}`, { waitUntil: 'networkidle' });
    // Wait for real content. Without this a capture can land on the loading
    // skeleton, which looks like a broken build in the published docs.
    await page.waitForSelector(shot.ready, { timeout: 10_000 });
    await page.waitForTimeout(700); // mock latency + font swap settle

    const outPath = resolve(OUT_DIR, shot.file);
    await page.screenshot({ path: outPath });
    const squeeze = tryShrink(outPath);
    const kb = (statSync(outPath).size / 1024).toFixed(0);
    const note = squeeze.shrunk
      ? `(palette-squeezed from ${(squeeze.before / 1024).toFixed(0)} KB)`
      : `(uncompressed — ${squeeze.reason})`;
    console.log(`[screenshots] ${shot.file}  ${vp.width * 2}x${vp.height * 2}  ${kb} KB ${note}`);
    await context.close();
  }
} finally {
  await browser.close();
  server.close();
}

console.log(`[screenshots] done → ${relative(ROOT, OUT_DIR)}/`);
