<!-- 1.9 The channel's pinned message, above the chat. Can be folded to one line, hidden
     into the events button while it's pinned (V103-04) or closed until the pin changes;
     mods can take it down (V101-34). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import type { Pinned } from '../../ipc/bindings/Pinned';
  import { tooltip } from '../../components/tooltip.svelte';
  import { settings } from '../../lib/settings.svelte';
  import Body from './Body.svelte';
  import { pinKey, pinLive, shows } from './eventStrip';
  import { eventStrip } from './eventStrip.svelte';
  import { shownName } from './names';

  interface Props {
    pin: Pinned;
    channel: string;
    /** Only for mods (1.57). */
    onunpin?: () => void;
  }

  let { pin, channel, onunpin }: Props = $props();

  let folded = $state(false);

  const key = $derived(pinKey(pin.id));
  const c = $derived(settings.v.chat);
  const sender = $derived(pin.message.sender);
  const name = $derived(sender ? shownName(sender, c.names, c.users[sender.id]) : null);
</script>

{#if shows(eventStrip.of(channel), key)}
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
    {#if onunpin}
      <button class="close" aria-label="Unpin" use:tooltip={{ label: 'Unpin', hint: 'Takes it down for everyone.' }} onclick={onunpin}>
        <Icon name="trash" size={13} />
      </button>
    {/if}
    {#if pinLive(pin, Date.now())}
      <button
        class="close"
        aria-label="Hide pinned message"
        use:tooltip={{ label: 'Hide', hint: 'Tucks it into the events button.' }}
        onclick={() => eventStrip.hide(channel, { key, live: true })}
      >
        <Icon name="chevronUp" size={13} />
      </button>
    {/if}
    <button
      class="close"
      aria-label="Close pinned message"
      use:tooltip={{ label: 'Close', hint: 'Only for you, until the next pin.' }}
      onclick={() => eventStrip.close(channel, key)}
    >
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
    font: var(--chat-size) / var(--chat-line) var(--chat-font);
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
