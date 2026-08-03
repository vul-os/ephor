<script lang="ts">
  import type { VisibilityDto } from '../types';
  import {
    CLASS_LABEL,
    CLASS_DESCRIPTION,
    LEVEL_LABEL,
    LEVEL_DESCRIPTION,
    mustNotPresentAsVerified,
    isVerifiablyBlind,
  } from '../visibility';

  let { visibility, size = 'lg' }: { visibility: VisibilityDto; size?: 'lg' | 'sm' } = $props();

  let warn = $derived(mustNotPresentAsVerified(visibility));
  let verifiablyBlind = $derived(isVerifiablyBlind(visibility));
</script>

<div class="badge" class:sm={size === 'sm'} class:warn>
  <div class="glyph" aria-hidden="true">
    {#if visibility.class === 'terminating'}
      <svg viewBox="0 0 24 24" fill="none"><path d="M6 12h12M14 6l6 6-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
    {:else if verifiablyBlind}
      <svg viewBox="0 0 24 24" fill="none"><path d="M12 3l7 3.5v5c0 4.5-3 8.2-7 9.5-4-1.3-7-5-7-9.5v-5L12 3z" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/><path d="M9 12l2 2 4-4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
    {:else}
      <svg viewBox="0 0 24 24" fill="none"><path d="M12 3l7 3.5v5c0 4.5-3 8.2-7 9.5-4-1.3-7-5-7-9.5v-5L12 3z" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/><path d="M12 8v5M12 16.5h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
    {/if}
  </div>
  <div class="text">
    <div class="class-row">
      <span class="class-label">{CLASS_LABEL[visibility.class]}</span>
      <span class="level-label">{LEVEL_LABEL[visibility.level]}</span>
    </div>
    <p class="desc">{CLASS_DESCRIPTION[visibility.class]}</p>
    {#if warn}
      <div class="assurance-note warn-text">
        <svg class="caveat-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none">
          <path
            d="M12 4.2 21 19.5H3L12 4.2z"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linejoin="round"
          /><path d="M12 10.4v4.1M12 17.3h.01" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
        <p><strong>Declared, not verified.</strong> {LEVEL_DESCRIPTION[visibility.level]} A relying party cannot check this claim independently (CONTRACT §3.4) — never present it as verified.</p>
      </div>
    {:else if verifiablyBlind}
      <p class="assurance-note ok-text">
        <strong>{LEVEL_LABEL[visibility.level]} — verifiable.</strong> {LEVEL_DESCRIPTION[visibility.level]}
      </p>
    {:else}
      <p class="assurance-note">
        <strong>Disclosed trust boundary.</strong> {LEVEL_DESCRIPTION[visibility.level]}
      </p>
    {/if}
  </div>
</div>

<style>
  /* No fill and no 1.5px coloured frame.
     The declared CLASS is the answer to "what is this coordinator", and it was
     being out-shouted by its own footnote: as a filled, amber-bordered box
     wrapping a second nested amber callout, this component took ~66% of its
     card and was the loudest element on the page. A permanent, never-changing
     disclosure rendered as a fat amber alert is how operators learn to ignore
     amber — so the alert chrome is gone and the class is simply set large in
     ink. The caveat keeps its words and its shape (see .warn-text) and loses
     only its volume. */
  .badge {
    display: flex;
    gap: 0.85rem;
    align-items: flex-start;
    padding: 0;
    background: none;
    border: none;
  }

  .badge.sm {
    gap: 0.6rem;
  }

  /* Kept, and kept small. The shield is the one piece of the old alert chrome
     worth carrying over: it marks the assurance level by SHAPE, so the state
     survives greyscale and colour-blindness without a coloured container. */
  .glyph {
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
    margin-top: 0.15rem;
    color: var(--accent);
    border-radius: 50%;
    background: var(--bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-default);
  }

  .badge.warn .glyph {
    color: var(--status-warning);
    border-color: color-mix(in srgb, var(--status-warning) 35%, var(--border-default));
  }

  .glyph svg {
    width: 1.1rem;
    height: 1.1rem;
  }

  .sm .glyph {
    width: 1.6rem;
    height: 1.6rem;
    margin-top: 0;
  }
  .sm .glyph svg {
    width: 0.95rem;
    height: 0.95rem;
  }

  .text {
    min-width: 0;
  }

  /* The headline. Ink, not amber: this is the current state, not a warning. */
  .class-row {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 1.55rem;
    line-height: 1.1;
    letter-spacing: -0.02em;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.55rem;
    flex-wrap: wrap;
  }

  .sm .class-row {
    font-size: 1.05rem;
    gap: 0.4rem;
  }

  /* The assurance level was a same-size sibling behind a "/" separator, which
     read as a second half of the title. It is a qualifier, so it is now a chip:
     clearly attached to the class, clearly subordinate to it. */
  .level-label {
    font-family: var(--font-mono);
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--status-warning);
    background: var(--status-warning-soft);
    border: 1px solid color-mix(in srgb, var(--status-warning) 32%, transparent);
    border-radius: var(--radius-full);
    padding: 0.16rem 0.5rem;
    white-space: nowrap;
  }

  /* Verifiable levels are a pass, not a caveat, so the chip switches semantic
     colour with the meaning rather than staying amber for every level. */
  .badge:not(.warn) .level-label {
    color: var(--status-success);
    background: var(--status-success-soft);
    border-color: color-mix(in srgb, var(--status-success) 32%, transparent);
  }

  .sm .level-label {
    font-size: 0.6rem;
    padding: 0.1rem 0.4rem;
  }

  .desc {
    margin: 0.4rem 0 0;
    font-size: 0.79rem;
    line-height: 1.5;
    color: var(--text-secondary);
    max-width: 58ch;
  }
  .sm .desc {
    display: none;
  }

  .assurance-note {
    margin: 0.7rem 0 0;
    font-size: 0.75rem;
    line-height: 1.5;
    color: var(--text-tertiary);
    max-width: 62ch;
  }

  .assurance-note p {
    margin: 0;
  }

  .sm .assurance-note {
    font-size: 0.72rem;
    margin-top: 0.45rem;
  }

  /* The caveat keeps every word and keeps its SHAPE — its own icon plus a rule
     separating it from the description — so it still reads as a caveat with
     colour stripped entirely. What it loses is volume: the amber fill and the
     inset amber bar are gone, and only the lead phrase stays amber. It is a
     permanent property of a declared claim, not an incident, and permanent
     incident-styling is what teaches operators to stop seeing amber. */
  .warn-text {
    display: flex;
    align-items: flex-start;
    gap: 0.45rem;
    margin-top: 0.7rem;
    padding-top: 0.7rem;
    border-top: 1px solid var(--border-default);
  }

  .caveat-icon {
    width: 0.95rem;
    height: 0.95rem;
    flex-shrink: 0;
    margin-top: 0.15rem;
    color: var(--status-warning);
  }

  .sm .caveat-icon {
    width: 0.85rem;
    height: 0.85rem;
  }

  .warn-text strong {
    color: var(--status-warning);
  }
  .ok-text strong {
    color: var(--accent);
  }
</style>
