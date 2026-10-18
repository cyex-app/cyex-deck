<!-- 1.56, V106-07 One message AutoMod holds: who, why, the text and Allow / Deny. A decided
     row stays, dimmed, and says who decided. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { HeldItem } from '../../ipc/bindings/HeldItem';
  import type { HeldStatus } from '../../ipc/bindings/HeldStatus';
  import { modIpc } from '../../ipc/moderation';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';

  interface Props {
    account: string;
    login: string;
    item: HeldItem;
    status?: HeldStatus;
    by?: string | null;
    /** Shown as #channel before the name, when rows come from several chats. */
    channel?: string;
  }

  let { account, login, item, status = 'pending', by = null, channel }: Props = $props();

  /** s3-11 Stays on until the status event decides the row (it unmounts the buttons), or
   * a timeout says it didn't go through. */
  const WAIT_MS = 10_000;
  let busy = $state(false);
  let wait: ReturnType<typeof setTimeout> | undefined;

  async function resolve(allow: boolean) {
    if (busy) return;
    busy = true;
    try {
      await modIpc.automod(account, login, item.messageId, allow);
      clearTimeout(wait);
      wait = setTimeout(() => {
        if (status !== 'pending') return;
        busy = false;
        toast.error('Twitch didn’t confirm that. Try again.');
      }, WAIT_MS);
    } catch (e) {
      busy = false;
      toast.error(errorText(e));
    }
  }

  $effect(() => {
    if (status !== 'pending') {
      clearTimeout(wait);
      busy = false;
    }
  });
  $effect(() => () => clearTimeout(wait));

  const done = $derived(
    status === 'allowed'
      ? by
        ? `Allowed by ${by}`
        : 'Allowed'
      : status === 'denied'
        ? by
          ? `Denied by ${by}`
          : 'Denied'
        : status === 'expired'
          ? 'Expired'
          : null,
  );
</script>

<div class="held" class:decided={status !== 'pending'}>
  <div class="who">
    {#if channel}<span class="chan">#{channel}</span>{/if}
    <b>{item.display}</b> · <span class="reason">{item.reason}</span>
  </div>
  <div class="msg">{item.text}</div>
  {#if done}
    <div class="done">{done}</div>
  {:else}
    <div class="row">
      <span class="btn"><Button small disabled={busy} onclick={() => resolve(true)}>Allow</Button></span>
      <span class="btn"><Button small variant="danger" disabled={busy} onclick={() => resolve(false)}>Deny</Button></span>
    </div>
  {/if}
</div>

<style>
  .held.decided {
    opacity: 0.6;
  }
  .who {
    font-size: 0.82rem;
  }
  .chan {
    margin-right: 4px;
    color: var(--muted);
  }
  .reason {
    color: var(--yellow);
  }
  .msg {
    margin: 4px 0 6px;
    font-size: 0.86rem;
    overflow-wrap: anywhere;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .btn :global(.btn) {
    min-width: 58px;
  }
  .done {
    color: var(--muted);
    font-size: var(--text-hint);
  }
</style>
