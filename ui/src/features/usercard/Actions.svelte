<!-- 1.36 The usercard's buttons: whisper (1.26), your own (S1), then block (F2) and hide
     (C28). They run in the card's chat, so that chat has to be open. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { MenuContext } from '../../ipc/bindings/MenuContext';
  import type { Profile } from '../../ipc/bindings/Profile';
  import { usercardIpc } from '../../ipc/rules';
  import { accounts } from '../../lib/accounts.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { userMenus } from '../chat/menus.svelte';
  import { whispers } from '../whispers/whispers.svelte';
  import { errorText } from '../../lib/errors';
  import { tooltip } from '../../components/tooltip.svelte';
  import { withUndo } from '../chat/undo';

  let { profile, channel }: { profile: Profile; channel: string } = $props();

  /** null until the lookup answers; `lookupFailed` hides the button rather than guess. */
  let blocked = $state<boolean | null>(null);
  let lookupFailed = $state(false);
  let busy = $state(false);

  $effect(() => {
    const id = profile.id;
    let stale = false;
    blocked = null;
    lookupFailed = false;
    usercardIpc
      .blocked(id)
      .then((b) => !stale && (blocked = b))
      .catch(() => !stale && (lookupFailed = true));
    return () => (stale = true);
  });

  $effect(() => userMenus.track());

  const hidden = $derived(settings.v.highlights.hiddenUsers.some((u) => u.id === profile.id));
  const buttons = $derived(userMenus.current.usercard);

  const ctx = (): MenuContext => ({
    login: profile.login,
    userId: profile.id,
    display: profile.displayName,
    msgId: null,
    msg: null,
    msgTime: null,
  });

  async function run(command: string, after?: () => void): Promise<boolean> {
    busy = true;
    try {
      await usercardIpc.run(channel, command, ctx());
      after?.();
      return true;
    } catch (e) {
      toast.error(errorText(e));
      return false;
    } finally {
      busy = false;
    }
  }

  const login = $derived(profile.login);
  const hide = () =>
    withUndo(`${login} is hidden.`, () => run(`/hideuser ${login}`), () => run(`/unhideuser ${login}`));
  const block = () =>
    withUndo(
      `${login} is blocked.`,
      () => run(`/block ${login}`, () => (blocked = true)),
      () => run(`/unblock ${login}`, () => (blocked = false)),
    );
</script>

<div class="actions">
  {#if accounts.can('whispers') && profile.id !== accounts.current?.userId}
    <Button small icon="whisper" onclick={() => whispers.show(profile.login)}>Whisper</Button>
  {/if}
  {#each buttons as b, i (i)}
    {#if b.t === 'item'}
      <Button small disabled={busy} onclick={() => run(b.command)}>{b.label}</Button>
    {/if}
  {/each}
  <span use:tooltip={hidden ? 'Shows their messages again.' : 'Only for you, not on Twitch.'}>
    <Button
      small
      disabled={busy}
      onclick={() => (hidden ? void run(`/unhideuser ${login}`) : void hide())}
    >
      {hidden ? 'Show again' : 'Hide'}
    </Button>
  </span>
  {#if !lookupFailed}
    <span use:tooltip={blocked ? 'Lets them whisper and follow you again.' : 'Blocks them on Twitch.'}>
      <Button
        small
        variant={blocked ? 'secondary' : 'danger'}
        disabled={busy || blocked === null}
        onclick={() => (blocked ? void run(`/unblock ${login}`, () => (blocked = false)) : void block())}
      >
        {blocked ? 'Unblock' : 'Block'}
      </Button>
    </span>
  {/if}
</div>

<style>
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 14px 8px;
  }
</style>
