<!-- 1.35 What's happening on your channel: follows, subs, gifts, cheers, raids, reward
     redemptions and watch streaks, and with the sign-in with a code power-ups, hype trains
     and goals (D3), newest first. The chips filter by kind and are saved. D4 Once the
     channel's alerts play in Twitch's alert box, each alert can be played again or skipped. -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import type { ActivityFilters } from '../../ipc/bindings/ActivityFilters';
  import type { SmActivity } from '../../ipc/bindings/SmActivity';
  import { smIpc } from '../../ipc/streammanager';
  import { formatTime } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import Card from './Card.svelte';
  import { ACTIVITY_KINDS, alertAction, visibleActivity } from './logic';
  import { sm } from './streammanager.svelte';

  const filters = $derived(settings.v.streammanager.activity);
  const rows = $derived(visibleActivity(sm.activity, filters));
  const alertBox = $derived(settings.v.streammanager.alertBox);

  function flip(key: keyof ActivityFilters) {
    void settings.patch('streammanager', { activity: { [key]: !filters[key] } });
  }

  function alert(r: SmActivity, replay: boolean) {
    if (!r.alert) return;
    smIpc
      .alert(r.alert, replay)
      .then(() => toast.info(replay ? 'Alert queued again.' : 'Alert skipped.'))
      .catch((e) => toast.error(String(e)));
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
      {sm.activity.length ? 'Nothing matches the filters.' : 'New follows, subs, cheers, raids, rewards and more show up here.'}
    </p>
  {:else}
    <ul>
      {#each rows as r (r.id)}
        {@const action = alertBox ? alertAction(r.status) : null}
        <li class="kind-{r.kind}">
          <span class="t">{formatTime(new Date(r.ts))}</span>
          <span class="text">{r.text}</span>
          {#if action === 'replay'}
            <IconButton icon="refresh" label="Play the alert again" size={13} onclick={() => alert(r, true)} />
          {:else if action === 'skip'}
            <IconButton icon="chevronRight" label="Skip the alert" size={13} onclick={() => alert(r, false)} />
          {/if}
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
  li.kind-powerup,
  li.kind-hype {
    border-left-color: var(--yellow);
  }
  li.kind-goal {
    border-left-color: var(--cyan);
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
</style>
