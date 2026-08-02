# Pier brand

The Pier mark is **a deck carried on three pilings, standing over open water**.
It is drawn as strokes with round caps — five lines, no fills, no gradients, no
more than one colour on the ground. Hold it at any size and it is the same
thing: a piece of infrastructure with water running underneath it and past it.

There are two marks:

- **`pier.svg`** — the mark. This is the logo, used everywhere: app icons,
  favicon, the console's sidebar lockup, product listings.
- **`pier-combined.svg`** — the paired mark: two piers on one waterline, the far
  one recessive. Reserved for the console's topbar and footer.

Both are drawn with `stroke="currentColor"` and are sized by width (the single
mark is landscape, 88×66; the paired mark 184×66). That is deliberate: a fixed
stroke colour cannot work across this product's two canvases — white disappears
on the warm-paper light theme and near-black disappears on the `#0c0c0c` dark
theme. Set `color` on the parent and the mark follows the theme. The paired mark
keeps its two-tone relationship by giving the far pier `opacity: 0.45` rather
than a second hardcoded hue, so the pairing survives on any surface.

Pier is a **sibling** of the [Kotva](../../kotva) mark, and deliberately so: same
construction (stroked line-art, round caps, warm hue on a tinted near-black
tile), different object. Kotva draws an anchor — the thing that holds. Pier
draws the structure the anchored vessel ties up against. The family reads
kotva → pier → depot.

## Concept

A **pier** is infrastructure that serves vessels it does not own. It is built,
maintained and paid for by someone; it is used by anyone who reaches it; and the
vessel that ties up is under no obligation to it, comes and goes on its own
schedule, and can use a different pier tomorrow. Many independent piers exist.
None of them is the harbour.

Pier the product is **the broker (coordinator) reference implementation of the
KOTVA standard**. It brokers reach between parties, is **content-blind** (it
carries sealed traffic it cannot read), is **hired, not depended-on**, and is
**swappable**. Pier is *not* a Vulos product, and a third party running pier must
never look Vulos-operated — the mark carries no Vulos chrome for exactly that
reason.

This mark replaced one built on an older name for the same product — a name that
meant *overseer*, and so pointed at authority. Authority is precisely what a
KOTVA coordinator is **not**: it is accountable, swappable, and never
load-bearing. "Pier" points at the architecture exactly, and the mark has to as
well; anything in it that reads as a seat of authority is a defect, not a style
choice. (The full naming rationale lives in the repo's top-level prose.)

The mark draws that directly:

- **The water runs past the deck.** The wave spans the mark's full width; the
  deck stops short of it. Beyond the last piling there is open water and nothing
  else. **No vessel is drawn** — that absence is the point. The berth belongs to
  whoever ties up there, never to the pier.
- **The wave is load-bearing, not ornament.** A straight bar under three legs
  reads as a plinth beneath columns, and the whole mark turns into a bank or a
  temple — the exact "authority" reading the rename exists to kill. A wave
  cannot be a plinth. This was verified by rasterising, not by assuming: the
  solid-base drafts were rejected at 16px for reading as a classical facade.
- **Three pilings, evenly spaced, deck overhanging both ends.** The structure
  continues past what holds it up. It is a span, not a monument.
- **Stroked, not solid.** Line-art keeps it light and keeps it in Kotva's
  family. Solid masses read as masonry; struts read as built infrastructure.
- **Tinted near-black tile, one bronze accent.** The tile is the Vulos-standard
  near-black surface (not a loud brand gradient); bronze is the one accent
  against it, kept restrained rather than glowing.

## Palette — "Bronze"

Vulos's cool near-black surfaces stay as-is; Pier's identity is carried entirely
by one warm accent, used sparingly, never as a full-bleed gradient tile. The
mark itself is **one colour** — there is no secondary mark hue to keep in sync.

| Token | Hex | Use |
|-------|-----|-----|
| **Bronze (canonical accent)** | **`#C89A56`** | the mark, favicon accent, console theme accent — the one value every surface should agree on |
| Bronze-ink (text-on-light) | `#8B5A2B` | wordmark fill on light backgrounds — deeper than the accent for legibility on white/cream |
| Tile ground | `#14100A` | the app tile. A near-black tinted toward bronze, at the same luminance as kotva's `#0f0d0b` and zana's `#14110b`, and identical to the console's `--accent-fill-contrast`. **Do not lighten it** — a ground at twice this brightness vanishes against the dark product grid these tiles sit in. |
| OG tile ground | `#1F1810` | og-image only: one step up from the card ground so the tile edge is visible against it |
| OG water | `#846639` | og-image only: the card's waterline rule |
| OG tagline cream | `#EAD4A6` | og-image tagline text |
| OG muted warm | `#B99A76` | og-image sub-tagline text |

`#C89A56` is the single source of truth for "Pier bronze" — the console uses this
same hex for its accent so the product mark and the product UI agree. No teal,
no purple, no Iris blue: this is a one-accent palette by design.

## Geometry

The mark is five paths in an `0 0 88 66` box, `stroke-width="8"`,
`stroke-linecap="round"`:

| Path | `d` |
|------|-----|
| deck | `M4 4 H72` |
| piling 1 | `M12 4 V40` |
| piling 2 | `M38 4 V40` |
| piling 3 | `M64 4 V40` |
| water | `M4 58 Q14 52 24 58 Q34 64 44 58 Q54 52 64 58 Q74 64 84 58` |

On the 128 tile it is placed at `x="22" y="32.5" width="84" height="63"` — 84 of
128 (66%), per the Vulos product-logo standard, so its optical weight matches the
rest of the fleet. `favicon.svg` is the one exception and says why in-file.

## Files

| File | Use |
|------|-----|
| `pier.svg` | **The mark.** Landscape 88×66, `currentColor`, no tile — the source the assets below are built from, and what the console inlines. |
| `pier-combined.svg` | **The paired mark.** Landscape 184×66, `currentColor` with the far pier at `opacity: 0.45` and one shared waterline. Console topbar + footer only. |
| `logo.svg` | The canonical 128 app tile, to the Vulos product-logo standard (128 box, `rx 28`, flat `#14100A` ground, mark at 84/128). |
| `logo-mark.svg` | The same tile under its historic name, because the repo README header and `make-icons.mjs` reference this path. |
| `logo-mono.svg` | Single-colour mark via `currentColor` — light/dark UI, print, watermarks. Identical geometry to `pier.svg`; the mark is already one colour, so there is no reduction to make. |
| `favicon.svg` | Same tile, mark enlarged to 96/128 within it so the gap above the water survives to 16px. |
| `wordmark.svg` | Mark tile + "Pier" lockup for headers/navbars. |
| `og-image.svg` | 1200×630 social card: mark, wordmark, and "The broker reference implementation of the KOTVA standard". |
| `make-icons.mjs` | `node brand/make-icons.mjs` — rasterizes the above into `icons/` (16 through 512px, apple-touch-icon, favicon-16/32, og-image.png). Uses `rsvg-convert` if present, falls back to `npx playwright`. |
| `icons/` | Generated PNGs (not hand-maintained — regenerate via `make-icons.mjs`). |

The root `logo.png` is `logo-mark.svg` rasterized at 512×512
(`rsvg-convert -w 512 -h 512 brand/logo-mark.svg -o logo.png`).

## Type

No external fonts are embedded or required. `wordmark.svg` and `og-image.svg` set
"Pier" with a system font stack (`system-ui, -apple-system, 'Segoe UI', Roboto,
sans-serif` / `'Helvetica Neue', Arial, sans-serif`) at a heavy weight — this keeps
the files small and dependency-free; it renders with whatever the OS's default UI
font is rather than a fixed typeface. If a locked, font-independent wordmark is ever
needed (e.g. for print), convert the `<text>` node to outlined `<path>` data with a
tool like `svg-text-to-path` and drop the `font-family`/`font-weight` attributes.

## Usage

- Keep clear space ≈ the tile corner radius around the mark.
- Don't recolor the bronze accent, stretch, skew, or add effects. Use
  `logo-mono.svg` when one flat color is needed.
- **Keep the wave, and keep it wavy.** Straightening it into a bar turns the mark
  into a colonnade on a plinth — a building, an authority. That is the one
  reading this brand exists to avoid.
- **Keep the water longer than the deck.** The overhang of water past the pier's
  end is what says the pier ends and the world does not. Don't trim it flush.
- **Don't add a boat.** The empty berth is the idea. A pier serves vessels it
  does not own; drawing one would claim it.
- Size by width, never by height. The single mark is landscape (88×66) and the
  paired mark wider still (184×66); forcing either into a square box squashes it.
- Don't hardcode a stroke colour on the masters. `pier.svg`, `pier-combined.svg`
  and `logo-mono.svg` are `currentColor` so one file works on the near-black and
  the warm-paper canvas — set `color` on the parent instead.
- The mark is thin-stroked, so check it at 16px after any change; the gap between
  the pilings' feet and the water is the first thing to close up, which is why
  `favicon.svg` enlarges the mark within its tile. Rasterise and look, don't
  assume — every draft of this mark that was judged on the source rather than on
  a 16px render was wrong.
