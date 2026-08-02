// Inlining one SVG more than once on a page is an id collision waiting to
// happen: `<defs>` children (masks, gradients, clip paths, filters) live in a
// document-global namespace, so the second copy's `url(#foo)` resolves to the
// FIRST copy's element. The symptom is subtle — one instance renders correctly
// and the others quietly inherit its geometry — which is exactly the kind of
// defect a screenshot review misses.
//
// The console renders the brand mark three times (sidebar masthead, topbar,
// sidebar footer). The default Pier mark happens to be plain rectangles with no
// ids at all, so today this is a no-op. It is here for the brand after next:
// a rebrander who supplies a mark with a gradient must not have to discover
// this hazard themselves.

const ID_ATTR = /\bid="([^"]+)"/g;

/**
 * Rewrite every `id` in an inline SVG (and every reference to it) so that N
 * copies of the same mark on one page cannot capture each other's `<defs>`.
 *
 * Handles the three ways SVG points at an id: `url(#x)` in a presentation
 * attribute or inline style, `href="#x"` / `xlink:href="#x"` on `<use>`, and
 * the bare-id form `attr="#x"` used by `clip-path`/`mask`/`filter` in older
 * markup.
 *
 * @param svg  raw SVG source
 * @param uid  suffix unique to this instance (e.g. 'topbar')
 */
export function scopeSvgIds(svg: string, uid: string): string {
  const ids = new Set<string>();
  for (const match of svg.matchAll(ID_ATTR)) ids.add(match[1]);
  if (ids.size === 0) return svg;

  let out = svg;
  for (const id of ids) {
    const scoped = `${id}--${uid}`;
    const esc = id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    out = out
      .replace(new RegExp(`\\bid="${esc}"`, 'g'), `id="${scoped}"`)
      .replace(new RegExp(`url\\(#${esc}\\)`, 'g'), `url(#${scoped})`)
      .replace(new RegExp(`(href|xlink:href)="#${esc}"`, 'g'), `$1="#${scoped}"`)
      .replace(new RegExp(`(clip-path|mask|filter)="#${esc}"`, 'g'), `$1="#${scoped}"`);
  }
  return out;
}

/**
 * `true` when the address is an email rather than a URL, so a single
 * `supportContact` brand field can carry either without the components needing
 * to know which brand they are rendering.
 */
export function isEmail(contact: string): boolean {
  return contact.includes('@') && !contact.includes('://');
}

/** `href` for a `supportContact`, mailto-wrapping it when it is an address. */
export function contactHref(contact: string): string {
  return isEmail(contact) ? `mailto:${contact}` : contact;
}
