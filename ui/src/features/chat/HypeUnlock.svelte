<!-- V108-C35 Emotes a hype train gave the chat's account: a card over the bottom of the chat,
     above the message box (not a chat line). Their pictures, "You unlocked 3 emotes", a click
     puts one in the message box. It goes after 8 s (held while the pointer or focus is on it),
     or with its x or Escape. Comes in like a toast; plain under reduced motion. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Emote } from '../../ipc/bindings/Emote';
  import { hypeIpc } from '../../ipc/hypetrain';
  import { density } from '../../lib/density.svelte';
  import { focus } from '../../lib/focus.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { emoteRatio } from '../emotes/emotes';
  import { unlockText } from './hypeTrain';
  import { emoteImg } from './urls';

  interface Props {
    /** The chat's account (user id); none shows nothing. */
    account: string | null;
    channelId: string | null;
    oninsert: (text: string) => void;
  }

  let { account, channelId, oninsert }: Props = $props();

  const SHOW_MS = 8_000;
  /** At most this many pictures; the line still counts them all. */
  const MAX_SHOWN = 12;

  let emotes = $state<Emote[]>([]);
  let held = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const theme = $derived(settings.v.appearance.theme);

  function close() {
    clearTimeout(timer);
    emotes = [];
  }

  function arm() {
    clearTimeout(timer);
    if (!held) timer = setTimeout(close, SHOW_MS);
  }

  function hold(on: boolean) {
    held = on;
    if (on) clearTimeout(timer);
    else if (emotes.length) arm();
  }

  onMount(() => {
    const off = hypeIpc.onUnlock((e) => {
      if (!account || e.account !== account) return;
      // A card that names no channel shows in the account's chats.
      if (e.channelId && e.channelId !== channelId) return;
      if (!e.emotes.length) return;
      emotes = e.emotes;
      arm();
    });
    return () => {
      clearTimeout(timer);
      void off.then((f) => f());
    };
  });

  // Another chat in this place: the card was for the last one.
  $effect(() => {
    void channelId;
    void account;
    close();
  });
</script>

{#if emotes.length}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions (holds the timer; Escape closes from its buttons) -->
  <div
    class="unlock"
    role="status"
    onpointerenter={() => hold(true)}
    onpointerleave={() => hold(false)}
    onfocusin={() => hold(true)}
    onfocusout={() => hold(false)}
    onkeydown={(e) => e.key === 'Escape' && close()}
  >
    <div class="row">
      <Icon name="zap" size={13} />
      <span class="text">{unlockText(emotes.length)}</span>
      <button class="close" type="button" aria-label="Close" use:tooltip={{ label: 'Close' }} onclick={close}>
        <Icon name="x" size={12} />
      </button>
    </div>
    <div class="emotes">
      {#each emotes.slice(0, MAX_SHOWN) as m (m.id)}
        {@const img = emoteImg(m.provider, m.id, theme, { animated: m.animated, still: focus.still, scale: density.scale })}
        <button class="emote" type="button" aria-label="Use {m.name}" use:tooltip={{ label: m.name }} onclick={() => oninsert(m.name)}>
          <img src={img.src} alt={m.name} style:aspect-ratio={emoteRatio(m.size)} decoding="async" />
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  .unlock {
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: 8px;
    z-index: var(--z-panel);
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px 10px;
    border: 1px solid var(--border-strong);
    border-left: 3px solid var(--twitch);
    border-radius: var(--radius);
    background: var(--elevated);
    box-shadow: var(--shadow-pop);
    color: var(--text);
    animation: rise var(--base) var(--ease);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .text {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .emotes {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .emote {
    display: grid;
    place-items: center;
    height: 36px;
    min-width: 36px;
    padding: 4px;
    border: 0;
    border-radius: var(--radius);
    background: none;
    cursor: pointer;
    transition: background-color var(--fast) ease-out;
  }
  .emote img {
    height: 28px;
    max-width: 84px;
    object-fit: contain;
  }
  .emote:hover {
    background: var(--row-hover);
  }
  .emote:active,
  .close:active {
    opacity: 0.6;
    transition: none;
  }
  .close {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin: -4px -4px -4px 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--muted);
    cursor: pointer;
    transition:
      background-color var(--fast) ease-out,
      color var(--fast) ease-out;
  }
  .close:hover {
    background: var(--row-hover);
    color: var(--text);
  }
  .emote:focus-visible,
  .close:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  @media (prefers-reduced-motion: reduce) {
    .unlock {
      animation: none;
    }
  }
</style>
