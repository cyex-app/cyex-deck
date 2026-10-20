<!-- 1.48, V107-R31 The channel's blocked terms: search, sort (newest, oldest, A–Z), add one,
     remove one with a second click. Refreshes by itself when a mod blocks or unblocks a
     term (the mod feed's live actions), and by hand. Needs Sign in with Twitch (Helix). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import GateFlag from '../../components/GateFlag.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Card from '../../components/PanelCard.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import Select from '../../components/Select.svelte';
  import type { BlockedTerm } from '../../ipc/bindings/BlockedTerm';
  import type { ModLine } from '../../ipc/bindings/ModLine';
  import { modIpc } from '../../ipc/moderation';
  import { accounts } from '../../lib/accounts.svelte';
  import { errorText } from '../../lib/errors';
  import { formatDate } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { changesTerms, shownTerms, SORTS, termProblem, type TermSort } from './terms';

  let { account, login, actions }: { account: string; login: string; actions: ModLine[] } = $props();

  const READ = 'moderator:read:blocked_terms';
  const me = $derived(accounts.list.find((a) => a.userId === account));
  const can = $derived(Boolean(me?.helix) && !me?.missingScopes.includes(READ));

  let list = $state<BlockedTerm[]>([]);
  let loading = $state(false);
  let query = $state('');
  let sort = $state<TermSort>('new');
  let draft = $state('');
  let adding = $state(false);
  let busy = $state<Record<string, boolean>>({});
  let problem = $state('');

  const rows = $derived(shownTerms(list, query, sort));

  function load() {
    if (!can) return;
    loading = true;
    modIpc
      .blockedTerms(account, login)
      .then((r) => {
        list = r;
        problem = '';
      })
      .catch((e) => (problem = errorText(e)))
      .finally(() => (loading = false));
  }

  $effect(() => {
    if (can) load();
  });

  // Live: a newer "blocked"/"unblocked" action than the last one seen reloads the list.
  let seen = 0;
  $effect(() => {
    const newest = actions.find(changesTerms);
    if (!newest || newest.ts <= seen) return;
    const first = seen === 0;
    seen = newest.ts;
    if (!first) load();
  });

  async function add() {
    const why = termProblem(draft, list);
    if (why) return toast.error(why);
    adding = true;
    try {
      const t = await modIpc.addBlockedTerm(account, login, draft.trim());
      list = [t, ...list.filter((x) => x.id !== t.id)];
      draft = '';
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      adding = false;
    }
  }

  async function remove(t: BlockedTerm) {
    busy[t.id] = true;
    try {
      await modIpc.removeBlockedTerm(account, login, t.id);
      list = list.filter((x) => x.id !== t.id);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy[t.id] = false;
    }
  }

  const until = (t: BlockedTerm) => (t.expiresAt ? `until ${formatDate(new Date(t.expiresAt))}` : '');
</script>

<Card title="Blocked terms" flush>
  {#snippet tools()}
    {#if can}<IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={load} />{/if}
  {/snippet}
  {#if !can}
    <p class="gate"><GateFlag kind="helix" /><span>Blocked terms need Sign in with Twitch.</span></p>
  {:else}
    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        void add();
      }}
    >
      <input class="field grow" placeholder="Block a term" maxlength="500" bind:value={draft} />
      <Button small type="submit" disabled={adding || !draft.trim()}>Add</Button>
    </form>
    <div class="find">
      <SearchField bind:value={query} placeholder="Search" small />
      <Select options={SORTS} value={sort} label="Sort" width="96px" onchange={(v) => (sort = v as TermSort)} />
    </div>
    {#if problem}
      <p class="empty">{problem} <Button small variant="link" onclick={load}>Retry</Button></p>
    {:else if !loading && !list.length}
      <p class="empty">No blocked terms.</p>
    {:else if !rows.length}
      <p class="empty">Nothing matches.</p>
    {:else}
      <ul>
        {#each rows as t (t.id)}
          <li>
            <span class="text">{t.text}</span>
            <span class="when">{t.createdAt ? formatDate(new Date(t.createdAt)) : ''} {until(t)}</span>
            <ConfirmButton label="Remove" confirm="Sure?" variant="ghost" disabled={busy[t.id]} onconfirm={() => void remove(t)} />
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</Card>

<style>
  .gate {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 10px;
    color: var(--muted);
    font-size: 0.78rem;
  }
  .add,
  .find {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
    padding: 8px 10px 0;
  }
  .find {
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }
  .add .field {
    flex: 1;
    min-width: 0;
  }
  .find > :global(:first-child) {
    flex: 1;
    min-width: 0;
  }
  ul {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 4px 0;
    overflow-y: auto;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 0 6px 0 10px;
  }
  li:hover {
    background: var(--surface-hover);
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 0.82rem;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .when {
    flex: none;
    color: var(--muted);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .empty {
    margin: 0;
    padding: 10px;
    color: var(--muted);
    font-size: 0.8rem;
  }
</style>
