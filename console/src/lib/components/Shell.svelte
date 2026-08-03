<script lang="ts">
  import type { Snippet } from 'svelte';
  import { tick } from 'svelte';
  import { router, type Route } from '../router.svelte';
  import { theme } from '../theme.svelte';
  import { IS_MOCK } from '../api';
  // The single branding site in this component. Nothing below branches on WHICH
  // brand is active — the mark, the wordmark and the two footer links are just
  // values, so a rebranded deployment needs no component change at all.
  import { brand } from '$brand';
  import { contactHref, scopeSvgIds } from '../brand-mark';

  let { children }: { children: Snippet } = $props();

  // Three copies of one inline SVG on a page collide on any ids inside <defs>.
  // Give each placement its own id namespace up front rather than relying on
  // the current default mark happening not to use any.
  const markSidebar = scopeSvgIds(brand.markSvg, 'sidebar');
  const markTopbar = scopeSvgIds(brand.markSvg, 'topbar');
  const markFooter = scopeSvgIds(brand.markSvg, 'footer');

  // Mobile: the sidebar collapses to a slide-in drawer behind a hamburger.
  let drawerOpen = $state(false);

  let hamburgerEl = $state<HTMLButtonElement>();
  let navEl = $state<HTMLElement>();

  async function openDrawer() {
    drawerOpen = true;
    // Move focus into the drawer landmark once it's slid into view — a
    // keyboard/AT user who just activated the hamburger should land inside
    // the panel they opened, not stay parked on a now-covered button.
    await tick();
    navEl?.focus();
  }

  function closeDrawer() {
    drawerOpen = false;
    // Return focus to the control that opened the drawer — never strand
    // focus on a menu item that just scrolled out of the viewport.
    hamburgerEl?.focus();
  }

  function go(id: Route) {
    router.go(id);
    if (drawerOpen) closeDrawer(); // dismiss the drawer after navigating on mobile
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && drawerOpen) closeDrawer();
  }

  // Plain labels under small muted section headers, no leading numbers.
  const NAV_GROUPS: { heading: string; items: { id: Route; label: string }[] }[] = [
    {
      heading: 'Posture',
      items: [
        { id: 'overview', label: 'Overview' },
        { id: 'descriptor', label: 'Descriptor' },
        { id: 'conformance', label: 'Conformance' },
      ],
    },
    {
      heading: 'Billing',
      items: [
        { id: 'tariff', label: 'Pricing' },
        { id: 'billing', label: 'Ledger' },
      ],
    },
    {
      heading: 'Identity',
      items: [{ id: 'keys', label: 'Keys' }],
    },
  ];
</script>

<svelte:window onkeydown={onKeydown} />


<div class="shell" class:drawer-open={drawerOpen}>
  <a href="#main" class="skip-link">Skip to content</a>

  <!-- scrim behind the mobile drawer -->
  <button
    type="button"
    class="scrim"
    aria-label="Close menu"
    aria-hidden={!drawerOpen}
    tabindex={drawerOpen ? 0 : -1}
    onclick={closeDrawer}
  ></button>

  <aside class="nav" class:open={drawerOpen} bind:this={navEl} tabindex="-1">
    <div class="brandblock">
      <!-- The brand's own mark, inlined from src/brands/<VITE_BRAND>.ts. Marks
           are currentColor-filled, so .mark tints them with the accent and one
           file holds up on both the near-black and the warm-paper canvas. -->
      <div class="mark" aria-hidden="true">{@html markSidebar}</div>
      <div class="wordblock">
        <span class="word">{brand.wordmark}</span>
        <span class="sub">{brand.tagline}</span>
      </div>
    </div>

    <nav aria-label="Console sections">
      {#each NAV_GROUPS as group (group.heading)}
        <p class="nav-heading">{group.heading}</p>
        <ul class="navlist">
          {#each group.items as item (item.id)}
            <li>
              <button
                type="button"
                class="navitem"
                class:active={router.current === item.id}
                onclick={() => go(item.id)}
                aria-current={router.current === item.id ? 'page' : undefined}
              >
                <span class="lbl">{item.label}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/each}
    </nav>

    <div class="nav-foot">
      <div class="foot-mark" aria-hidden="true">{@html markFooter}</div>
      <p class="foot-note">Binds operator-local by default (127.0.0.1:8090). Bearer-token gated, fail-closed.</p>
      <!-- Both destinations belong to the deployment, not to this codebase: an
           operator who rebrands the console points them at their own docs and
           their own inbox by editing one brand file. -->
      <p class="foot-links">
        <a href={brand.docsUrl} target="_blank" rel="noreferrer noopener">Docs</a>
        <span class="dot" aria-hidden="true">·</span>
        <a href={contactHref(brand.supportContact)}>Support</a>
      </p>
    </div>
  </aside>

  <div class="content-col">
    <header class="topbar">
      <div class="crumbs">
        <button
          type="button"
          class="hamburger"
          aria-label="Open menu"
          aria-expanded={drawerOpen}
          bind:this={hamburgerEl}
          onclick={openDrawer}
        >
          <svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M3 6h18M3 12h18M3 18h18" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>
        </button>
        <span class="crumb-mark" aria-hidden="true">{@html markTopbar}</span>
        <span class="crumb-kicker">Coordinator control plane</span>
      </div>
      <!-- The fixture-data disclosure. It used to be stated TWICE and neither
           placement was good: a bare "Demo data" pill in the sidebar (which
           said nothing about what that meant) and, on the Overview route only,
           a full-sentence banner 1254px down the page — below the fold, so the
           only copy carrying VITE_MOCK=1 was the one an operator had to scroll
           to find. Stated once here instead: the topbar is sticky and shared by
           every route, so the complete sentence is now permanently on screen
           everywhere rather than intermittently off it. -->
      {#if IS_MOCK}
        <p class="mode-strip">
          <span class="light-dot" aria-hidden="true"></span>
          <span><strong>Demo data.</strong> Fixture data (<span class="mono">VITE_MOCK=1</span>), not a live coordinator admin API. See <span class="mono">console/README.md</span>.</span>
        </p>
      {/if}
      <button
        type="button"
        class="theme-toggle"
        role="switch"
        aria-checked={theme.resolved() === 'dark'}
        aria-label={theme.resolved() === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
        onclick={() => theme.toggle()}
      >
        <span class="track" class:dark={theme.resolved() === 'dark'}>
          <span class="thumb">
            {#if theme.resolved() === 'dark'}
              <svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M20 14.5A8.5 8.5 0 119.5 4a7 7 0 1010.5 10.5z" fill="currentColor"/></svg>
            {:else}
              <svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><circle cx="12" cy="12" r="4.5" fill="currentColor"/><path d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
            {/if}
          </span>
        </span>
        <span class="tlabel" aria-hidden="true">{theme.resolved() === 'dark' ? 'Dark' : 'Light'}</span>
      </button>
    </header>

    <main id="main" tabindex="-1">
      {@render children()}
    </main>
  </div>
</div>

<style>
  .skip-link {
    position: absolute;
    left: -999px;
    top: 0;
    background: var(--text-primary);
    color: var(--bg-base);
    padding: 0.6rem 1rem;
    z-index: 100;
  }
  .skip-link:focus {
    left: 0.5rem;
    top: 0.5rem;
  }

  .shell {
    display: grid;
    grid-template-columns: 15.5rem 1fr;
    min-height: 100vh;
  }

  .nav {
    background: var(--bg-surface);
    border-right: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    padding: 1.4rem 1.1rem;
    position: sticky;
    top: 0;
    height: 100vh;
  }

  /* The aside is a focus target (drawer open moves focus here) but should
     never show its own ring on desktop where it's never actually clicked
     into — only the drawer path (mobile, focused programmatically) cares. */
  .nav:focus-visible {
    outline: none;
    box-shadow: none;
  }

  /* Scrim is inert on desktop; only the mobile drawer reveals it. */
  .scrim {
    display: none;
    border: none;
    padding: 0;
  }

  /* ── Mobile: single column; the sidebar becomes a slide-in drawer ── */
  @media (max-width: 900px) {
    .shell {
      grid-template-columns: 1fr;
    }
    .nav {
      position: fixed;
      top: 0;
      left: 0;
      bottom: 0;
      z-index: 60;
      width: 16rem;
      max-width: 82vw;
      height: 100dvh;
      transform: translateX(-100%);
      transition: transform var(--dur) var(--ease);
      box-shadow: var(--shadow-lg);
      overflow-y: auto;
    }
    .nav.open {
      transform: translateX(0);
    }
    .scrim {
      display: block;
      position: fixed;
      inset: 0;
      z-index: 50;
      background: var(--nav-scrim);
      opacity: 0;
      pointer-events: none;
      transition: opacity var(--dur) var(--ease);
      cursor: default;
    }
    .shell.drawer-open .scrim {
      opacity: 1;
      pointer-events: auto;
    }
  }

  .brandblock {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0 0.3rem 1rem;
    margin-bottom: 0.7rem;
    border-bottom: 1px solid var(--border-default);
    position: relative;
  }

  /* A brighter hairline fading toward the trailing edge under the brand
     block, echoing the panel-header underline elsewhere in the console —
     the sidebar masthead gets the same "ruled letterhead" treatment. */
  .brandblock::after {
    content: '';
    position: absolute;
    left: 0.3rem;
    right: 0;
    bottom: -1px;
    height: 1px;
    background: linear-gradient(90deg, var(--border-emphasis), transparent 85%);
  }

  /* ── Brand mark placements ──────────────────────────────────────────────
     The mark is supplied by the brand, so its aspect ratio is NOT known here:
     the default is landscape, the first alternate is near-square, the next one
     could be portrait. So every placement sets a height and lets width follow from
     the viewBox (`width: auto` on a replaced element with an intrinsic ratio),
     with a max-width backstop for a pathologically wide mark. Never the other
     way round — fixing width would squash a portrait mark.

     The marks are currentColor-filled, so `color` here is what tints them. */
  .mark {
    color: var(--accent);
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }
  .mark :global(svg) {
    height: 1.75rem;
    width: auto;
    max-width: 4.5rem;
    display: block;
  }

  .crumb-mark {
    display: flex;
    align-items: center;
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .crumb-mark :global(svg) {
    height: 1rem;
    width: auto;
    max-width: 3rem;
    display: block;
  }

  .foot-mark {
    color: var(--text-faint);
    margin-bottom: var(--space-3);
    display: flex;
  }
  .foot-mark :global(svg) {
    height: 0.9rem;
    width: auto;
    max-width: 3rem;
    display: block;
  }

  .wordblock {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
    min-width: 0;
  }

  .word {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 1.05rem;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .sub {
    font-family: var(--font-mono);
    font-size: 0.69rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .nav-heading {
    font-family: var(--font-mono);
    font-size: 0.69rem;
    font-weight: 600;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-muted);
    margin: 1.2rem 0 0.4rem;
    padding: 0 0.6rem;
  }
  .nav-heading:first-of-type {
    margin-top: 0.3rem;
  }

  /* Same small IDE-grade tick as .panel-kicker elsewhere in the console — a
     prompt mark, not a bullet — so the sidebar's own section labels read as
     part of the same typographic family as every panel header on the page. */
  .nav-heading::before {
    content: '';
    display: inline-block;
    width: 0.5em;
    height: 1px;
    margin-right: 0.5em;
    background: currentColor;
    opacity: 0.6;
    vertical-align: middle;
  }

  .navlist {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .navitem {
    width: 100%;
    display: flex;
    align-items: center;
    padding: 0.48rem 0.65rem;
    border-radius: var(--radius-sm);
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.87rem;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: background-color var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease),
      box-shadow var(--dur) var(--ease), transform var(--dur-fast) var(--ease);
  }

  .navitem:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .navitem:active {
    background: var(--bg-active);
    transition-duration: calc(var(--dur-fast) / 2);
  }

  /* Active state reaches for the tokens app.css built specifically for a
     selection surface (--bg-selected / --bg-selected-border, which app.css
     mixes from the active brand's accent) rather than inventing a fresh
     accent-tint — a precise inset edge plus that surface reads as "this is
     where you are" without the tint feeling like a hover left on by mistake. */
  .navitem.active {
    background: var(--bg-selected);
    color: var(--text-primary);
    font-weight: 600;
    box-shadow: inset 0 0 0 1px var(--bg-selected-border), inset 2px 0 0 var(--accent);
  }

  .navitem.active:hover {
    background: color-mix(in srgb, var(--bg-selected) 85%, var(--bg-hover) 15%);
  }

  .lbl {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* NOT `margin-top: auto`. Six nav items in three groups is ~330px of a
     900px-tall sidebar, so pinning this block to the bottom opened a ~460px
     hole BETWEEN the nav and the footer — a hole reads as a rendering fault,
     whereas the same space left trailing at the bottom of a column reads as
     nothing at all. So the footer now simply follows the nav. */
  .nav-foot {
    margin-top: 1.4rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .foot-note {
    font-size: 0.68rem;
    color: var(--text-tertiary);
    line-height: 1.5;
    margin: 0;
  }

  /* Where this deployment's operator goes for help. Both hrefs are brand
     values, so a rebrand never leaves an operator pointed at someone else's
     documentation or inbox. */
  .foot-links {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-family: var(--font-mono);
    font-size: 0.68rem;
    margin: 0;
  }
  .foot-links a {
    color: var(--text-tertiary);
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: color var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }
  .foot-links a:hover {
    color: var(--accent);
    border-bottom-color: color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .foot-links .dot {
    color: var(--text-ghost);
  }

  .content-col {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.6rem 1rem;
    padding: 0.7rem 1.8rem;
    border-bottom: 1px solid var(--border-default);
    background: var(--nav-scrim);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    box-shadow: var(--shadow-sm);
    position: sticky;
    top: 0;
    z-index: 10;
  }

  .crumbs {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    flex: 0 1 auto;
  }

  .crumb-kicker {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    font-weight: 500;
    letter-spacing: 0.02em;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    /* Without this the ellipsis never engages: a flex item defaults to
       min-width:auto, and for nowrap text that min-content floor is the whole
       string, so .crumbs could not shrink and shoved the theme toggle onto a
       row of its own — a three-row topbar on a 390px phone. */
    min-width: 0;
  }

  /* Amber, and the only amber in the chrome. Across the console amber now means
     exactly one thing — "do not treat this as verified or live" — which covers
     both the declared-not-verified caveat and fixture data. It is deliberately
     NOT the brand bronze: bronze is brand and navigation, and a disclosure
     wearing the brand colour was one of the four different jobs bronze used to
     be doing at once. */
  .mode-strip {
    display: flex;
    align-items: flex-start;
    gap: 0.45rem;
    margin: 0;
    min-width: 0;
    flex: 1 1 22rem;
    font-size: 0.72rem;
    line-height: 1.4;
    color: var(--text-secondary);
    background: var(--status-warning-soft);
    border: 1px solid color-mix(in srgb, var(--status-warning) 38%, transparent);
    border-radius: var(--radius-sm);
    padding: 0.32rem 0.6rem;
  }

  .mode-strip .light-dot {
    color: var(--status-warning);
    margin-top: 0.42rem;
  }

  .mode-strip strong {
    color: var(--status-warning);
    font-family: var(--font-mono);
  }

  /* Below the drawer breakpoint the topbar row has no width to spare, so the
     disclosure takes a full row of its own rather than being ellipsised. It is
     a required statement — it may wrap, but it may never be truncated. */
  @media (max-width: 900px) {
    .mode-strip {
      order: 3;
      flex-basis: 100%;
    }
  }

  /* Hamburger: hidden on desktop, shown when the sidebar is a drawer. */
  .hamburger {
    display: none;
    align-items: center;
    justify-content: center;
    width: 2.1rem;
    height: 2.1rem;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
    transition: color var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease),
      background-color var(--dur-fast) var(--ease);
  }
  .hamburger svg {
    width: 1.1rem;
    height: 1.1rem;
  }
  .hamburger:hover {
    color: var(--text-primary);
    border-color: var(--border-emphasis);
    background: var(--bg-hover);
  }
  .hamburger:active {
    background: var(--bg-active);
  }
  @media (max-width: 900px) {
    .hamburger {
      display: inline-flex;
    }
    /* flex-basis:0, not just min-width:0. Flex assigns items to lines by their
       HYPOTHETICAL main size and only shrinks afterwards, within a line — so
       with basis:auto the crumbs measured their full ~273px, the toggle no
       longer fit beside them and wrapped to a row of its own before any
       shrinking could happen. Zero basis lets both share row one, after which
       grow hands the crumbs whatever is left and the kicker ellipsises. */
    .crumbs {
      flex: 1 1 0;
    }
  }

  .theme-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    padding: 0.2rem;
    border-radius: var(--radius-sm);
  }

  .track {
    width: 2.5rem;
    height: 1.4rem;
    border-radius: var(--radius-full);
    background: var(--bg-base);
    border: 1px solid var(--border-strong);
    display: flex;
    align-items: center;
    padding: 0.12rem;
    transition: background-color var(--dur) var(--ease), border-color var(--dur) var(--ease);
  }

  .track.dark {
    background: color-mix(in srgb, var(--accent) 30%, var(--bg-base));
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border-strong));
    justify-content: flex-end;
  }

  .thumb {
    width: 1.05rem;
    height: 1.05rem;
    border-radius: 50%;
    background: var(--bg-elevated);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--shadow-sm);
    transition: transform var(--dur) var(--ease);
  }

  .theme-toggle:hover .thumb {
    transform: scale(1.06);
  }

  .thumb svg {
    width: 0.7rem;
    height: 0.7rem;
  }

  .tlabel {
    font-family: var(--font-mono);
    font-size: 0.78rem;
    color: var(--text-secondary);
  }

  main {
    padding: 1.5rem 1.8rem;
    max-width: 78rem;
    width: 100%;
    margin: 0 auto;
  }

  @media (max-width: 640px) {
    main {
      padding: 1.1rem;
    }
    .topbar {
      padding: 0.9rem 1.1rem;
    }
  }
</style>
