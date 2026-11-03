<!-- V108-C35 Emotes a hype train gave the chat's account: a card over the bottom of the chat,
     above the message box (not a chat line). A click puts the emote in the message box. It
     goes after 8 s (held while the pointer or focus is on it), or with its x or Escape.
     V108-C35b One emote at a time, big and in the middle, "Hype train emote unlocked" and "<name> · yours to keep"; more
     wait their turn (the Golden Kappa first, gold, "Yours for 24 hours"). The emote is
     revealed with a small overshoot, a glow and a ring of sparkles (about 1 s, transform and
     opacity only); reduced motion shows it plainly. -->
<script lang="ts">
  import { onMount, tick } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Emote } from '../../ipc/bindings/Emote';
  import { hypeIpc } from '../../ipc/hypetrain';
  import { layers } from '../../lib/escape';
  import { focus } from '../../lib/focus.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { enqueue, isGoldenKappa, unlockLines } from './hypeLook';
  import HypeSparks from './HypeSparks.svelte';
  import { emoteLarge } from './urls';

  interface Props {
    /** The chat's account (user id); none shows nothing. */
    account: string | null;
    channelId: string | null;
    oninsert: (text: string) => void;
    /** V108-C35h The last card closed by hand: focus goes back to the message box if it was lost. */
    onclosed?: () => void;
  }

  let { account, channelId, oninsert, onclosed }: Props = $props();

  const SHOW_MS = 8_000;

  let current = $state<Emote | null>(null);
  let queue: Emote[] = [];
  let held = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const theme = $derived(settings.v.appearance.theme);
  const gold = $derived(current ? isGoldenKappa(current.id) : false);
  const lines = $derived(current ? unlockLines(current) : null);
  const img = $derived(current ? emoteLarge(current.provider, current.id, theme, { animated: current.animated, still: focus.still }) : '');

  /** The next one waiting, or none. */
  function next() {
    clearTimeout(timer);
    current = queue.shift() ?? null;
    // C35g A card removed under the pointer or focus fires no leave/focusout: drop the hold.
    if (current) arm();
    else held = false;
  }

  /** Closed by hand (x or Escape). */
  function close() {
    next();
    if (current) return;
    void tick().then(() => {
      const at = document.activeElement;
      if (!at || at === document.body) onclosed?.();
    });
  }

  function clear() {
    clearTimeout(timer);
    held = false;
    queue = [];
    current = null;
  }

  function arm() {
    clearTimeout(timer);
    // V108-C35h The emote picker (or any layer) covers the card: it waits, so the emote isn't missed.
    if (!held) timer = setTimeout(() => (layers.size ? arm() : next()), SHOW_MS);
  }

  function hold(on: boolean) {
    held = on;
    if (on) clearTimeout(timer);
    else if (current) arm();
  }

  function use(e: Emote) {
    oninsert(e.name);
  }

  onMount(() => {
    const off = hypeIpc.onUnlock((e) => {
      if (!account || e.account !== account) return;
      // A card that names no channel shows in the account's chats.
      if (e.channelId && e.channelId !== channelId) return;
      if (!e.emotes.length) return;
      queue = enqueue(current, queue, e.emotes);
      if (!current) next();
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
    clear();
  });
</script>

{#if current && lines}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions (holds the timer; Escape closes from its buttons) -->
  <div
    class="unlock"
    class:gold
    role="status"
    onpointerenter={() => hold(true)}
    onpointerleave={() => hold(false)}
    onfocusin={() => hold(true)}
    onfocusout={() => hold(false)}
    onkeydown={(e) => e.key === 'Escape' && close()}
  >
    <button class="close" type="button" aria-label="Close" use:tooltip={{ label: 'Close' }} onclick={close}>
      <Icon name="x" size={12} />
    </button>
    {#key current.id}
      <div class="stage">
        <span class="glow" aria-hidden="true"></span>
        <HypeSparks kind="ring" look={gold ? 'gold' : 'pastel'} />
        <button class="emote" type="button" aria-label="Use {current.name}" use:tooltip={{ label: current.name }} onclick={() => current && use(current)}>
          <img src={img} alt={current.name} decoding="async" />
        </button>
      </div>
      <div class="text">
        <span class="title">{lines.title}</span>
        <span class="sub">{lines.sub}</span>
      </div>
    {/key}
  </div>
{/if}

<style>
  .unlock {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 8px;
    z-index: var(--z-panel);
    width: min(240px, calc(100% - 16px));
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 12px 12px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--elevated);
    box-shadow: var(--shadow-pop);
    color: var(--text);
    text-align: center;
    animation: rise var(--base) var(--ease);
    --glow: var(--hype-glow);
  }
  .unlock.gold {
    border-color: color-mix(in srgb, var(--hype-gold) 60%, var(--border-strong));
    --glow: var(--hype-glow-gold);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .stage {
    position: relative;
    display: grid;
    place-items: center;
    width: 96px;
    /* V108-C35d Room for the 64 px emote, its padding and the overshoot. */
    height: 96px;
  }
  .glow {
    position: absolute;
    inset: 4px 10px;
    border-radius: 50%;
    background: radial-gradient(closest-side, var(--glow), transparent);
    opacity: 0.35;
    animation: glow 1.1s var(--ease) 0.08s both;
  }
  @keyframes glow {
    from {
      opacity: 0;
      transform: scale(0.5);
    }
    40% {
      opacity: 0.9;
      transform: scale(1.15);
    }
  }
  .emote {
    position: relative;
    display: grid;
    place-items: center;
    padding: 4px;
    border: 0;
    border-radius: var(--radius);
    background: none;
    cursor: pointer;
    transition: background-color var(--fast) ease-out;
    /* A small overshoot, then rest: the emote is the moment. */
    animation: reveal 0.7s linear(0, 0.38 12%, 0.82 26%, 1.08 42%, 1.1 50%, 1.03 64%, 0.99 78%, 1) 0.08s both;
  }
  @keyframes reveal {
    from {
      opacity: 0;
      transform: scale(0.6);
    }
    20% {
      opacity: 1;
    }
  }
  .emote img {
    display: block;
    height: 64px;
    max-width: 160px;
    object-fit: contain;
  }
  .emote:hover {
    background: var(--row-hover);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    max-width: 100%;
    animation: fade var(--base) ease-out 0.25s both;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .title {
    font-weight: 600;
  }
  .sub {
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .emote:active,
  .close:active {
    opacity: 0.6;
    transition: none;
  }
  .close {
    position: absolute;
    top: 4px;
    right: 4px;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
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
    .unlock,
    .glow,
    .emote,
    .text {
      animation: none;
    }
  }
</style>
