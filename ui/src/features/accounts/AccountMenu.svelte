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
  import { errorText } from '../../lib/errors';
  import { lockFor } from '../../lib/gate';
  import { toast } from '../../lib/toast.svelte';
  import { accountName, accountNeedsAttention } from './status';
  import { unlock } from '../unlock/unlock.svelte';

  // V1051-02 `active`: the 7TV view is open, so the button shows where you are.
  let {
    onmanage,
    onsubs,
    oncosmetics,
    active = false,
    streamer = 'off',
  }: {
    onmanage: () => void;
    onsubs: () => void;
    oncosmetics: () => void;
    active?: boolean;
    /** V106-17 Streamer mode dot: on, or on while your own channel is live. */
    streamer?: 'off' | 'on' | 'live';
  } = $props();

  const modeHint = $derived(
    streamer === 'live'
      ? 'Streamer mode is on. You’re live.\n'
      : streamer === 'on'
        ? 'Streamer mode is on.\n'
        : '',
  );

  const current = $derived(accounts.current);
  const attention = $derived(accounts.list.some(accountNeedsAttention));
  const walletLock = $derived(lockFor(!!current && accounts.can('wallet', current.userId), 'helix'));

  function pick(userId: string) {
    accountsIpc.setDefault(userId).catch((e) => toast.error(errorText(e, 'switch accounts')));
  }

  function wallet() {
    if (current) subsIpc.page(current.userId, 'wallet').catch((e) => toast.error(errorText(e, 'open the wallet')));
  }

  /** Asks what to unlock: subscriptions and wallet, and maybe 7TV too. */
  function ask() {
    if (current) unlock.ask('twitch', current.userId);
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
      { label: 'Subscriptions', icon: 'star', lock: walletLock, unlock: ask, run: onsubs },
      { label: 'Payment methods', icon: 'card', lock: walletLock, unlock: ask, run: wallet },
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
    class:active
    aria-label={streamer === 'off' ? 'Accounts' : 'Accounts, streamer mode on'}
    onclick={show}
    use:tooltip={{ label: current.displayName, hint: modeHint + 'Switch the default account or manage accounts.' }}
  >
    <Avatar src={current.avatar} name={current.displayName} size={22} />
    {#if attention}<span class="dot"></span>{/if}
    {#if streamer !== 'off'}<span class="mode" class:live={streamer === 'live'}></span>{/if}
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
  .trigger.active {
    background: var(--accent-dim);
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
  .mode {
    position: absolute;
    bottom: 3px;
    right: 3px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px var(--bg);
  }
  .mode.live {
    background: var(--red);
  }
</style>
