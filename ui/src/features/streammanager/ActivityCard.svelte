<!-- 1.35 What's happening on your channel: follows, subs, gifts, cheers, raids, reward
     redemptions and watch streaks, newest first. The chips filter by kind and are saved. -->
<script lang="ts">
  import type { ActivityFilters } from '../../ipc/bindings/ActivityFilters';
  import { formatTime } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import Card from './Card.svelte';
  import { ACTIVITY_KINDS, visibleActivity } from './logic';
  import { sm } from './streammanager.svelte';

  const filters = $derived(settings.v.streammanager.activity);
  const rows = $derived(visibleActivity(sm.activity, filters));

  function flip(key: keyof ActivityFilters) {
    void settings.patch('streammanager', { activity: { [key]: !filters[key] } });
  }
</script>

<Card title="Activity" wide>
  {#snippet tools()}
    <div class="chips" role="group" aria-label="Show">
      {#each ACTIVITY_KINDS as k (k.key)}
        <button class="chip kind-{k.kind}" class:on={filters[k.key]} aria-pressed={filters[k.key]} onclick={() => flip(k.key)}>
          {k.label}
        </button>
      {/each}
    </div>
  {/snippet}
  {#if rows.length === 0}
    <p class="note">
      {sm.activity.length ? 'Nothing matches the filters.' : 'New follows, subs, gifts, cheers, raids, rewards and streaks show up here.'}
    </p>
  {:else}
    <ul>
      {#each rows as r (r.id)}
        <li class="kind-{r.kind}">
          <span class="t">{formatTime(new Date(r.ts))}</span>
          <span class="text">{r.text}</span>
        </li>
      {/each}
    </ul>
  {/if}
</Card>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
  }
  .chip {
    padding: 1px 7px;
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--dim);
    font-size: 0.72rem;
    cursor: pointer;
  }
  .chip.on {
    background: var(--bg);
    border-color: var(--border-strong);
    color: var(--text);
  }
  ul {
    margin: -4px -10px;
    padding: 0;
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
  li.kind-sub,
  li.kind-gift {
    border-left-color: var(--twitch);
  }
  li.kind-cheer {
    border-left-color: var(--yellow);
  }
  li.kind-raid {
    border-left-color: var(--red);
  }
  li.kind-redemption {
    border-left-color: var(--cyan);
  }
  li.kind-follow {
    border-left-color: var(--green);
  }
  .t {
    flex-shrink: 0;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .text {
    min-width: 0;
    overflow-wrap: anywhere;
    user-select: text;
  }
</style>
