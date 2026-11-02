<!-- V108-F01 card fits the window (min-width 0). V108-C26 /mods, /vips and /roles: who holds a role and since when. A name opens its usercard.
     V108-C27 A card in its own window, like the usercard: drag it anywhere that isn't a
     control, pin it on top from the frame; unpinned it closes on a click elsewhere. Any
     chat: mods and VIPs as Twitch shares them; your own channel shows every role, and there
     + gives one and ⊖ takes one (the confirm bubble, V108-C21), as the Roles Manager does.
     Route: `roles/<channel>/<role or all>/<account>` (roleList.ts). -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import ConfirmCallout from '../../components/ConfirmCallout.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import Select from '../../components/Select.svelte';
  import SortControl from '../../components/SortControl.svelte';
  import type { ChannelRole } from '../../ipc/bindings/ChannelRole';
  import type { RoleHolder } from '../../ipc/bindings/RoleHolder';
  import type { RoleListing } from '../../ipc/bindings/RoleListing';
  import { chatIpc } from '../../ipc/chat';
  import { rolesIpc } from '../../ipc/roles';
  import { errorText } from '../../lib/errors';
  import { flip, pickKey } from '../../lib/sort';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { dragCard } from '../../lib/window.svelte';
  import { cleanLogin, ROLE_NAME, ROLES } from '../streammanager/rolesManager';
  import When from '../streammanager/When.svelte';
  import { GROUP_NAME, LIST_SORT, listTitle, parseRolesRoute, shownGroups, SORTS, total, unreadNote, type ListSort } from './roleList';

  let { route }: { route: string; popout?: boolean } = $props();

  // The route never changes for a window.
  // svelte-ignore state_referenced_locally
  const { channel, role, account } = parseRolesRoute(route);

  let box = $state<HTMLElement>();
  let listing = $state<RoleListing | null>(null);
  let error = $state('');
  let query = $state('');
  let sort = $state<ListSort>(LIST_SORT);
  /** Logins with a change running. */
  let busy = $state<Record<string, boolean>>({});
  /** The ⊖ waiting for its bubble. */
  let armed = $state<{ anchor: HTMLElement; pressX?: number; role: ChannelRole; who: RoleHolder } | null>(null);
  let adding = $state(false);
  let newLogin = $state('');
  let newRole = $state<ChannelRole>(role ?? 'mod');

  const groups = $derived(listing ? shownGroups(listing.groups, role, query, sort) : []);
  const count = $derived(listing ? total(shownGroups(listing.groups, role, '', sort)) : 0);
  const kept = $derived(listing ? unreadNote(listing.unread, role, !!account) : null);
  const none = $derived(role ? `No ${listTitle(role).toLowerCase()} yet.` : 'No roles given yet.');
  const own = $derived(!!listing?.own && !!account);
  const roleChoices = ROLES.map((r) => ({ value: r, label: ROLE_NAME[r] }));

  /** A header click sorts by its column; the same column again flips the direction. */
  function byColumn(key: ListSort['key']) {
    sort = sort.key === key ? flip(sort) : pickKey(SORTS, sort, key);
  }

  async function load() {
    try {
      listing = await rolesIpc.listing(account, channel);
      error = '';
    } catch (e) {
      error = errorText(e);
    }
  }

  onMount(() => {
    box?.focus();
    void load();
  });

  function open(who: string) {
    chatIpc.openUsercard(channel, who).catch(() => {});
  }

  /** One change through the Roles Manager's IPC (Rust picks the order and says the line),
   *  then the list again. */
  async function change(who: string, give: ChannelRole[], take: ChannelRole[]): Promise<boolean> {
    if (!account) return false;
    busy[who] = true;
    try {
      const v = await rolesIpc.set(account, channel, who, give, take);
      if (v.line) toast.success(v.line);
      return true;
    } catch (e) {
      toast.error(errorText(e));
      return false;
    } finally {
      busy[who] = false;
      void load();
    }
  }

  function arm(e: MouseEvent, r: ChannelRole, who: RoleHolder) {
    armed = { anchor: e.currentTarget as HTMLElement, pressX: e.detail === 0 ? undefined : e.clientX, role: r, who };
  }

  async function add() {
    const who = cleanLogin(newLogin);
    if (!who) return;
    if (await change(who, [newRole], [])) {
      newLogin = '';
      adding = false;
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="card" bind:this={box} tabindex="-1" role="dialog" aria-label={listTitle(role)} onmousedown={(e) => dragCard(e, '.body')}>
  <header>
    <span class="title">{listTitle(role)}{#if listing}<span class="count">{formatNumber(count)}</span>{/if}</span>
    {#if own}
      <IconButton icon="plus" label="Add" hint="Give someone a role." size={14} active={adding} toggle onclick={() => (adding = !adding)} />
    {/if}
  </header>
  {#if own && adding}
    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        void add();
      }}
    >
      <!-- svelte-ignore a11y_autofocus -->
      <input class="field" placeholder="Username" aria-label="Username" bind:value={newLogin} spellcheck="false" autocomplete="off" autofocus />
      <Select label="Role" width="128px" value={newRole} options={roleChoices} onchange={(v) => (newRole = v as ChannelRole)} />
      <Button small variant="primary" type="submit" disabled={!cleanLogin(newLogin) || busy[cleanLogin(newLogin)]}>Add</Button>
    </form>
  {/if}
  {#if listing && count > 0}
    <div class="tools">
      <SearchField bind:value={query} placeholder="Find someone" label="Find someone" onclear={() => box?.focus()} />
      <SortControl keys={SORTS} {sort} label="Sort people" onchange={(s) => (sort = s)} />
    </div>
  {/if}
  <div class="body">
    {#if error}
      <p class="note">{error}</p>
    {:else if !listing}
      <p class="note">Loading…</p>
    {:else}
      {#if count > 0}
        <div class="cols" class:own>
          {#each [['name', 'Name'], ['time', 'Since']] as const as [k, text] (k)}
            <button type="button" class="col label" class:on={sort.key === k} class:end={k === 'time'} aria-label="Sort by {text.toLowerCase()}" onclick={() => byColumn(k)}>
              {text}{#if sort.key === k}<Icon name={sort.desc ? 'arrowDown' : 'arrowUp'} size={10} />{/if}
            </button>
          {/each}
        </div>
      {/if}
      {#if kept}<p class="note">{kept}</p>{/if}
      {#if groups.length === 0 && !(role && kept)}
        <p class="note">{query ? 'No one matches.' : none}</p>
      {/if}
      {#each groups as g (g.role)}
        {#if !role}<h3>{GROUP_NAME[g.role]}<span class="count">{formatNumber(g.people.length)}</span></h3>{/if}
        {#if g.people.length === 0}<p class="note">{query ? 'No one matches.' : none}</p>{/if}
        {#each g.people as p (p.id)}
          <div class="row" class:busy={busy[p.login]}>
            <button class="who" onclick={() => open(p.login)}>
              <Avatar src={p.avatar} name={p.display} size={22} />
              <span class="name">{p.display}{#if p.display.toLowerCase() !== p.login}{' '}<span class="login">({p.login})</span>{/if}</span>
              {#if p.grantedAt}<When at={p.grantedAt} day />{/if}
            </button>
            {#if own}
              <IconButton icon="x" label="Remove {ROLE_NAME[g.role]}" size={12} disabled={busy[p.login]} onclick={(e) => arm(e, g.role, p)} />
            {/if}
          </div>
        {/each}
      {/each}
      {#if listing.truncated}<p class="note">Some people may be missing.</p>{/if}
    {/if}
  </div>
</div>
{#if armed}
  {@const a = armed}
  <ConfirmCallout
    anchor={a.anchor}
    pressX={a.pressX}
    label="Click to remove {ROLE_NAME[a.role]} from {a.who.display}"
    onconfirm={() => {
      armed = null;
      void change(a.who.login, [], [a.role]);
    }}
    ondismiss={() => {
      armed = null;
      a.anchor.focus({ preventScroll: true });
    }}
  />
{/if}

<style>
  .card {
    display: flex;
    flex-direction: column;
    flex: 1;
    width: 100%;
    min-width: 0;
    height: 100%;
    min-height: 0;
    background: var(--bg);
    outline: none;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 34px;
    padding: 2px 6px 2px 12px;
    border-bottom: 1px solid var(--border);
    font-weight: 700;
    font-size: 0.85rem;
  }
  .title,
  h3 {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }
  .count {
    color: var(--dim);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }
  .add {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px 0;
  }
  .add input {
    flex: 1;
    min-width: 0;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px 4px;
  }
  .tools > :global(.search) {
    flex: 1;
    min-width: 0;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 6px 8px;
  }
  .cols {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    justify-content: space-between;
    padding: 6px 12px 4px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .cols.own {
    padding-right: 38px;
  }
  .col {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    background: none;
    border: 0;
  }
  .col:hover,
  .col.on {
    color: var(--text);
  }
  h3 {
    margin: 8px 6px 2px;
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--muted);
  }
  .row {
    display: flex;
    align-items: center;
    border-radius: var(--radius);
  }
  .row:hover {
    background: var(--elevated-2);
  }
  .row.busy {
    opacity: 0.5;
  }
  .who {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 4px 6px;
    background: none;
    border: 0;
    color: var(--text);
    font-size: 0.85rem;
    text-align: left;
  }
  .row > :global(.icon-btn) {
    flex: none;
    width: 26px;
    height: 26px;
    color: var(--dim);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .login {
    color: var(--dim);
  }
  .note {
    margin: 8px 6px;
    color: var(--dim);
    font-size: 0.8rem;
  }
</style>
