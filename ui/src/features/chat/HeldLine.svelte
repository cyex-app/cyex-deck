<!-- V111-17 A message AutoMod holds, in chat for the channel's mods (Rust never sends it to a
     viewer's room): who, why, the message, and Approve / Deny where this account moderates.
     The line follows Twitch's decision, whoever made it: "Approved by Kuni", "Denied", "Expired". -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { HeldNote } from '../../ipc/bindings/HeldNote';
  import { modIpc } from '../../ipc/moderation';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { HELD_WAIT_MS, heldDone } from '../feeds/heldDone';
  import { heldLabel } from '../feeds/heldReason';

  interface Props {
    held: HeldNote;
    channel: string;
    /** The account that moderates this chat; none: the state only, no buttons. */
    account: string | null;
    time: string;
    ts: number;
  }

  let { held, channel, account, time, ts }: Props = $props();

  let busy = $state(false);
  let wait: ReturnType<typeof setTimeout> | undefined;
  const done = $derived(heldDone(held.status, held.by));
  const color = $derived(held.user.color ? `--cd:${held.user.color.dark};--cl:${held.user.color.light}` : undefined);

  async function resolve(allow: boolean) {
    if (busy || !account) return;
    busy = true;
    try {
      await modIpc.automod(account, channel, held.messageId, allow);
      clearTimeout(wait);
      // The decision event redraws the line; without one in time, say so.
      wait = setTimeout(() => {
        if (held.status !== 'pending') return;
        busy = false;
        toast.error('Twitch didn’t confirm that. Try again.');
      }, HELD_WAIT_MS);
    } catch (e) {
      busy = false;
      toast.error(errorText(e));
    }
  }

  $effect(() => {
    if (held.status !== 'pending') {
      clearTimeout(wait);
      busy = false;
    }
  });
  $effect(() => () => clearTimeout(wait));
</script>

<div class="held" class:decided={held.status !== 'pending'}>
  <div class="notice-text">
    {#if time}<span class="ts" data-stamp={ts}>{time}</span>{/if}AutoMod held a message from <span
      class="who"
      class:tint={!!color}
      style={color}
      role="link"
      tabindex="-1"
      data-user={held.user.login}>{held.user.name}</span
    ><span class="reason">{' · '}{heldLabel(held.reason)}</span>
  </div>
  <div class="text" dir="auto">{held.text}</div>
  {#if done}
    <div class="done">{done}</div>
  {:else if account}
    <div class="acts">
      <Button small disabled={busy} onclick={() => resolve(true)}>Approve</Button>
      <Button small variant="danger" disabled={busy} onclick={() => resolve(false)}>Deny</Button>
    </div>
  {/if}
</div>

<style>
  .held {
    padding: 2px 0 2px 8px;
    border-left: 2px solid var(--yellow);
  }
  .held.decided {
    border-left-color: var(--border);
    opacity: 0.7;
  }
  .ts {
    margin-right: 6px;
    color: var(--dim);
    font-size: 0.85em;
    font-variant-numeric: tabular-nums;
  }
  .who {
    font-weight: 700;
    cursor: pointer;
  }
  .who.tint {
    color: var(--cd);
  }
  :global(:root[data-theme='light']) .who.tint {
    color: var(--cl);
  }
  .who:hover {
    text-decoration: underline;
  }
  .reason {
    color: var(--yellow);
  }
  .text {
    margin: 2px 0 4px;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .acts {
    display: flex;
    gap: 6px;
  }
  .acts :global(.btn) {
    min-width: 64px;
  }
  .done {
    color: var(--muted);
    font-size: var(--text-hint);
  }
</style>
