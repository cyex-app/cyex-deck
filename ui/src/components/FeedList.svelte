<!-- Timed lines, newest first: the activity feeds (1.35, 1.48) and the mod actions
     (V103-15). The time (24 h, 3.2; with the date when it isn't today, 3.1), a colour by
     kind on the left, then the line's own content. -->
<script lang="ts" generics="T extends { key: string; ts: number; kind: string }">
  import type { Snippet } from 'svelte';

  import { formatDate, formatTime } from '../lib/format';

  interface Props {
    rows: T[];
    row: Snippet<[T]>;
    empty: string;
    /** Under the last line, e.g. "Load older". */
    foot?: Snippet;
  }

  let { rows, row, empty, foot }: Props = $props();

  // s2-12 A feed left open past midnight: the day moves on, and so do the labels.
  let today = $state(formatDate(new Date()));
  $effect(() => {
    const t = setInterval(() => (today = formatDate(new Date())), 60_000);
    return () => clearInterval(t);
  });

  function when(ts: number): string {
    const d = new Date(ts);
    const day = formatDate(d);
    return day === today ? formatTime(d) : `${day} ${formatTime(d)}`;
  }
</script>

{#if rows.length === 0}
  <p class="empty">{empty}</p>
{:else}
  <ul>
    {#each rows as r (r.key)}
      <li class="kind-{r.kind}">
        <span class="t">{when(r.ts)}</span>
        <span class="text">{@render row(r)}</span>
      </li>
    {/each}
  </ul>
{/if}
{#if foot}<div class="foot">{@render foot()}</div>{/if}

<style>
  ul {
    margin: 0;
    padding: 4px 0;
    list-style: none;
  }
  li {
    display: flex;
    gap: 8px;
    padding: 4px 10px;
    font-size: 0.82rem;
    border-left: 2px solid transparent;
  }
  li:nth-child(even) {
    background: var(--row-alt);
  }
  /* Activity */
  li.kind-sub,
  li.kind-gift {
    border-left-color: var(--twitch);
  }
  li.kind-cheer,
  li.kind-powerup,
  li.kind-hype,
  li.kind-suspicious {
    border-left-color: var(--yellow);
  }
  li.kind-raid {
    border-left-color: var(--red);
  }
  li.kind-redemption,
  li.kind-goal {
    border-left-color: var(--cyan);
  }
  li.kind-follow {
    border-left-color: var(--green);
  }
  /* Mod actions */
  li.kind-ban {
    border-left-color: var(--red);
  }
  li.kind-timeout,
  li.kind-warn {
    border-left-color: var(--yellow);
  }
  li.kind-delete {
    border-left-color: var(--border-strong);
  }
  li.kind-mode {
    border-left-color: var(--cyan);
  }
  li.kind-role {
    border-left-color: var(--green);
  }
  li.kind-automod {
    border-left-color: var(--twitch);
  }
  .t {
    flex-shrink: 0;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .empty {
    margin: 0;
    padding: 12px;
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px 10px;
    color: var(--muted);
    font-size: var(--text-hint);
  }
</style>
