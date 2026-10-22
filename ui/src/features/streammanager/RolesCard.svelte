<!-- V107-R40 Dashboard › Community › My assigned roles: every channel where you moderate,
     lead the mods, edit or manage business, with the day you got it where Twitch says.
     "Open" shows the channel's chat; "Step down" asks once more, inline, and leaves the role. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import Card from '../../components/PanelCard.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import { windowsIpc } from '../../ipc/windows';
  import type { CmAssigned } from '../../ipc/bindings/CmAssigned';
  import { cmIpc } from '../../ipc/community';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { roomRoute } from '../chat/roomRoute';
  import { ROLE_LABEL, ROLE_LEAVE, ROLE_ORDER, type RoleFilter, roleCounts, roleKey, shownRoles } from './community';
  import SmBar from './SmBar.svelte';
  import SmProblem from './SmProblem.svelte';
  import { sm } from './streammanager.svelte';
  import When from './When.svelte';

  let list = $state<CmAssigned[]>([]);
  let loading = $state(true);
  let failed = $state('');
  /** Twitch had more than was read. */
  let truncated = $state(false);
  let query = $state('');
  let role = $state<RoleFilter>('all');
  /** Rows whose step down is running. */
  let busy = $state<Record<string, boolean>>({});
  let seq = 0;

  const counts = $derived(roleCounts(list));
  const shown = $derived(shownRoles(list, role, query));
  const filters = $derived([
    { value: 'all' as RoleFilter, label: 'All' },
    ...ROLE_ORDER.filter((r) => counts[r] > 0).map((r) => ({ value: r as RoleFilter, label: ROLE_LABEL[r] })),
  ]);
  // A filter whose roles are all gone falls back to all.
  $effect(() => {
    if (role !== 'all' && !counts[role]) role = 'all';
  });

  async function load() {
    const mine = ++seq;
    loading = true;
    try {
      const r = await cmIpc.assigned();
      if (mine !== seq) return;
      list = r.items;
      truncated = r.truncated;
      failed = '';
    } catch (e) {
      if (mine !== seq) return;
      failed = errorText(e);
      // V107-N4C The list stays up: say the refresh failed, or it looks like it worked.
      if (list.length) toast.error(failed);
    } finally {
      if (mine === seq) loading = false;
    }
  }
  void load();

  async function leave(r: CmAssigned) {
    const k = roleKey(r);
    busy[k] = true;
    try {
      await cmIpc.stepDown(r.role, r.id);
      list = list.filter((x) => roleKey(x) !== k);
      toast.success(`Stepped down in ${r.display}.`);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy[k] = false;
    }
  }

  function open(r: CmAssigned) {
    windowsIpc.openPanel(roomRoute(r.login, sm.manager?.accountId ?? null), `#${r.login}`, 380, 640).catch((e) => toast.error(errorText(e)));
  }
</script>

<Card title="My assigned roles" flush>
  {#snippet tools()}
    <IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={() => void load()} />
  {/snippet}
  {#if loading && !list.length}
    <Loading rows={5} label="Loading roles" />
  {:else if failed && !list.length}
    <SmProblem text={failed} onretry={() => void load()} />
  {:else if !list.length}
    <EmptyState icon="shield" text="You don’t hold a role in any channel." />
  {:else}
    <SmBar>
      <SearchField small bind:value={query} placeholder="Search channels" />
      <div class="filters">
        {#if filters.length > 2}<Segmented options={filters} value={role} label="Role" onchange={(v) => (role = v)} />{/if}
        <span class="count">{formatNumber(list.length)} {list.length === 1 ? 'role' : 'roles'}</span>
      </div>
    </SmBar>
    <ul>
      {#each shown as r (roleKey(r))}
        <li>
          <Avatar src={r.avatar} name={r.display} size={28} />
          <span class="who">
            <span class="name">{r.display}</span>
            <span class="meta">
              {ROLE_LABEL[r.role]}
              {#if r.since}· since <When at={r.since} day />{/if}
            </span>
          </span>
          <IconButton icon="chat" label="Open {r.display}" hint="Opens its chat." size={14} onclick={() => open(r)} />
          <span class="act">
            <ConfirmButton
              label={ROLE_LEAVE[r.role].label}
              confirm={ROLE_LEAVE[r.role].confirm}
              variant="ghost"
              disabled={busy[roleKey(r)]}
              onconfirm={() => void leave(r)}
            />
          </span>
        </li>
      {/each}
    </ul>
    {#if !shown.length}<p class="note pad">No channel matches.</p>{/if}
    <!-- V107-N4E `truncated` is also a role list that failed to load, not only "more". -->
    {#if truncated}<p class="note pad">Some roles may be missing. Refresh to try again.</p>{/if}
  {/if}
</Card>

<style>
  ul {
    margin: 0;
    padding: 4px;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    border-radius: var(--radius);
  }
  li:hover {
    background: var(--row-hover);
  }
  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name {
    font-weight: 600;
    font-size: 0.85rem;
  }
  .meta {
    color: var(--dim);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  /* One width for the button, so the icons above it line up. */
  .act {
    flex: none;
    display: flex;
    justify-content: flex-end;
    min-width: 84px;
  }
  .pad {
    padding: 4px 12px;
  }
</style>
