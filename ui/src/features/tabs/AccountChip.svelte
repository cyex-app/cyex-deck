<!-- 1.12 Which account a tile uses. Click to switch it. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Icon from '../../components/Icon.svelte';
  import { openMenu, type MenuEntry } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { accounts } from '../../lib/accounts.svelte';
  import IdentityDialog from '../chat/IdentityDialog.svelte';
  import { tabs } from './tabs.svelte';

  let { tile }: { tile: Tile } = $props();

  let identity = $state(false);

  const account = $derived(tile.account ? accounts.list.find((a) => a.userId === tile.account) : undefined);
  const label = $derived(
    account
      ? `Chatting as ${account.displayName}`
      : tile.account
        ? 'That account is gone, so this reads anonymously'
        : 'Reading anonymously',
  );

  function set(userId: string | null) {
    if (userId === tile.account) return;
    void tabs.apply({ op: 'edit', tile: tile.id, channel: tile.channel, show: tile.show, account: userId });
  }

  function open(e: MouseEvent) {
    const items: MenuEntry[] = accounts.list.map((a) => ({
      label: a.displayName,
      icon: 'user',
      checked: a.userId === tile.account,
      run: () => set(a.userId),
    }));
    items.push('separator', { label: 'Anonymous', icon: 'eyeOff', checked: !account, run: () => set(null) });
    if (account && (accounts.can('badgePicker', account.userId) || accounts.can('chatColor', account.userId))) {
      items.push('separator', { label: 'Badge and color…', icon: 'badge', run: () => (identity = true) });
    }
    openMenu(e.currentTarget as HTMLElement, items);
  }
</script>

{#if accounts.list.length || tile.account}
  <button
    class="chip"
    aria-label={label}
    onclick={open}
    onpointerdown={(e) => e.stopPropagation()}
    use:tooltip={{ label, hint: 'Switch the account for this channel.' }}
  >
    {#if account}
      <Avatar src={account.avatar} name={account.displayName} size={18} />
    {:else}
      <Icon name="eyeOff" size={15} />
    {/if}
  </button>
{/if}

{#if identity && account}
  <IdentityDialog account={account.userId} login={tile.channel.login} onclose={() => (identity = false)} />
{/if}

<style>
  .chip {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .chip:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
