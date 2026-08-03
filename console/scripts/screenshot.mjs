#!/usr/bin/env node
// Reference screenshots of the operator console.
//
// Assumes `pnpm build` has already produced `dist/`. Serves that build
// statically, loads the console in mock mode (the build's default, VITE_MOCK=1)
// and captures the reference set.
//
// Usage: node scripts/screenshot.mjs   (or `pnpm screenshot`, after `pnpm build`)
//
// ── Framed viewport, not full page ──────────────────────────────────────────
// Every shot is a 1440x900 (16:10) frame of what an operator actually sees.
// This used to pass `fullPage: true`, which produces an image as tall as the
// document rather than as tall as the screen, and the results were unusable as
// illustrations: the Overview came out 2948x2508 — very nearly square, so it
// dominated any page it was placed in — and the 390px phone shot came out
// 794x6000, a 1:7.6 ribbon that had hit Chromium's capture ceiling. A console
// screenshot is meant to show the fold, which is the whole subject of the
// layout; a full-page capture shows the fold plus everything the layout was
// designed to push below it.
//
// deviceScaleFactor 2 keeps the type crisp on retina displays. The images are
// therefore 2880x1800 pixels for a 1440x900 frame and are meant to be
// presented at their CSS size (half), which is what site/index.html does.
//
// ── Brand ───────────────────────────────────────────────────────────────────
// The build under dist/ MUST be the default (Pier) one. This repo ships a
// rebrandable console and a gate whose entire purpose is to stop a Pier
// deployment being mistaken for a Vulos-operated service; publishing a
// screenshot taken from `VITE_BRAND=vulos pnpm build` onto the Pier site would
// do exactly that, and no string-scanning gate can see inside a PNG. So it is
// asserted here, at the only point where a wrong-brand build turns into a
// published image.

import { createServer } from 'node:http';
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { existsSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const consoleRoot = path.resolve(__dirname, '..');
const repoRoot = path.resolve(consoleRoot, '..');
const distDir = path.join(consoleRoot, 'dist');

// docs/img/ is what README.md embeds; site/assets/ is what the landing page
// embeds. They used to be kept in step by hand and had silently drifted — the
// copies under site/ were a pre-rename build showing the old name and the old
// mark, which a text grep for the old name could never have found. Writing both
// from one run is what stops that recurring.
const DOCS_IMG = path.join(repoRoot, 'docs', 'img');
const SITE_ASSETS = path.join(repoRoot, 'site', 'assets');

const FRAME = { width: 1440, height: 900 };
const PHONE = { width: 390, height: 844 };

/** file, route, colour scheme, viewport, and every directory it is written to. */
const SHOTS = [
  { file: 'console-dark.png', route: 'overview', scheme: 'dark', out: [DOCS_IMG, SITE_ASSETS] },
  { file: 'console-light.png', route: 'overview', scheme: 'light', out: [DOCS_IMG, SITE_ASSETS] },
  { file: 'console-billing-dark.png', route: 'billing', scheme: 'dark', out: [DOCS_IMG] },
  { file: 'console-billing-light.png', route: 'billing', scheme: 'light', out: [DOCS_IMG] },
  { file: 'console-mobile-dark.png', route: 'overview', scheme: 'dark', viewport: PHONE, out: [DOCS_IMG] },
];

// A marker that must be on screen before the shutter fires, per route. Without
// this a capture can land on the loading skeleton and look like a broken build.
const READY = {
  overview: 'text=Coordinator posture',
  billing: 'text=Prepaid ledger',
};

if (!existsSync(distDir)) {
  console.error('dist/ not found — run `pnpm build` first.');
  process.exit(1);
}

const indexHtml = await readFile(path.join(distDir, 'index.html'), 'utf8');
if (!indexHtml.includes('<title>Pier ')) {
  // Anchored on the CLOSING tag: index.html carries an authoring comment whose
  // text includes the literal "<title>", and an unanchored match reported that
  // comment's first few characters instead of the real title.
  const title = indexHtml.match(/<title>([^<]*)<\/title>/)?.[1] ?? '(none)';
  console.error(
    `dist/ is not the default Pier build — its <title> is "${title}".\n` +
      'These images ship on the Pier README and the Pier landing page; a shot of\n' +
      'another brand would put that brand on both. Rebuild with VITE_BRAND unset:\n' +
      '  env -u VITE_BRAND pnpm build',
  );
  process.exit(1);
}

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2',
  '.woff': 'font/woff',
  '.json': 'application/json',
  '.png': 'image/png',
};

/**
 * Squeeze a capture to a 256-colour palette, in place, if ImageMagick is here.
 *
 * A console screenshot is flat fill and antialiased text — very few distinct
 * tones — so this is a ~55% saving (485 KB → 217 KB on the light Overview) at
 * an RMSE of 0.3%, which is invisible on inspection at 1:1. Worth doing for an
 * image that loads on a landing page.
 *
 * OPTIONAL on purpose: `magick` is not a Node dependency and a contributor
 * without it must still be able to regenerate these. Absent, this no-ops and
 * says so, and the uncompressed PNG is perfectly correct — just larger.
 *
 * The result is only accepted if it actually came out smaller, so a future
 * ImageMagick that inflates the file, or one that writes a truncated image,
 * cannot silently replace a good capture with a worse one.
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

const server = createServer(async (req, res) => {
  try {
    const urlPath = decodeURIComponent(req.url.split('?')[0]);
    let filePath = path.join(distDir, urlPath === '/' ? 'index.html' : urlPath);
    if (!filePath.startsWith(distDir)) throw new Error('bad path');
    if (!existsSync(filePath) || !statSync(filePath).isFile()) {
      filePath = path.join(distDir, 'index.html'); // SPA fallback (hash routing anyway)
    }
    res.writeHead(200, { 'content-type': MIME[path.extname(filePath)] ?? 'application/octet-stream' });
    res.end(await readFile(filePath));
  } catch (e) {
    res.writeHead(500);
    res.end(String(e));
  }
});

await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const { port } = server.address();
const base = `http://127.0.0.1:${port}`;
console.log(`serving dist/ at ${base}`);

const browser = await chromium.launch();

async function shoot({ route, scheme, file, viewport, out }) {
  const vp = viewport ?? FRAME;
  const context = await browser.newContext({ viewport: vp, colorScheme: scheme, deviceScaleFactor: 2 });
  const page = await context.newPage();
  await page.goto(`${base}/#/${route}`, { waitUntil: 'networkidle' });
  const ready = READY[route];
  if (ready) await page.waitForSelector(ready, { timeout: 10_000 });
  // let the mock client's artificial latency + font swap settle
  await page.waitForTimeout(700);

  const [first, ...rest] = out;
  await mkdir(first, { recursive: true });
  const primary = path.join(first, file);
  await page.screenshot({ path: primary });
  const shrink = tryShrink(primary);
  // Copy the identical bytes everywhere else it is needed rather than
  // re-capturing, so the published copies can never differ.
  const bytes = await readFile(primary);
  for (const dir of rest) {
    await mkdir(dir, { recursive: true });
    await writeFile(path.join(dir, file), bytes);
  }
  const kb = (bytes.byteLength / 1024).toFixed(0);
  const note = shrink.shrunk
    ? `(palette-squeezed from ${(shrink.before / 1024).toFixed(0)} KB)`
    : `(uncompressed — ${shrink.reason})`;
  console.log(
    `${file}  ${vp.width * 2}x${vp.height * 2}  ${kb} KB ${note}  →  ` +
      out.map((d) => path.relative(repoRoot, d)).join(', '),
  );
  await context.close();
}

try {
  for (const shot of SHOTS) await shoot(shot);
} finally {
  await browser.close();
  server.close();
}
