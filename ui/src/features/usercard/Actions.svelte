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

  async function run(command: string, after?: () => void) {
    busy = true;
    try {
      await usercardIpc.run(channel, command, ctx());
      after?.();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }
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
  <Button
    small
    variant="ghost"
    disabled={busy}
    onclick={() => run(`/${hidden ? 'unhideuser' : 'hideuser'} ${profile.login}`)}
  >
    {hidden ? 'Show again' : 'Hide'}
  </Button>
  {#if !lookupFailed}
    <Button
      small
      variant={blocked ? 'ghost' : 'danger'}
      disabled={busy || blocked === null}
      onclick={() => run(`/${blocked ? 'unblock' : 'block'} ${profile.login}`, () => (blocked = !blocked))}
    >
      {blocked ? 'Unblock' : 'Block'}
    </Button>
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
