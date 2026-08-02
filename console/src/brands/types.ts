// The console's brand seam.
//
// This UI is REBRANDABLE, not merely unbranded. Pier is the broker (coordinator)
// reference implementation of the KOTVA standard — anyone may run one, and a
// third-party deployment must never be mistakable for a Vulos-operated service.
// So every branding site in the app (name, mark, accent, favicon, links, and the
// localStorage namespace) reads from ONE object that a deployment supplies.
// `pier.ts` is the default; `vulos.ts` is simply the first alternate brand, with
// no privileges the next one won't have.
//
// To add a brand: drop `<name>.meta.ts` + `<name>.ts` in this directory, register
// it in vite.config.ts's BRANDS map, and build with `VITE_BRAND=<name>`.
// Components never branch on which brand is active — there is exactly one
// `import { brand } from '$brand'` shape and no per-brand conditionals anywhere
// outside this directory.

/**
 * The half of a brand that is plain, importable data.
 *
 * Split out from `Brand` because vite.config.ts needs these values at build time
 * to stamp the HTML <head> (title, description, favicon, accent custom
 * properties) — and vite.config.ts is bundled by esbuild, which cannot resolve
 * Vite's virtual/`?raw` module ids. Keeping the mark's *source* out of this file
 * lets one set of values serve both the browser bundle and the build config,
 * with no duplicated strings to drift.
 */
export interface BrandMeta {
  /** Brand name, used in the document title and the accessible mark label. */
  name: string;
  /** Text rendered as the wordmark in the sidebar masthead. Often `name`. */
  wordmark: string;
  /** Second line under the wordmark, and the second half of the page title. */
  tagline: string;
  /** `<meta name="description">` for this deployment. */
  description: string;

  /** Brand accent as a raw hex. Everything accent-shaped derives from it. */
  accent: string;
  /**
   * Text/glyph colour sitting ON a solid `accent` surface. Cannot be derived in
   * CSS (there is no portable "pick readable ink for this colour" function), so
   * each brand states it: near-black under a mid-light accent, near-white under
   * a dark one.
   */
  accentContrast: string;
  /** Fill role: the true brand colour, read only against its own label. */
  accentFill: string;
  /** Label colour on an `accentFill` surface. Same reasoning as accentContrast. */
  accentFillContrast: string;

  /**
   * Namespace for every localStorage key this build writes. MUST be unique per
   * brand: two brands of this console served from one origin previously shared
   * one hardcoded key namespace and silently overwrote each other's theme choice
   * and admin token.
   */
  storagePrefix: string;
  /** href stamped into `<link rel="icon">`. */
  faviconHref: string;
  /** Where this deployment's operator documentation lives. */
  docsUrl: string;
  /** Email address or URL an operator of this deployment should contact. */
  supportContact: string;

  /**
   * Path to the mark's SVG source, relative to the `console/` directory.
   * Single source of truth: vite.config.ts reads it to emit the favicon, and
   * resolves `virtual:brand-mark/<brand>` from it for the bundle. The mark is
   * never copied into a TS string — it stays one file on disk.
   */
  markPath: string;
}

/** A brand as the app sees it: the metadata plus the inlined mark. */
export interface Brand extends BrandMeta {
  /**
   * Inline SVG source for the mark, filled with `currentColor` so one file
   * serves the sidebar, the topbar and the footer on both themes. Rendered with
   * `{@html}` — it is build-time repo content, never user input.
   */
  markSvg: string;
}
