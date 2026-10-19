<!-- 1.42 A celebration bought in the channel's chat: its emote bursts up over the
     stream for a few seconds. Off with Settings → Chat → Effects. -->
<script lang="ts">
  import { chatxIpc } from '../../ipc/chatx';
  import { settings } from '../../lib/settings.svelte';
  import { emoteLarge } from '../chat/urls';

  let { login }: { login: string } = $props();

  const COUNT = 28;
  const LIFE_MS = 4500;

  interface Piece {
    key: number;
    src: string;
    x: number;
    delay: number;
    size: number;
    drift: number;
  }
  let pieces = $state.raw<Piece[]>([]);
  let n = 0;

  $effect(() => {
    let stop: (() => void) | undefined;
    let gone = false;
    // L1 Bursts still in the air end with the player.
    const timers = new Set<ReturnType<typeof setTimeout>>();
    void chatxIpc
      .onCelebration((c) => {
        if (c.channel !== login || !settings.v.chat.effects) return;
        const src = emoteLarge('twitch', c.emoteId, settings.v.appearance.theme, { animated: true });
        const burst = Array.from({ length: COUNT }, () => ({
          key: n++,
          src,
          x: Math.random() * 100,
          delay: Math.random() * 900,
          size: 32 + Math.random() * 40,
          drift: (Math.random() - 0.5) * 120,
        }));
        pieces = [...pieces, ...burst];
        const t = setTimeout(() => {
          timers.delete(t);
          pieces = pieces.filter((p) => !burst.includes(p));
        }, LIFE_MS);
        timers.add(t);
      })
      .then((u) => (gone ? u() : (stop = u)));
    return () => {
      gone = true;
      stop?.();
      for (const t of timers) clearTimeout(t);
      if (timers.size) pieces = [];
    };
  });
</script>

{#if pieces.length}
  <div class="burst" aria-hidden="true">
    {#each pieces as p (p.key)}
      <img
        src={p.src}
        alt=""
        style:left="{p.x}%"
        style:width="{p.size}px"
        style:animation-delay="{p.delay}ms"
        style:--drift="{p.drift}px"
      />
    {/each}
  </div>
{/if}

<style>
  .burst {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    z-index: 2;
  }
  img {
    position: absolute;
    bottom: -80px;
    opacity: 0;
    animation: rise 3.4s cubic-bezier(0.2, 0.7, 0.3, 1) forwards;
  }
  @keyframes rise {
    0% {
      opacity: 0;
      transform: translate(0, 0) scale(0.6);
    }
    15% {
      opacity: 1;
    }
    80% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translate(var(--drift), -110vh) scale(1.1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    img {
      animation-duration: 0.01s;
    }
  }
</style>
