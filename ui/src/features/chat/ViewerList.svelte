<!-- 1.48, V101-35 Who's in a chat, grouped like Twitch's community tab. Anyone gets
     Twitch's list (mods, VIPs and a sample of viewers); a moderator with a Helix sign-in
     gets every viewer. Click a name for its usercard, right-click for mod actions (C36),
     with Unmod or Remove VIP for those who are one (M12), and plugins' entries (S11). Used
     in chat and the mod view. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenu, type MenuEntry } from '../../components/menu.svelte';
  import type { Chatters } from '../../ipc/bindings/Chatters';
  import { chatIpc } from '../../ipc/chat';
  import { modIpc } from '../../ipc/moderation';
  import { pluginsIpc } from '../../ipc/plugins';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { mods, userItems } from '../modview/moderation.svelte';
  import type { Standing } from '../modview/standing';
  import { pluginItems } from '../plugins/pluginMenus';
  import { plugins } from '../plugins/plugins.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  interface Props {
    account: string | null;
    login: string;
    onclose?: () => void;
  }

  let { account, login, onclose }: Props = $props();

  /** Names drawn per group; a filter finds the rest. */
  const SHOWN = 500;

  let data = $state.raw<Chatters | null>(null);
  let error = $state('');
  let loading = $state(true);
  let query = $state('');

  function load() {
    loading = true;
    error = '';
    chatIpc
      .viewers(account, login)
      .then((c) => (data = c))
      .catch((e) => (error = errorText(e)))
      .finally(() => (loading = false));
  }
  load();

  const groups = $derived.by(() => {
    if (!data) return [];
    const q = query.trim().toLowerCase();
    const pick = (list: string[]) => (q ? list.filter((n) => n.includes(q)) : list);
    const no = { mod: false, vip: false };
    return [
      { label: 'Broadcaster', list: pick(data.broadcasters), is: no },
      { label: 'Moderators', list: pick(data.moderators), is: { mod: true, vip: false } },
      { label: 'VIPs', list: pick(data.vips), is: { mod: false, vip: true } },
      { label: 'Staff', list: pick(data.staff), is: no },
      { label: 'Viewers', list: pick(data.viewers), is: no },
    ].filter((g) => g.list.length);
  });

  const listed = $derived(
    data ? data.broadcasters.length + data.moderators.length + data.vips.length + data.staff.length + data.viewers.length : 0,
  );

  function open(name: string) {
    chatIpc.openUsercard(login, name).catch((e) => toast.error(errorText(e)));
  }

  /** `/timeout name 600` through the mod path, as the usercard runs it. */
  function run(line: string) {
    if (!account) return;
    const [cmd, ...args] = line.slice(1).split(' ');
    modIpc
      .run(account, login, cmd, args.join(' '))
      .then((said) => said && toast.info(said))
      .catch((e) => toast.error(errorText(e)));
  }

  /** C36 Right-click a name: usercard and copy, plus mod actions where the account moderates,
   * then plugins' entries (S11). */
  function menu(e: MouseEvent, name: string, is: Standing) {
    e.preventDefault();
    const role = mods.role(account, login);
    const items: MenuEntry[] = [
      { label: 'Usercard', icon: 'user', run: () => open(name) },
      { label: 'Copy name', icon: 'copy', run: () => void navigator.clipboard.writeText(name) },
    ];
    if (role && name !== login) items.push('separator', ...userItems(role, name, run, () => open(name), is));
    items.push(
      'separator',
      ...pluginItems(plugins.menus, 'name', null, (p) =>
        pluginsIpc.runName(p.plugin, p.index, login, name, null, null).catch((err) => toast.error(errorText(err))),
      ),
    );
    openMenu(e, items);
  }
</script>

<div class="viewers">
  <div class="bar">
    <span class="count">{data ? `${formatNumber(Number(data.count))} in chat` : 'Viewers'}</span>
    <IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={load} />
    {#if onclose}<IconButton icon="x" label="Back to chat" size={14} onclick={onclose} />{/if}
  </div>
  <label class="search">
    <Icon name="search" size={13} />
    <input bind:value={query} placeholder="Find someone" spellcheck="false" autocomplete="off" />
  </label>
  <div class="list">
    {#if error}
      <p class="note">Couldn’t load the list: {error}</p>
    {:else if !data}
      <Loading />
    {:else if !listed}
      <p class="note">Nobody’s in chat right now.</p>
    {:else if !groups.length}
      <p class="note">Nobody by that name here.</p>
    {:else}
      {#each groups as g (g.label)}
        <div class="group">{g.label} · {formatNumber(g.list.length)}</div>
        {#each g.list.slice(0, SHOWN) as name (name)}
          <button class="name" onclick={() => open(name)} oncontextmenu={(e) => menu(e, name, g.is)}>{name}</button>
        {/each}
        {#if g.list.length > SHOWN}
          <p class="note">{formatNumber(g.list.length - SHOWN)} more. Type a name to find them.</p>
        {/if}
      {/each}
      {#if !data.complete && !query.trim()}
        <p class="note">
          Twitch lists everyone only for moderators. These are {formatNumber(listed)} of {formatNumber(Number(data.count))}.
        </p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .viewers {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 6px 4px 10px;
    font-size: 0.82rem;
  }
  .count {
    flex: 1;
    font-weight: 600;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    margin: 0 8px 6px;
    padding: 0 8px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--dim);
  }
  .search:focus-within {
    border-color: var(--accent-border);
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: none;
    border: 0;
    outline: 0;
    color: var(--text);
    font-size: 0.85rem;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 4px 8px;
  }
  .group {
    padding: 8px 6px 2px;
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 700;
  }
  .name {
    display: block;
    width: 100%;
    padding: 2px 6px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.85rem;
    text-align: left;
    content-visibility: auto;
    contain-intrinsic-size: auto 22px;
  }
  .name:hover {
    background: var(--surface-hover);
  }
  .note {
    margin: 8px 6px;
    color: var(--muted);
    font-size: 0.8rem;
  }
</style>
