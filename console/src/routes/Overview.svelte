<script lang="ts">
  import { client } from '../lib/api';
  import type { ReportDto, ReceiptsResponse, PrepaidAccount, SignedDescriptorDto } from '../lib/types';
  import VisibilityBadge from '../lib/components/VisibilityBadge.svelte';
  import ConformanceStrip, { CLAUSE_TITLE } from '../lib/components/ConformanceStrip.svelte';
  import StatCard from '../lib/components/StatCard.svelte';
  import { ledgerMoney, kindQuantity, kindLabel, integer } from '../lib/format';
  import { router } from '../lib/router.svelte';
  import type { ResourceKind } from '../lib/types';
  import { RESOURCE_KINDS } from '../lib/types';

  let descriptor = $state<SignedDescriptorDto | null>(null);
  let conformance = $state<ReportDto | null>(null);
  let receipts = $state<ReceiptsResponse | null>(null);
  let accounts = $state<PrepaidAccount[]>([]);
  let loading = $state(true);

  const UPTIME_SINCE = new Date('2026-06-11T08:00:00Z');
  const now = new Date('2026-07-23T14:00:00Z');
  const uptimeMs = now.getTime() - UPTIME_SINCE.getTime();
  const uptimeDays = Math.floor(uptimeMs / 86_400_000);
  const uptimeHours = Math.floor((uptimeMs % 86_400_000) / 3_600_000);

  $effect(() => {
    (async () => {
      loading = true;
      const [d, c, r, a] = await Promise.all([
        client.getDescriptor(),
        client.getConformance(),
        client.getReceipts(),
        client.getPrepaidAccounts(),
      ]);
      descriptor = d;
      conformance = c;
      receipts = r;
      accounts = a;
      loading = false;
    })();
  });

  let usageTotals = $derived.by(() => {
    const totals: Partial<Record<ResourceKind, number>> = {};
    for (const r of receipts?.receipts ?? []) {
      totals[r.kind] = (totals[r.kind] ?? 0) + r.metered_units;
    }
    return totals;
  });

  let totalBalance = $derived(accounts.reduce((sum, a) => sum + a.balance_minor, 0));
  let lowBalanceCount = $derived(accounts.filter((a) => a.balance_minor < a.low_balance_threshold_minor).length);
  let currency = $derived(accounts[0]?.currency ?? 'USD');
</script>

<div class="page">
  <header class="page-head reveal">
    <h1>Coordinator posture</h1>
    {#if descriptor}
      <span class="page-head-aside">
        <span class="kind-chip mono">{descriptor.kind}</span>
      </span>
    {/if}
    <p class="lede">The declared posture and the numbers an operator checks first.</p>
  </header>

  {#if loading || !descriptor || !conformance}
    <div class="loading-state reveal" role="status" aria-live="polite">
      <p class="loading-line">
        <span class="loading-dot" aria-hidden="true"></span>
        Reading the current signed descriptor…
      </p>

      <div class="grid-top" aria-hidden="true">
        <div class="panel skeleton-panel">
          <div class="skeleton-head">
            <span class="skeleton skeleton-kicker"></span>
            <span class="skeleton skeleton-title"></span>
          </div>
          <div class="skeleton-body">
            <span class="skeleton skeleton-badge"></span>
          </div>
        </div>
        <div class="panel skeleton-panel">
          <div class="skeleton-head">
            <span class="skeleton skeleton-kicker"></span>
            <span class="skeleton skeleton-title"></span>
          </div>
          <div class="skeleton-body">
            <div class="skeleton-strip">
              {#each Array.from({ length: 8 }) as _, i (i)}
                <span class="skeleton skeleton-light"></span>
              {/each}
            </div>
            <span class="skeleton skeleton-line" style="width: 90%"></span>
            <span class="skeleton skeleton-line" style="width: 64%"></span>
          </div>
        </div>
      </div>

      <div class="stat-grid" aria-hidden="true">
        {#each Array.from({ length: 4 }) as _, i (i)}
          <div class="panel skeleton-panel skeleton-stat">
            <span class="skeleton skeleton-stat-label"></span>
            <span class="skeleton skeleton-stat-value"></span>
            <span class="skeleton skeleton-stat-hint"></span>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="grid-top">
      <section class="panel visibility-panel reveal reveal-1">
        <!-- The "Edit descriptor →" button used to sit in this header row and
             took enough of it that the title wrapped as "Declared content-" /
             "visibility". The title is the COORD-4 clause name and is worth
             keeping verbatim, so the action moved to the card's foot instead —
             which is also where an action belongs relative to the state it
             acts on. -->
        <div class="panel-header">
          <div>
            <span class="panel-kicker">COORD-4</span>
            <h2>Declared content-visibility</h2>
          </div>
        </div>
        <div class="panel-body">
          <VisibilityBadge visibility={descriptor.visibility} />
          {#if descriptor.note}
            <p class="descriptor-note">{descriptor.note}</p>
          {/if}
          <button class="btn btn-ghost panel-action" type="button" onclick={() => router.go('descriptor')}>
            Edit descriptor →
          </button>
        </div>
      </section>

      <section class="panel conformance-panel reveal reveal-2">
        <div class="panel-header">
          <div>
            <span class="panel-kicker">COORD-1..8</span>
            <h2>Conformance</h2>
          </div>
          <span class="pill" class:pill-pass={conformance.is_conformant} class:pill-violation={!conformance.is_conformant}>
            {conformance.is_conformant ? 'No violations' : 'Violations found'}
          </span>
        </div>
        <div class="panel-body">
          <ConformanceStrip report={conformance} />
          <p class="strip-note">Amber lights are <strong>behavioral</strong> — decidable only against real traffic, not a violation. Hover a light for its clause.</p>
          {#if conformance.findings.length > 0}
            <dl class="clause-legend">
              {#each conformance.findings as f (f.id)}
                <div class="clause-row">
                  <dt>{f.id}</dt>
                  <dd>{CLAUSE_TITLE[f.id] ?? f.id} <span class="clause-ref">{f.clause}</span></dd>
                </div>
              {/each}
            </dl>
          {:else}
            <p class="clause-empty">No clause findings reported for this coordinator kind.</p>
          {/if}
        </div>
      </section>
    </div>

    <!-- Every one of these four cards used to repeat the hint "metered this
         period, all payers" under its figure. Four identical sentences say
         nothing the section label cannot say once, and they pushed the second
         row of cards off the fold. Stated once, in the label. -->
    <div class="stat-section">
      <span class="panel-kicker stat-section-label">Metered this period · all payers</span>
      <div class="stat-grid">
        {#each RESOURCE_KINDS as k, i (k)}
          <div class="reveal reveal-{i + 3}">
            <StatCard
              label={kindLabel(k)}
              value={kindQuantity(k, usageTotals[k] ?? 0).split(' ')[0]}
              unit={kindQuantity(k, usageTotals[k] ?? 0).split(' ').slice(1).join(' ')}
            />
          </div>
        {/each}
      </div>
    </div>

    <div class="stat-section">
      <span class="panel-kicker stat-section-label">Ledger &amp; identity</span>
      <div class="stat-grid">
        <!-- Was permanently bronze. A figure painted in the brand colour
             whatever it says is decoration, not signal — so the tint is now
             earned: amber only while a payer is actually below their top-up
             threshold, ink the rest of the time. That is the one number on
             this page that can demand action. -->
        <div class="reveal reveal-3">
          <StatCard
            label="Prepaid balance"
            value={ledgerMoney(totalBalance, currency)}
            accent={lowBalanceCount > 0 ? 'warn' : 'ink'}
            hint={lowBalanceCount > 0 ? `${lowBalanceCount} payer${lowBalanceCount > 1 ? 's' : ''} below top-up threshold` : 'all payers above threshold'}
          />
        </div>
        <div class="reveal reveal-4">
          <StatCard
            label="Receipts issued"
            value={integer(receipts?.receipts.length ?? 0)}
            hint="signed usage receipts on file"
          />
        </div>
        <div class="reveal reveal-5">
          <StatCard
            label="Uptime"
            value={`${uptimeDays}d ${uptimeHours}h`}
            hint="in-memory store — resets on restart"
          />
        </div>
        <div class="reveal reveal-6">
          <StatCard
            label="Operator key"
            value={descriptor.identity_hex.slice(0, 10) + '…'}
            hint="current signing identity"
          />
        </div>
      </div>
    </div>

    <!-- The fixture-data disclosure used to live here, at the very bottom of
         this one route — 1254px down, below the fold, and duplicated by a
         "Demo data" pill in the sidebar. It now sits in the sticky topbar in
         Shell.svelte, where the full sentence is on screen on every route and
         never scrolls away. Nothing was softened; it moved up. -->
  {/if}
</div>

<style>
  /* ---------- page rhythm ----------
     A two-tier gap scale, both drawn from --space-*: --space-5 between major
     regions of the page (head → top grid → each stat section), and --space-2/3
     for the tighter relationships within a region (a section's label to its
     grid). The bigger the visual distance between two things, the bigger the
     token — nothing here is an ad-hoc rem.

     The outer step was --space-6 (32px). At four stacked regions that alone
     spent ~96px of a 900px viewport on air, which is why the second card row
     sat below the fold. --space-5 keeps the regions clearly separate and buys
     the row back. .page-head and .lede now come from app.css, shared by every
     route instead of copied into six of them. */
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  /* The descriptor kind, in the page head's trailing slot. It was the panel
     kicker "Kind · reachability-adapter"; up here it identifies the whole page
     and frees the panel kicker to name the clause the panel is about. */
  .kind-chip {
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-tertiary);
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-full);
    padding: 0.16rem 0.6rem;
  }

  /* ---------- loading state ----------
     A composed skeleton of the real layout rather than a bare status line —
     the operator sees the shape of what's coming (two-panel row, then a row
     of metric cards) before the numbers land. */
  .loading-state {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .loading-line {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: 0.85rem;
  }

  .loading-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
    flex-shrink: 0;
    animation: loading-pulse calc(var(--dur-slow) * 4) var(--ease) infinite;
  }

  @keyframes loading-pulse {
    0%,
    100% {
      opacity: 0.35;
      transform: scale(0.85);
    }
    50% {
      opacity: 1;
      transform: scale(1);
    }
  }

  .skeleton {
    display: block;
    border-radius: var(--radius-xs);
    background: color-mix(in srgb, var(--bg-elevated) 65%, var(--bg-hover) 35%);
    animation: skeleton-pulse calc(var(--dur-slow) * 4) var(--ease) infinite;
  }

  @keyframes skeleton-pulse {
    0%,
    100% {
      opacity: 0.5;
    }
    50% {
      opacity: 1;
    }
  }

  .skeleton-head {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-5) var(--space-3);
    border-bottom: 1px solid var(--border-subtle);
  }

  .skeleton-kicker {
    width: 5rem;
    height: 0.6rem;
  }

  .skeleton-title {
    width: 60%;
    height: 1.05rem;
  }

  .skeleton-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-5);
  }

  .skeleton-badge {
    height: 6.4rem;
    border-radius: var(--radius-md);
  }

  .skeleton-strip {
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    gap: var(--space-2);
  }

  @media (max-width: 760px) {
    .skeleton-strip {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }

  .skeleton-light {
    height: 3.2rem;
    border-radius: var(--radius-sm);
  }

  .skeleton-line {
    height: 0.6rem;
  }

  .skeleton-stat {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-5);
  }

  .skeleton-stat-label {
    width: 45%;
    height: 0.6rem;
  }

  .skeleton-stat-value {
    width: 70%;
    height: 1.5rem;
    margin-top: var(--space-1);
  }

  .skeleton-stat-hint {
    width: 85%;
    height: 0.55rem;
    margin-top: var(--space-1);
  }

  /* ---------- top grid ---------- */
  .grid-top {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.35fr);
    gap: var(--space-4);
    align-items: stretch;
  }

  @media (max-width: 980px) {
    .grid-top {
      grid-template-columns: 1fr;
    }
  }

  /* The panel is the flex column and the body is what grows. NOT
     `.panel-body { height: 100% }`: the body's 100% resolves against the whole
     PANEL, header included, so the body ended up one header taller than the
     space it had and pushed its own last child — the "Edit descriptor →"
     action — clean out of the card and on top of the section label below it.
     Nothing measured that: the escape is vertical, so scrollWidth was 1440 and
     no element crossed the right edge. It was visible only in the screenshot. */
  .visibility-panel {
    display: flex;
    flex-direction: column;
  }

  .visibility-panel .panel-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    flex: 1;
    min-height: 0;
  }

  /* The descriptor's own note. Was a bordered, recessed .note box, which gave a
     one-sentence aside the same visual weight as the state above it; it is a
     footnote, so it is now set as one. */
  .descriptor-note {
    margin: 0;
    font-size: 0.75rem;
    line-height: 1.5;
    color: var(--text-tertiary);
    padding-left: var(--space-3);
    border-left: 2px solid var(--border-strong);
  }

  /* Actions sit at the foot of the state they act on, pushed to the bottom so
     the card's action line stays put whatever the panel height resolves to. */
  .panel-action {
    align-self: flex-start;
    margin-top: auto;
    padding-left: 0;
    padding-right: 0.6rem;
  }

  .strip-note {
    margin: var(--space-4) 0 0;
    font-size: 0.76rem;
    color: var(--text-tertiary);
    line-height: 1.5;
  }

  .clause-legend {
    margin: var(--space-4) 0 0;
    padding-top: var(--space-4);
    border-top: 1px solid var(--border-default);
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2) var(--space-5);
  }

  @media (max-width: 560px) {
    .clause-legend {
      grid-template-columns: 1fr;
    }
  }

  .clause-empty {
    margin: var(--space-4) 0 0;
    padding-top: var(--space-4);
    border-top: 1px solid var(--border-default);
    font-size: 0.78rem;
    color: var(--text-tertiary);
  }

  .clause-row {
    display: flex;
    gap: var(--space-2);
    align-items: baseline;
    font-size: 0.76rem;
    line-height: 1.4;
  }

  .clause-row dt {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 0.66rem;
    color: var(--text-tertiary);
    flex-shrink: 0;
    width: 4.4rem;
  }

  .clause-row dd {
    margin: 0;
    color: var(--text-secondary);
  }

  .clause-ref {
    font-family: var(--font-mono);
    font-size: 0.68rem;
    color: var(--text-faint);
  }

  /* ---------- metric cards ---------- */
  .stat-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .stat-section-label {
    padding-left: var(--space-1);
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-3);
  }

  @media (max-width: 760px) {
    .stat-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  /* Below ~520px a two-up grid is too narrow for a full metric — "1,065,700"
     was being ellipsised to "1,06…", which defeats the point of showing it.
     One column per row keeps every figure whole. */
  @media (max-width: 520px) {
    .stat-grid {
      grid-template-columns: 1fr;
    }
  }

</style>
