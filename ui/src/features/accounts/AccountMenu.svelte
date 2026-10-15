<!-- Top-bar account button (1.3, 1.23): pick the default account, open the manager, your
     subscriptions (1.28) and Twitch's payment methods in the app (1.27). The shared menu
     since V101-30. The last two need Sign in with Twitch, else they're greyed out (V102-20). -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import { openMenuAt, type MenuEntry } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { accountsIpc } from '../../ipc/accounts';
  import { subsIpc } from '../../ipc/subs';
  import { accounts } from '../../lib/accounts.svelte';
  import { lockFor } from '../../lib/gate';
  import { toast } from '../../lib/toast.svelte';
  import { accountName, accountNeedsAttention } from './status';

  let { onmanage, onsubs, oncosmetics }: { onmanage: () => void; onsubs: () => void; oncosmetics: () => void } = $props();

  const current = $derived(accounts.current);
  const attention = $derived(accounts.list.some(accountNeedsAttention));
  const walletLock = $derived(lockFor(!!current && accounts.can('wallet', current.userId), 'helix'));

  function pick(userId: string) {
    accountsIpc.setDefault(userId).catch((e) => toast.error(String(e)));
  }

  function wallet() {
    if (current) subsIpc.page(current.userId, 'wallet').catch((e) => toast.error(String(e)));
  }

  function show(e: MouseEvent) {
    const items: MenuEntry[] = [
      ...accounts.list.map((a) => ({
        label: accountName(a),
        avatar: a.avatar ?? '',
        checked: a.isDefault,
        warn: accountNeedsAttention(a),
        note: a.isDefault ? 'Default account' : 'Make this the default account',
        run: () => pick(a.userId),
      })),
      'separator',
      // V102-20 Only with Twitch's own sign-in.
      { label: 'Subscriptions', icon: 'star', lock: walletLock, run: onsubs },
      { label: 'Payment methods', icon: 'card', lock: walletLock, run: wallet },
      // V1051-02 No lock: the view explains itself.
      { label: '7TV paint and badge', icon: 'seventv', run: oncosmetics },
      'separator',
      { label: 'Manage accounts', icon: 'user', run: onmanage },
    ];
    openMenuAt(e, items);
  }
</script>

{#if current}
  <button
    class="trigger"
    aria-label="Accounts"
    onclick={show}
    use:tooltip={{ label: current.displayName, hint: 'Switch the default account or manage accounts.' }}
  >
    <Avatar src={current.avatar} name={current.displayName} size={22} />
    {#if attention}<span class="dot"></span>{/if}
  </button>
{:else}
  <Button small variant="ghost" icon="login" onclick={onmanage}>Sign in</Button>
{/if}

<style>
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
  .trigger:hover {
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
</style>
