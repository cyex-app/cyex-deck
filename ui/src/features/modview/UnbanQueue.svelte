<!-- M2 Unban request queue: pending requests, newest first, each approved or denied with
     a note the viewer sees. V107-N3b A failed read says so in the panel with Retry (it was a
     toast on every open, also from the dock's menu, and then read as "No pending requests").
     V108-C36 Like whispers: popped out and wide, the list on the left and the open request
     on the right (switch by picking another); docked or narrow, the list, and a request opens
     over it with a Back button. The eye hides who banned them (`modview.hideBannedBy`).
     V108-C36b The list says how many and sorts newest or oldest first; rows read "3 days ago ·
     their message". Arrow keys move through it (side by side the request follows), Enter
     opens, Escape goes back when stacked. Confirming a request opens the next one either way. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Card from '../../components/PanelCard.svelte';
  import SortControl from '../../components/SortControl.svelte';
  import type { UnbanRequest } from '../../ipc/bindings/UnbanRequest';
  import { modIpc } from '../../ipc/moderation';
  import { ago, formatNumber } from '../../lib/format';
  import { TIME, type Sort } from '../../lib/sort';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { errorText } from '../../lib/errors';
  import UnbanDetail from './UnbanDetail.svelte';
  import { isSplit, listMove, nextAfter, orderRequests } from './unban';

  let { account, login, popped = false }: { account: string; login: string; popped?: boolean } = $props();

  let list = $state<UnbanRequest[]>([]);
  let loading = $state(true);
  let busy = $state<Record<string, boolean>>({});
  let problem = $state('');
  let selected = $state<string | null>(null);
  /** Stacked: the open request shows instead of the list. */
  let open = $state(false);
  let width = $state(0);
  let sort = $state<Sort<'time'>>({ key: 'time', desc: true });
  /** V108-C36b The request's (i), kept while switching between requests. */
  let more = $state(false);
  let rows = $state<HTMLElement>();

  const split = $derived(isSplit(popped, width));
  const hideBy = $derived(settings.v.modview.hideBannedBy);
  const ordered = $derived(orderRequests(list, sort.desc));
  const current = $derived(list.find((r) => r.id === selected) ?? null);
  /** The row that holds the list's one tab stop. */
  const focusId = $derived(current?.id ?? ordered[0]?.id ?? null);

  /** V108-C36f Counts reads, so only the latest one lands. */
  let loadSeq = 0;
  /** V108-C36f Requests confirmed here: a read that was out meanwhile must not bring them back. */
  const done = new Set<string>();

  function load() {
    loading = true;
    const mine = ++loadSeq;
    modIpc
      .unbanRequests(account, login)
      .then((all) => {
        if (mine !== loadSeq) return;
        const r = all.filter((x) => !done.has(x.id));
        list = r;
        problem = '';
        // Side by side, the first one opens so the right pane is never empty for nothing.
        if (!r.some((x) => x.id === selected)) selected = split ? (orderRequests(r, sort.desc)[0]?.id ?? null) : null;
      })
      .catch((e) => {
        if (mine !== loadSeq) return;
        // With rows showing, the panel's error line is hidden: say it as a toast.
        if (list.length) toast.error(errorText(e));
        else problem = errorText(e);
      })
      .finally(() => {
        if (mine === loadSeq) loading = false;
      });
  }
  load();

  $effect(() => {
    if (split && !selected && ordered.length) selected = ordered[0].id;
  });

  let detail = $state<HTMLElement>();

  /** V108-C36g Stacked, focus moves to the open request's Back button, so Escape goes back. */
  async function focusDetail() {
    await tick();
    detail?.querySelector<HTMLElement>('button[aria-label="Back to requests"]')?.focus();
  }

  function pick(id: string) {
    selected = id;
    open = true;
    if (!split) void focusDetail();
  }

  const rowOf = (id: string) => rows?.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`);

  async function back() {
    open = false;
    await tick();
    if (selected) rowOf(selected)?.focus();
  }

  function onListKey(e: KeyboardEvent) {
    const at = ordered.findIndex((r) => r.id === (document.activeElement as HTMLElement | null)?.dataset.id);
    const to = listMove(e.key, Math.max(at, 0), ordered.length);
    if (to === null) return;
    e.preventDefault();
    const id = ordered[to].id;
    // Side by side the request follows the keys; stacked, Enter opens it.
    selected = id;
    rowOf(id)?.focus();
  }

  async function resolve(r: UnbanRequest, approve: boolean, note: string) {
    if (busy[r.id]) return;
    busy[r.id] = true;
    try {
      await modIpc.resolveUnban(account, login, r.id, approve, note);
      const next = nextAfter(
        ordered.map((x) => x.id),
        r.id,
      );
      // V108-F57j Picked another meanwhile: that choice stays.
      const mine = selected === r.id;
      done.add(r.id);
      list = list.filter((x) => x.id !== r.id);
      if (!mine) return;
      // The next one opens, side by side or stacked; with none left the (empty) list shows.
      selected = next;
      open = !!next;
      // V108-C36g The removed request held focus: it goes to the next one.
      if (next) {
        if (split) {
          await tick();
          rowOf(next)?.focus();
        } else void focusDetail();
      }
    } catch (e) {
      const who = r.display || r.login;
      toast.error(`Couldn’t ${approve ? `unban ${who}` : `deny ${who}’s request`}: ${errorText(e)}`);
    } finally {
      busy[r.id] = false;
    }
  }

  const flipHide = () => void settings.patch('modview', { hideBannedBy: !hideBy });

  const asked = (iso: string) => {
    const t = Date.parse(iso);
    return Number.isNaN(t) ? '' : ago(t, Date.now(), true);
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
    {#if loading}<p class="empty">Loading…</p>{:else}<EmptyState icon="undo" text="No requests waiting." />{/if}
  {:else}
    <div class="unban" class:split bind:clientWidth={width}>
      <aside hidden={!split && open && !!current}>
        <div class="head">
          <span class="count">{formatNumber(list.length)} {list.length === 1 ? 'request' : 'requests'}</span>
          <SortControl keys={[TIME]} {sort} label="Sort requests" onchange={(s) => (sort = s)} />
        </div>
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions (arrow keys move between the rows' buttons) -->
        <ul aria-label="Unban requests" bind:this={rows} onkeydown={onListKey}>
          {#each ordered as r (r.id)}
            <li>
              <button
                class="req"
                class:on={split && selected === r.id}
                data-id={r.id}
                tabindex={r.id === focusId ? 0 : -1}
                aria-current={split && selected === r.id ? 'true' : undefined}
                onclick={() => pick(r.id)}
              >
                <Avatar src={r.avatar} name={r.display || r.login} size={32} />
                <span class="who">
                  <span class="name"><bdi>{r.display || r.login}</bdi></span>
                  <span class="last">
                    {#if asked(r.createdAt)}<span class="when">{asked(r.createdAt)}</span><span class="dot">·</span>{/if}
                    <span class="text" class:none={!r.text} dir="auto">{r.text || 'No message'}</span>
                  </span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </aside>
      <section bind:this={detail} hidden={!split && !(open && current)}>
        <!-- Stacked, the request mounts when opened, not on each arrow key (its reads are many). -->
        {#if current && (split || open)}
          {#key current.id}
            <UnbanDetail
              {account}
              {login}
              request={current}
              {hideBy}
              stacked={!split}
              {more}
              onmore={(v) => (more = v)}
              onback={back}
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
    /* V108-C36d minmax(0, …): a long message must not size the column past the window. */
    grid-template-columns: minmax(0, 1fr);
    min-height: 0;
    min-width: 0;
  }
  .unban.split {
    grid-template-columns: minmax(220px, 300px) minmax(0, 1fr);
  }
  aside[hidden],
  section[hidden] {
    display: none;
  }
  aside {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  .split aside {
    border-right: 1px solid var(--border);
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    height: 38px;
    padding: 0 6px 0 14px;
    border-bottom: 1px solid var(--border);
  }
  .count {
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  ul {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: 4px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .req {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
    transition: background-color var(--fast) ease-out;
  }
  .req:hover {
    background: var(--row-hover);
  }
  .req.on {
    background: var(--accent-dim);
  }
  .req:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    font-size: 0.86rem;
  }
  .last {
    display: flex;
    gap: 5px;
    min-width: 0;
    color: var(--muted);
    font-size: var(--text-hint);
    white-space: nowrap;
  }
  .when {
    flex: none;
    font-variant-numeric: tabular-nums;
  }
  .dot {
    flex: none;
    color: var(--dim);
  }
  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .text.none {
    color: var(--dim);
  }
  section {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .empty {
    margin: 0;
    padding: 12px;
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
