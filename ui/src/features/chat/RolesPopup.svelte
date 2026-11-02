<!-- V108-C26 /mods, /vips and /roles: who holds a role in your channel and since when, as a
     small pop-up over the chat, just above the message box. A name opens its usercard.
     Escape (with the search empty) or a click outside closes it. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Icon from '../../components/Icon.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import SortControl from '../../components/SortControl.svelte';
  import type { ChannelRole } from '../../ipc/bindings/ChannelRole';
  import type { RoleListing } from '../../ipc/bindings/RoleListing';
  import { chatIpc } from '../../ipc/chat';
  import { rolesIpc } from '../../ipc/roles';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import When from '../streammanager/When.svelte';
  import { GROUP_NAME, LIST_SORT, listTitle, shownGroups, SORTS, total, type ListSort } from './roleList';

  interface Props {
    account: string;
    login: string;
    /** Opens on this role alone; none shows every role. */
    role: ChannelRole | null;
    onclose: () => void;
  }

  let { account, login, role, onclose }: Props = $props();

  let box = $state<HTMLElement>();
  let listing = $state<RoleListing | null>(null);
  let error = $state('');
  let query = $state('');
  let sort = $state<ListSort>(LIST_SORT);

  const groups = $derived(listing ? shownGroups(listing.groups, role, query, sort) : []);
  const count = $derived(listing ? total(shownGroups(listing.groups, role, '', sort)) : 0);
  const none = $derived(role ? `No ${listTitle(role).toLowerCase()} yet.` : 'No roles given yet.');

  onMount(() => {
    box?.focus();
    rolesIpc
      .listing(account, login)
      .then((l) => (listing = l))
      .catch((e) => (error = errorText(e)));
    const down = (e: MouseEvent) => {
      const t = e.target as Element | null;
      if (box && t && !box.contains(t) && !t.closest('.menu')) onclose();
    };
    addEventListener('mousedown', down, true);
    return () => removeEventListener('mousedown', down, true);
  });

  function open(who: string) {
    chatIpc.openUsercard(login, who).catch(() => {});
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="pop"
  bind:this={box}
  tabindex="-1"
  role="dialog"
  aria-label={listTitle(role)}
  onkeydown={(e) => e.target === box && e.key === 'Escape' && onclose()}
>
  <header>
    <span>{listTitle(role)}{#if listing} <span class="count">{formatNumber(count)}</span>{/if}</span>
    <button class="x" aria-label="Close" onclick={onclose}><Icon name="x" size={14} /></button>
  </header>
  {#if listing && count > 0}
    <div class="tools">
      <SearchField bind:value={query} placeholder="Find someone" label="Find someone" small onclear={() => box?.focus()} />
      <SortControl keys={SORTS} {sort} label="Sort people" onchange={(s) => (sort = s)} />
    </div>
  {/if}
  <div class="body">
    {#if error}
      <p class="note">{error}</p>
    {:else if !listing}
      <p class="note">Loading…</p>
    {:else if groups.length === 0}
      <p class="note">{query ? 'No one matches.' : none}</p>
    {:else}
      {#each groups as g (g.role)}
        {#if !role}<h3>{GROUP_NAME[g.role]} <span class="count">{formatNumber(g.people.length)}</span></h3>{/if}
        {#if g.people.length === 0}<p class="note">{query ? 'No one matches.' : none}</p>{/if}
        {#each g.people as p (p.id)}
          <button class="row" onclick={() => open(p.login)}>
            <Avatar src={p.avatar} name={p.display} size={22} />
            <span class="name">{p.display}{#if p.display.toLowerCase() !== p.login} <span class="login">({p.login})</span>{/if}</span>
            {#if p.grantedAt}<When at={p.grantedAt} day />{/if}
          </button>
        {/each}
      {/each}
      {#if listing.truncated}<p class="note">Some people may be missing.</p>{/if}
    {/if}
  </div>
</div>

<style>
  .pop {
    position: absolute;
    right: 8px;
    bottom: 8px;
    z-index: var(--z-panel);
    display: flex;
    flex-direction: column;
    width: min(380px, calc(100% - 16px));
    max-height: min(480px, calc(100% - 16px));
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    outline: none;
    box-shadow: var(--shadow-pop);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 6px 6px 12px;
    border-bottom: 1px solid var(--border);
    font-weight: 700;
    font-size: 0.85rem;
  }
  .count {
    color: var(--dim);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }
  .x {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .x:hover {
    color: var(--text);
  }
  .tools {
    display: flex;
    gap: 6px;
    padding: 8px 10px 4px;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 6px 8px;
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
    gap: 8px;
    width: 100%;
    padding: 4px 6px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.85rem;
    text-align: left;
  }
  .row:hover {
    background: var(--elevated-2);
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
