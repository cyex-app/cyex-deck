<!-- V107-R36 Roles Manager: everyone with a role in your channel, like the dashboard's
     Community › Roles Manager. A chip per role with ⊖ to take it (asks once more, inline) and
     + to give one; last modified as "2 years ago", the date on hover; filter, sort, search;
     Add (a name, then roles). VIP slots up top: adding a VIP is off when they're all taken or
     locked. Rust runs every change and reads the lists again (`rolesIpc`). -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import { openMenu, type MenuEntry } from '../../components/menu.svelte';
  import Card from '../../components/PanelCard.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { ChannelRole } from '../../ipc/bindings/ChannelRole';
  import type { RoleMember } from '../../ipc/bindings/RoleMember';
  import type { RolesView } from '../../ipc/bindings/RolesView';
  import { rolesIpc } from '../../ipc/roles';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { relative, stamp } from './community';
  import {
    addOrder,
    blocked,
    cleanLogin,
    ROLE_HINT,
    ROLE_NAME,
    ROLES,
    roleCounts,
    type RoleFilter,
    type RoleSort,
    shownMembers,
    SORTS,
    vipCount,
  } from './rolesManager';
  import { sm } from './streammanager.svelte';

  const m = $derived(sm.manager);

  let view = $state<RolesView | null>(null);
  let loading = $state(true);
  let failed = $state('');
  let query = $state('');
  let filter = $state<RoleFilter>('all');
  let sort = $state<RoleSort>('newest');
  /** Rows with a change running, by login. */
  let busy = $state<Record<string, boolean>>({});
  /** The chip waiting for its second click: `login/role`. */
  let armed = $state('');
  let armTimer: ReturnType<typeof setTimeout> | undefined;
  let now = $state(Date.now());

  let adding = $state(false);
  let newLogin = $state('');
  let picked = $state<ChannelRole[]>([]);

  const members = $derived(view?.members ?? []);
  const counts = $derived(roleCounts(members));
  const shown = $derived(shownMembers(members, filter, query, sort));
  const vips = $derived(view?.vips ?? null);
  const vipNote = $derived(vips ? blocked('vip', [], vips) : null);

  async function load() {
    if (!m) return;
    loading = true;
    try {
      view = await rolesIpc.view(m.accountId, m.login);
      failed = '';
      now = Date.now();
    } catch (e) {
      failed = errorText(e);
    } finally {
      loading = false;
    }
  }
  $effect(() => {
    void m?.accountId;
    void load();
  });

  const holdsOf = (login: string) => members.find((x) => x.login === login)?.roles ?? [];

  /** One change; true when it went through. */
  async function change(login: string, role: ChannelRole, on: boolean): Promise<boolean> {
    if (!m) return false;
    busy[login] = true;
    try {
      view = await rolesIpc.set(m.accountId, m.login, login, role, on, holdsOf(login));
      now = Date.now();
      return true;
    } catch (e) {
      toast.error(errorText(e));
      // The lists may have moved anyway (a swap that half went through).
      void load();
      return false;
    } finally {
      busy[login] = false;
    }
  }

  async function take(r: RoleMember, role: ChannelRole) {
    disarm();
    if (await change(r.login, role, false)) toast.success(`${r.display} is no longer ${article(role)}.`);
  }

  async function give(login: string, roles: ChannelRole[]): Promise<number> {
    let done = 0;
    for (const role of addOrder(roles, holdsOf(login))) {
      if (!(await change(login, role, true))) break;
      done++;
    }
    return done;
  }

  const article = (r: ChannelRole) => `${/^[AEIOU]/.test(ROLE_NAME[r]) ? 'an' : 'a'} ${ROLE_NAME[r]}`;

  function arm(key: string) {
    clearTimeout(armTimer);
    armed = key;
    armTimer = setTimeout(() => (armed = ''), 3000);
  }

  function disarm() {
    clearTimeout(armTimer);
    armed = '';
  }
  $effect(() => () => clearTimeout(armTimer));

  function addMenu(e: MouseEvent, r: RoleMember) {
    const items: MenuEntry[] = ROLES.filter((role) => !r.roles.includes(role)).map((role) => {
      const why = blocked(role, r.roles, vips);
      return {
        label: ROLE_NAME[role],
        note: why ?? ROLE_HINT[role],
        disabled: !!why,
        run: () => void give(r.login, [role]).then((n) => n && toast.success(`Made ${r.display} ${article(role)}.`)),
      };
    });
    openMenu(e.currentTarget as HTMLElement, items, 'start');
  }

  function filterMenu(e: MouseEvent) {
    const items: MenuEntry[] = [
      { label: 'All roles', hint: String(counts.all), checked: filter === 'all', run: () => (filter = 'all') },
      'separator',
      ...ROLES.map((r) => ({ label: ROLE_NAME[r], hint: String(counts[r]), checked: filter === r, run: () => (filter = r) })),
    ];
    openMenu(e.currentTarget as HTMLElement, items, 'start');
  }

  function sortMenu(e: MouseEvent) {
    openMenu(
      e.currentTarget as HTMLElement,
      SORTS.map((s) => ({ label: s.label, checked: sort === s.value, run: () => (sort = s.value) })),
      'end',
    );
  }

  function flip(role: ChannelRole) {
    picked = picked.includes(role) ? picked.filter((r) => r !== role) : [...picked, role];
  }

  async function submitAdd() {
    const login = cleanLogin(newLogin);
    if (!login || !picked.length) return;
    const want = addOrder(picked, holdsOf(login));
    if (!want.length) {
      toast.info(`${login} has those roles already.`);
      return;
    }
    const n = await give(login, want);
    if (n === want.length) {
      toast.success(n === 1 ? `Made ${login} ${article(want[0])}.` : `Gave ${login} ${n} roles.`);
      newLogin = '';
      picked = [];
      adding = false;
    }
  }

  const newHolds = $derived(holdsOf(cleanLogin(newLogin)));
</script>

<Card title="Roles Manager" flush>
  {#snippet tools()}
    {#if vips}
      <span
        class="slots"
        class:warn={!!vipNote}
        use:tooltip={{ label: vipNote ?? (vips.allowed == null ? 'VIP slots' : `${vips.allowed - vips.used} VIP slots left`), hint: vipNote ? undefined : 'More unlock through Twitch achievements.' }}
        >{vipCount(vips)}</span
      >
    {/if}
    <IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={() => void load()} />
    <IconButton icon="plus" label="Add" hint="Give someone a role." size={14} active={adding} toggle onclick={() => (adding = !adding)} />
  {/snippet}

  {#if adding}
    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        void submitAdd();
      }}
    >
      <input class="field" placeholder="Username" bind:value={newLogin} spellcheck="false" autocomplete="off" />
      <div class="picks" role="group" aria-label="Roles to give">
        {#each ROLES as r (r)}
          {@const has = newHolds.includes(r)}
          {@const why = blocked(r, newHolds, vips)}
          <button
            type="button"
            class="pick"
            class:on={picked.includes(r) || has}
            class:off={has || !!why}
            aria-disabled={has || !!why}
            aria-pressed={picked.includes(r) || has}
            onclick={() => !has && !why && flip(r)}
            use:tooltip={{ label: ROLE_NAME[r], hint: has ? 'They have it.' : (why ?? ROLE_HINT[r]) }}>{ROLE_NAME[r]}</button
          >
        {/each}
      </div>
      <div class="row">
        <Button small variant="primary" type="submit" disabled={!cleanLogin(newLogin) || !picked.length || busy[cleanLogin(newLogin)]}>Add</Button>
        <Button small variant="ghost" onclick={() => (adding = false)}>Cancel</Button>
      </div>
    </form>
  {/if}

  {#if loading && !view}
    <Loading rows={5} label="Loading roles" />
  {:else if failed && !view}
    <div class="gap">
      <p class="bad">{failed}</p>
      <Button small onclick={() => void load()}>Retry</Button>
    </div>
  {:else if !members.length}
    <EmptyState icon="shield" text="Nobody has a role in your channel yet." />
  {:else}
    <div class="bar">
      <SearchField small bind:value={query} placeholder="Search names" />
      <div class="row">
        <button class="drop" onclick={filterMenu}>{filter === 'all' ? 'All roles' : ROLE_NAME[filter]}<Icon name="chevronDown" size={12} /></button>
        <span class="spacer"></span>
        <button class="drop" onclick={sortMenu}>{SORTS.find((s) => s.value === sort)?.label}<Icon name="chevronDown" size={12} /></button>
      </div>
    </div>
    <ul>
      {#each shown as r (r.id || r.login)}
        <li class:busy={busy[r.login]}>
          <Avatar src={r.avatar} name={r.display} size={28} />
          <div class="main">
            <div class="top">
              <span class="name">{r.display}</span>
              {#if r.display.toLowerCase() !== r.login}<span class="login">{r.login}</span>{/if}
              {#if r.modified}
                <time class="when" datetime={r.modified} use:tooltip={stamp(r.modified)}>{relative(r.modified, now)}</time>
              {/if}
            </div>
            <div class="chips">
              {#each r.roles as role (role)}
                {@const key = `${r.login}/${role}`}
                {#if armed === key}
                  <button class="chip confirm" disabled={busy[r.login]} onclick={() => void take(r, role)} onblur={disarm}>Remove {ROLE_NAME[role]}?</button>
                {:else}
                  <span class="chip">
                    {ROLE_NAME[role]}
                    <button class="rm" aria-label="Remove {ROLE_NAME[role]}" disabled={busy[r.login]} onclick={() => arm(key)}>
                      <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"
                        ><circle cx="8" cy="8" r="6.25" fill="none" stroke="currentColor" stroke-width="1.5" /><path d="M5 8h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg
                      >
                    </button>
                  </span>
                {/if}
              {/each}
              {#if r.roles.length < ROLES.length}
                <button class="chip plus" aria-label="Add a role for {r.display}" disabled={busy[r.login]} onclick={(e) => addMenu(e, r)}>
                  <Icon name="plus" size={12} />
                </button>
              {/if}
            </div>
          </div>
        </li>
      {/each}
    </ul>
    {#if !shown.length}<p class="note pad">Nobody matches.</p>{/if}
  {/if}
</Card>

<style>
  .slots {
    align-self: center;
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .slots.warn {
    color: var(--yellow);
    border-color: color-mix(in srgb, var(--yellow) 45%, transparent);
  }
  .add {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }
  .picks {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .pick {
    padding: 2px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
    font-size: 0.75rem;
    cursor: pointer;
  }
  .pick:hover:not(.off) {
    background: var(--surface-hover);
    color: var(--text);
  }
  .pick.on {
    border-color: var(--accent-border);
    background: var(--accent-dim);
    color: var(--text);
  }
  .pick.off {
    cursor: default;
    opacity: 0.55;
  }
  .bar {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .spacer {
    flex: 1;
  }
  .drop {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
    font-size: 0.75rem;
    cursor: pointer;
    white-space: nowrap;
  }
  .drop:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  ul {
    margin: 0;
    padding: 4px;
    list-style: none;
  }
  li {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 6px;
    border-radius: var(--radius);
    transition: opacity var(--fast) var(--ease);
  }
  li:hover {
    background: var(--row-hover);
  }
  li.busy {
    opacity: 0.6;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .top {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    font-size: 0.86rem;
  }
  .login {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--dim);
    font-size: 0.75rem;
  }
  .when {
    margin-left: auto;
    flex-shrink: 0;
    color: var(--dim);
    font-size: 0.72rem;
    white-space: nowrap;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 20px;
    padding: 0 3px 0 7px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--elevated);
    color: var(--text);
    font-size: 0.72rem;
    white-space: nowrap;
  }
  .rm {
    display: inline-flex;
    padding: 1px;
    border: 0;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--dim);
    cursor: pointer;
  }
  .rm:hover:not(:disabled) {
    color: var(--red);
  }
  .chip.confirm {
    padding: 0 7px;
    border-color: var(--danger-solid);
    background: var(--danger-solid);
    color: #fff;
    cursor: pointer;
  }
  .chip.plus {
    padding: 0 5px;
    background: transparent;
    border-style: dashed;
    color: var(--muted);
    cursor: pointer;
  }
  .chip.plus:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--muted);
  }
  button:disabled {
    cursor: default;
  }
  .gap {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 12px;
  }
  .bad {
    margin: 0;
    color: var(--red);
    font-size: 0.82rem;
  }
  .pad {
    padding: 4px 12px;
  }
</style>
