<!-- 1.9 The channel's pinned message, above the chat. Can be folded to one line or hidden
     until the pin changes. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import type { Pinned } from '../../ipc/bindings/Pinned';
  import { settings } from '../../lib/settings.svelte';
  import Body from './Body.svelte';
  import { shownName } from './names';

  let { pin, channel }: { pin: Pinned; channel: string } = $props();

  let folded = $state(false);
  let hiddenId = $state<string | null>(null);

  const c = $derived(settings.v.chat);
  const sender = $derived(pin.message.sender);
  const name = $derived(sender ? shownName(sender, c.names, c.users[sender.id]) : null);
</script>

{#if hiddenId !== pin.id}
  <div class="pin" class:paid={pin.paid} class:folded>
    <Icon name="pin" size={14} />
    <button class="text" onclick={() => (folded = !folded)} aria-expanded={!folded}>
      {#if sender && name}
        <span
          class="name"
          style="--cd:{sender.color.dark};--cl:{sender.color.light}">{name.name}</span
        >:
      {/if}
      <Body fragments={pin.message.body} {channel} />
      {#if pin.by && !folded}<span class="by">Pinned by {pin.by}</span>{/if}
    </button>
    <button class="close" aria-label="Hide pinned message" onclick={() => (hiddenId = pin.id)}>
      <Icon name="x" size={13} />
    </button>
  </div>
{/if}

<style>
  .pin {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin: 6px 8px 2px;
    padding: 7px 8px 7px 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
  }
  .pin.paid {
    border-color: var(--twitch);
  }
  .pin > :global(svg) {
    flex: none;
    margin-top: 3px;
  }
  .text {
    flex: 1;
    min-width: 0;
    padding: 0;
    background: none;
    border: 0;
    color: var(--text);
    text-align: left;
    font: var(--chat-size) / 1.5 var(--chat-font);
    overflow-wrap: anywhere;
  }
  .folded .text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name {
    color: var(--cd);
    font-weight: 700;
  }
  :global(:root[data-theme='light']) .name {
    color: var(--cl);
  }
  .by {
    display: block;
    margin-top: 2px;
    color: var(--dim);
    font-size: 0.8em;
  }
  .close {
    display: grid;
    place-items: center;
    flex: none;
    width: 22px;
    height: 22px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .close:hover {
    color: var(--text);
  }
</style>
