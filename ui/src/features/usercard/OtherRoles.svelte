<!-- V111-13 Where they are mod, VIP, founder or artist in other channels, from roles.tv: one
     line of counts, each opening its list of channels with since-dates, 20 at a time. Hidden
     when they hold no role; one short line when roles.tv can't be reached. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { OtherRole } from '../../ipc/bindings/OtherRole';
  import type { RoleChannel } from '../../ipc/bindings/RoleChannel';
  import { usercardIpc } from '../../ipc/rules';
  import { appendChannels, ROLE_LABELS, roleSummary, sinceText } from './facts';

  let { userId, channel }: { userId: string; channel: string } = $props();

  let summary = $state.raw<ReturnType<typeof roleSummary> | null>(null);
  let failed = $state(false);
  let open = $state<OtherRole | null>(null);
  let list = $state.raw<RoleChannel[]>([]);
  let next = $state<string | null>(null);
  let busy = $state(false);
  let pageFailed = $state(false);
  /** Answers for a list that was closed or swapped meanwhile are dropped. */
  let asked = 0;

  // A card shows one user for its whole life.
  // svelte-ignore state_referenced_locally
  usercardIpc
    .roleCounts(userId)
    .then((c) => (summary = roleSummary(c)))
    .catch(() => (failed = true));

  function load(role: OtherRole, after: string | null) {
    const n = ++asked;
    busy = true;
    pageFailed = false;
    usercardIpc
      .rolePage(userId, role, after)
      .then((p) => {
        if (n !== asked) return;
        // V111-13 A channel can come on two pages: one row per id.
        list = after ? appendChannels(list, p.channels) : appendChannels([], p.channels);
        next = p.next;
      })
      .catch(() => n === asked && (pageFailed = true))
      .finally(() => n === asked && (busy = false));
  }

  function flip(role: OtherRole) {
    if (open === role) {
      open = null;
      asked++;
      busy = false;
      return;
    }
    open = role;
    list = [];
    next = null;
    load(role, null);
  }

  const here = (c: RoleChannel) => c.login.toLowerCase() === channel.toLowerCase();
</script>

{#if failed}
  <p class="line dim"><Icon name="shield" size={14} />Couldn’t load roles.</p>
{:else if summary?.length}
  <div class="roles">
    <p class="line">
      <span use:tooltip={'Roles in other channels, from roles.tv'}><Icon name="shield" size={14} /></span>
      <span class="chips">
        {#each summary as r (r.role)}
          <button class="chip" class:on={open === r.role} aria-expanded={open === r.role} onclick={() => flip(r.role)}
            >{r.text}<Icon name={open === r.role ? 'chevronUp' : 'chevronDown'} size={11} /></button
          >
        {/each}
      </span>
    </p>
    {#if open}
      <div class="list" aria-label={`${ROLE_LABELS[open]} in`}>
        {#each list as c (c.id)}
          <div class="row" class:ended={!c.active}>
            <span class="name">{c.name}{#if here(c)}<span class="dim"> · here</span>{/if}</span>
            <span class="since">{c.active ? sinceText(c.since) : 'ended'}</span>
          </div>
        {/each}
        {#if pageFailed}
          <p class="dim small">Couldn’t load roles.</p>
        {:else if busy}
          <p class="dim small">Loading…</p>
        {:else if next}
          <button class="more" onclick={() => open && load(open, next)}>Show more</button>
        {/if}
        <p class="dim small">From roles.tv</p>
      </div>
    {/if}
  </div>
{/if}

<style>
  .roles,
  .line {
    margin: 0;
  }
  .roles {
    padding: 0 14px 8px;
    font-size: 0.82rem;
  }
  p.line {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }
  .line.dim {
    padding: 0 14px 8px;
    font-size: 0.82rem;
  }
  .line :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    min-width: 0;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 20px;
    padding: 0 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.76rem;
  }
  .chip:hover {
    background: var(--surface-hover);
  }
  .chip.on {
    border-color: var(--border-strong);
    background: var(--surface-hover);
  }
  .list {
    display: grid;
    gap: 0;
    /* V111-18 About 8 rows, whole rows only; fewer when the window is short so the logs keep
       a few lines. The one scroll bar of the top part, and it stays inside the list. */
    max-height: 160px;
    max-height: round(down, min(160px, 18vh), 20px);
    margin: 6px 0 0 22px;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
    min-width: 0;
    height: 20px;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .since {
    flex: none;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .ended .name {
    color: var(--muted);
  }
  .dim {
    color: var(--muted);
  }
  .small {
    margin: 2px 0 0;
    font-size: 0.74rem;
  }
  .more {
    justify-self: start;
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text, var(--text));
    font-size: 0.78rem;
    text-decoration: underline;
  }
</style>
