<!-- 1.17 Which account a tab joins on. One account: named. Several: the default, with a
     list of the others. "Anonymous" reads without sending. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import { formatNumber } from '../../lib/format';
  import { tooltip } from '../../components/tooltip.svelte';

  interface Props {
    account: string | null;
    anon: boolean;
    /** 1.34 Anonymous greyed out, e.g. for a channel going to the miner. */
    noAnon?: boolean;
  }

  let { account = $bindable(), anon = $bindable(), noAnon = false }: Props = $props();

  let open = $state(false);
  let trigger = $state<HTMLButtonElement>();
  let wasOpen = false;

  // Escape or a pick closes the list: focus goes back to the button, not the dialog or the page.
  $effect(() => {
    if (open) {
      wasOpen = true;
      return;
    }
    if (!wasOpen) return;
    wasOpen = false;
    void tick().then(() => {
      const at = document.activeElement;
      if (!at || at === document.body || at.matches('[role="dialog"], [tabindex="-1"]')) trigger?.focus();
    });
  });

  const list = $derived(accounts.list);
  const chosen = $derived(list.find((a) => a.userId === account) ?? accounts.current);

  function count(userId: string): string {
    const c = accounts.connectionsOf(userId);
    return c?.joined ? formatNumber(c.joined) : '';
  }

  function pick(userId: string) {
    account = userId;
    open = false;
  }
</script>

{#if !list.length}
  <p class="plain">Reads anonymously. Sign in to chat.</p>
{:else}
  <div class="row">
    {#if list.length === 1 && chosen}
      <span class="as" class:off={anon}>
        Joins as <Avatar src={chosen.avatar} name={chosen.displayName} size={18} />
        <b>{chosen.displayName}</b>
      </span>
    {:else if chosen}
      <div class="menu">
        <button class="trigger" bind:this={trigger} disabled={anon} onclick={() => (open = !open)} aria-haspopup="menu" aria-expanded={open}>
          <Avatar src={chosen.avatar} name={chosen.displayName} size={18} />
          <span class="name">{chosen.displayName}</span>
          {#if count(chosen.userId)}<span class="count" use:tooltip={'Chats open'}>{count(chosen.userId)}</span>{/if}
          <Icon name="chevronDown" size={14} />
        </button>
        <Popover bind:open align="start" role="menu" label="Account">
          <div class="list">
            {#each list as a (a.userId)}
              <button
                class="item"
                role="menuitemradio"
                aria-checked={a.userId === chosen.userId}
                onclick={() => pick(a.userId)}
              >
                <Avatar src={a.avatar} name={a.displayName} size={20} />
                <span class="name">{a.displayName}{#if a.isDefault}<span class="def"> default</span>{/if}</span>
                {#if count(a.userId)}<span class="count" use:tooltip={'Chats open'}>{count(a.userId)}</span>{/if}
                {#if a.userId === chosen.userId}<span class="check"><Icon name="check" size={14} /></span>{/if}
              </button>
            {/each}
          </div>
        </Popover>
      </div>
    {/if}
    <Checkbox label="Anonymous" checked={anon} disabled={noAnon} onchange={(v) => (anon = v)} />
  </div>
  {#if anon}<p class="plain">Reads chat and watches, can’t send messages.</p>{/if}
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .as {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .off {
    opacity: 0.5;
  }
  .plain {
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 0.82rem;
  }
  .menu {
    position: relative;
  }
  .trigger {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    min-width: 200px;
    padding: 0 8px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .trigger:disabled {
    opacity: 0.5;
  }
  .list {
    max-height: 240px;
    overflow-y: auto;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 32px;
    padding: 0 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
  }
  .item:hover {
    background: var(--surface-hover);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }
  .def,
  .count {
    color: var(--dim);
    font-size: 0.78rem;
  }
  .check {
    display: flex;
    color: var(--accent-text);
  }
</style>
