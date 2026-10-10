<!-- Top-bar account button (1.3, 1.23): pick the default account, open the manager, your
     subscriptions (1.28) and Twitch's payment methods in the app (1.27). -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { accountsIpc } from '../../ipc/accounts';
  import { subsIpc } from '../../ipc/subs';
  import { accounts } from '../../lib/accounts.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { accountName, accountNeedsAttention } from './status';

  let { onmanage, onsubs }: { onmanage: () => void; onsubs: () => void } = $props();

  let open = $state(false);

  const current = $derived(accounts.current);
  const attention = $derived(accounts.list.some(accountNeedsAttention));

  function pick(userId: string) {
    open = false;
    accountsIpc.setDefault(userId).catch((e) => toast.error(String(e)));
  }

  function manage() {
    open = false;
    onmanage();
  }

  function subs() {
    open = false;
    onsubs();
  }

  function wallet() {
    open = false;
    if (current) subsIpc.page(current.userId, 'wallet').catch((e) => toast.error(String(e)));
  }
</script>

<div class="menu">
  {#if current}
    <button
      class="trigger"
      class:open
      aria-label="Accounts"
      onclick={() => (open = !open)}
      use:tooltip={{ label: current.displayName, hint: 'Switch the default account or manage accounts.' }}
    >
      <Avatar src={current.avatar} name={current.displayName} size={22} />
      {#if attention}<span class="dot"></span>{/if}
    </button>
  {:else}
    <Button small variant="ghost" icon="login" onclick={onmanage}>Sign in</Button>
  {/if}

  <Popover bind:open>
    <div class="label head">Default account</div>
    {#each accounts.list as a (a.userId)}
      <button class="item" role="menuitemradio" aria-checked={a.isDefault} onclick={() => pick(a.userId)}>
        <Avatar src={a.avatar} name={a.displayName} size={20} />
        <span class="name">{accountName(a)}</span>
        {#if accountNeedsAttention(a)}<span class="warn"><Icon name="alert" size={14} /></span>{/if}
        {#if a.isDefault}<span class="check"><Icon name="check" size={14} /></span>{/if}
      </button>
    {/each}
    <hr />
    <button class="item" role="menuitem" onclick={subs}>
      <Icon name="star" size={15} />
      <span class="name">Subscriptions</span>
    </button>
    <button class="item" role="menuitem" onclick={wallet}>
      <Icon name="card" size={15} />
      <span class="name">Payment methods</span>
    </button>
    <hr />
    <button class="item" role="menuitem" onclick={manage}>
      <Icon name="user" size={15} />
      <span class="name">Manage accounts</span>
    </button>
  </Popover>
</div>

<style>
  .menu {
    position: relative;
  }
  .trigger {
    position: relative;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
  }
  .trigger:hover,
  .trigger.open {
    background: var(--surface-hover);
  }
  .dot {
    position: absolute;
    top: 3px;
    right: 3px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--red);
    box-shadow: 0 0 0 2px var(--bg);
  }
  .head {
    padding: 6px 8px 4px;
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
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .check {
    color: var(--accent-text);
    display: flex;
  }
  .warn {
    display: flex;
    color: var(--yellow);
  }
  hr {
    margin: 4px 0;
    border: 0;
    border-top: 1px solid var(--border);
  }
</style>
