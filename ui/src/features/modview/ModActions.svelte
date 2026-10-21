<!-- V103-15 The "Mod actions" panel of the mod view (1.48) and the stream manager (1.35):
     bans, timeouts, warnings, deleted messages, chat modes, mods and VIPs, AutoMod
     decisions and more, newest first. Each line: when, who, what (with its length), to
     whom, why. Live since it opened, plus Twitch's own history with a GQL sign-in (V102-20).
     Filters like the activity feed: kinds (saved), a moderator, a user. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Button from '../../components/Button.svelte';
  import FeedList from '../../components/FeedList.svelte';
  import FilterChips from '../../components/FilterChips.svelte';
  import GateFlag from '../../components/GateFlag.svelte';
  import Card from '../../components/PanelCard.svelte';
  import type { ModActionFilters } from '../../ipc/bindings/ModActionFilters';
  import type { ModLine } from '../../ipc/bindings/ModLine';
  import { modIpc } from '../../ipc/moderation';
  import { accounts } from '../../lib/accounts.svelte';
  import { formatDuration } from '../../lib/format';
  import { ACTION_KINDS, moderatorsIn, visibleActions } from './modActions';
  import { errorText } from '../../lib/errors';
  import Select from '../../components/Select.svelte';

  interface Props {
    items: ModLine[];
    /** Twitch has older ones. */
    older: boolean;
    filters: ModActionFilters;
    onflip: (key: keyof ModActionFilters) => void;
    /** The account that mods the channel, and the channel's login. */
    account: string | null;
    login: string;
  }

  let { items, older, filters, onflip, account, login }: Props = $props();

  let moderator = $state('');
  let user = $state('');
  let loading = $state(false);
  let failed = $state<string | null>(null);

  const gql = $derived(account !== null && Boolean(accounts.list.find((a) => a.userId === account)?.gql));
  const chips = $derived(ACTION_KINDS.map((k) => ({ key: k.key, label: k.label, on: filters[k.key] })));
  // V107-R30 What each line's stamp shows, next to the kinds.
  const stampChips = $derived([
    { key: 'date', label: 'Date', on: filters.date },
    { key: 'time', label: 'Time', on: filters.time },
  ]);
  const mods = $derived(moderatorsIn(items));
  const rows = $derived(visibleActions(items, { filters, moderator, user }).map((l) => ({ ...l, key: l.id, kind: l.group })));

  async function load(more: boolean) {
    if (!account || !gql || loading) return;
    loading = true;
    failed = null;
    try {
      await modIpc.actionsHistory(account, login, more);
    } catch (e) {
      failed = errorText(e);
    } finally {
      loading = false;
    }
  }

  // Twitch's recent ones, once per channel (Rust keeps them with the live ones).
  $effect(() => {
    if (gql && account && login) untrack(() => void load(false));
  });
</script>

<Card title="Mod actions" flush>
  <div class="bar">
    <FilterChips {chips} onflip={(k) => onflip(k as keyof ModActionFilters)} />
    <span class="stamps"><FilterChips chips={stampChips} onflip={(k) => onflip(k as keyof ModActionFilters)} /></span>
  </div>
  <div class="bar">
    <Select
      label="Moderator"
      width="40%"
      value={moderator}
      options={[{ value: '', label: 'Any mod' }, ...mods.map((m) => ({ value: m.login, label: m.name }))]}
      onchange={(v) => (moderator = v)}
    />
    <input placeholder="User" aria-label="User" bind:value={user} />
  </div>
  <FeedList {rows} date={filters.date} time={filters.time} empty={items.length ? 'Nothing matches the filters.' : 'No mod actions yet.'}>
    {#snippet row(l)}
      <span class="who">{l.moderator?.name ?? 'A mod'}</span>
      {l.action}
      {#if l.target}<span class="who">{l.target.name}</span>{/if}
      {#if l.secs}<span class="len">{formatDuration(l.secs)}</span>{/if}
      {#if l.about}<span class="about">“{l.about}”</span>{/if}
      {#if l.reason}<span class="why">{l.reason}</span>{/if}
      {#if l.from}<span class="from">in {l.from}’s chat</span>{/if}
    {/snippet}
    {#snippet foot()}
      {#if !gql}
        <GateFlag kind="gql" />
        <span>Earlier actions need a GQL sign-in.</span>
      {:else if failed}
        <span>{failed}</span>
        <Button small variant="ghost" onclick={() => void load(older)}>Try again</Button>
      {:else if older}
        <Button small variant="ghost" disabled={loading} onclick={() => void load(true)}>{loading ? 'Loading…' : 'Load older'}</Button>
      {/if}
    {/snippet}
  </FeedList>
</Card>

<style>
  .bar {
    display: flex;
    gap: 6px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }
  .bar:first-of-type {
    flex-wrap: wrap;
    align-items: center;
  }
  /* V107-R30 The stamp chips sit apart at the end, also when they wrap (a divider there
     was left alone at the start of the next row). */
  .stamps {
    display: flex;
    margin-left: auto;
  }
  .bar + .bar {
    padding-top: 0;
    border-top: 0;
  }
  .bar:has(+ .bar) {
    border-bottom: 0;
  }
  input {
    min-width: 0;
    padding: 3px 6px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.78rem;
  }
  input {
    flex: 1;
  }
  input:focus-visible {
    outline: none;
    border-color: var(--focus-ring);
    box-shadow: 0 0 0 1px var(--focus-ring);
  }
  input::placeholder {
    color: var(--dim);
  }
  .who {
    font-weight: 600;
  }
  .len {
    padding: 0 4px;
    background: var(--bg);
    border-radius: var(--radius);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  .about {
    color: var(--dim);
  }
  .why,
  .from {
    color: var(--muted);
  }
  .why::before {
    content: '· ';
  }
  .from::before {
    content: '· ';
  }
</style>
