<!-- 1.56 AutoMod queue: messages held for review, with Allow and Deny. The list is kept
     live from EventSub; resolving drops the row. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { HeldItem } from '../../ipc/bindings/HeldItem';
  import { modIpc } from '../../ipc/moderation';
  import { toast } from '../../lib/toast.svelte';

  let { account, login, held }: { account: string; login: string; held: HeldItem[] } = $props();

  let busy = $state<Record<string, boolean>>({});

  async function resolve(item: HeldItem, allow: boolean) {
    busy[item.messageId] = true;
    try {
      await modIpc.automod(account, login, item.messageId, allow);
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy[item.messageId] = false;
    }
  }
</script>

{#if held.length === 0}
  <p class="empty">Nothing held.</p>
{:else}
  <ul>
    {#each held as h (h.messageId)}
      <li>
        <div class="who"><b>{h.display}</b> · <span class="reason">{h.reason}</span></div>
        <div class="msg">{h.text}</div>
        <div class="row">
          <Button small disabled={busy[h.messageId]} onclick={() => resolve(h, true)}>Allow</Button>
          <Button small variant="danger" disabled={busy[h.messageId]} onclick={() => resolve(h, false)}>Deny</Button>
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    margin: 0;
    padding: 6px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    padding: 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .who {
    font-size: 0.82rem;
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
  .empty {
    padding: 12px;
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
