<!-- M2 Unban request queue: pending requests, newest first, each approved or denied with
     a note the viewer sees. V107-N3b A failed read says so in the panel with Retry (it was a
     toast on every open, also from the dock's menu, and then read as "No pending requests").
     V108-C36 Like whispers: popped out and wide, the list on the left and the open request
     on the right (switch by picking another); docked or narrow, the list, and a request opens
     over it with a Back button. The eye hides who banned them (`modview.hideBannedBy`). -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Card from '../../components/PanelCard.svelte';
  import type { UnbanRequest } from '../../ipc/bindings/UnbanRequest';
  import { modIpc } from '../../ipc/moderation';
  import { ago } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { errorText } from '../../lib/errors';
  import UnbanDetail from './UnbanDetail.svelte';
  import { isSplit, nextAfter } from './unban';

  let { account, login, popped = false }: { account: string; login: string; popped?: boolean } = $props();

  let list = $state<UnbanRequest[]>([]);
  let loading = $state(true);
  let busy = $state<Record<string, boolean>>({});
  let problem = $state('');
  let selected = $state<string | null>(null);
  /** Stacked: the open request shows instead of the list. */
  let open = $state(false);
  let width = $state(0);

  const split = $derived(isSplit(popped, width));
  const hideBy = $derived(settings.v.modview.hideBannedBy);
  const current = $derived(list.find((r) => r.id === selected) ?? null);

  function load() {
    loading = true;
    modIpc
      .unbanRequests(account, login)
      .then((r) => {
        list = r;
        problem = '';
        // Side by side, the first one opens so the right pane is never empty for nothing.
        if (!r.some((x) => x.id === selected)) selected = split ? (r[0]?.id ?? null) : null;
      })
      .catch((e) => (problem = errorText(e)))
      .finally(() => (loading = false));
  }
  load();

  $effect(() => {
    if (split && !selected && list.length) selected = list[0].id;
  });

  function pick(id: string) {
    selected = id;
    open = true;
  }

  async function resolve(r: UnbanRequest, approve: boolean, note: string) {
    if (busy[r.id]) return;
    busy[r.id] = true;
    try {
      await modIpc.resolveUnban(account, login, r.id, approve, note);
      const next = nextAfter(list.map((x) => x.id), r.id);
      list = list.filter((x) => x.id !== r.id);
      // Side by side the next one opens; stacked, back to the list.
      selected = split ? next : null;
      open = false;
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy[r.id] = false;
    }
  }

  const flipHide = () => void settings.patch('modview', { hideBannedBy: !hideBy });

  const asked = (iso: string) => {
    const t = Date.parse(iso);
    return Number.isNaN(t) ? '' : ago(t);
  };
</script>

<Card title="Unban requests" flush>
  {#snippet tools()}
    <IconButton
      icon={hideBy ? 'eyeOff' : 'eye'}
      label={hideBy ? 'Show who banned them' : 'Hide who banned them'}
      hint="Keeps the mod’s name off screen, e.g. on stream."
      size={14}
      active={hideBy}
      toggle
      onclick={flipHide}
    />
    <IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={load} />
  {/snippet}
  {#if problem && !list.length}
    <p class="empty">{problem} <Button small variant="link" onclick={load}>Retry</Button></p>
  {:else if !list.length}
    <p class="empty">{loading ? 'Loading…' : 'No pending requests.'}</p>
  {:else}
    <div class="unban" class:split bind:clientWidth={width}>
      <aside hidden={!split && open && !!current}>
        <ul>
          {#each list as r (r.id)}
            <li>
              <button class="req" class:on={split && selected === r.id} onclick={() => pick(r.id)}>
                <Avatar src={null} name={r.display || r.login} size={28} />
                <span class="who">
                  <span class="line1">
                    <span class="name"><bdi>{r.display || r.login}</bdi></span>
                    <span class="when">{asked(r.createdAt)}</span>
                  </span>
                  <span class="last" dir="auto">{r.text || 'No message.'}</span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </aside>
      <section hidden={!split && !(open && current)}>
        {#if !split && current}
          <button class="back" onclick={() => (open = false)}><Icon name="chevronLeft" size={14} /> Requests</button>
        {/if}
        {#if current}
          {#key current.id}
            <UnbanDetail
              {account}
              {login}
              request={current}
              {hideBy}
              busy={!!busy[current.id]}
              onresolve={(approve, note) => resolve(current, approve, note)}
            />
          {/key}
        {:else}
          <EmptyState icon="undo" text="Pick a request." />
        {/if}
      </section>
    </div>
  {/if}
</Card>

<style>
  .unban {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr;
    min-height: 0;
    min-width: 0;
  }
  .unban.split {
    grid-template-columns: minmax(200px, 280px) 1fr;
  }
  aside[hidden],
  section[hidden] {
    display: none;
  }
  aside {
    min-height: 0;
    overflow-y: auto;
  }
  .split aside {
    border-right: 1px solid var(--border);
  }
  ul {
    margin: 0;
    padding: 6px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .req {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
    transition: background-color var(--fast) ease-out;
  }
  .req:hover {
    background: var(--surface-hover);
  }
  .req.on {
    background: var(--accent-dim);
  }
  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .line1 {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    font-size: 0.84rem;
  }
  .when {
    margin-left: auto;
    flex: none;
    color: var(--dim);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .last {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--dim);
    font-size: 0.78rem;
  }
  section {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .back {
    display: flex;
    align-items: center;
    gap: 4px;
    align-self: flex-start;
    margin: 6px 8px 0;
    padding: 4px 6px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 12px;
  }
  .back:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .empty {
    margin: 0;
    padding: 12px;
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
