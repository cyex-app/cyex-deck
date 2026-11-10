<!-- V108-C35b A few sparkles for a hype train moment: a burst out of the bolt (start, level
     up), a shower along the bar (end), a ring round an unlocked emote. Transform and opacity
     through element.animate, so they run off the main thread; each is gone in about 1 s.
     The parent keys this on its celebration, so a new one replaces the last. Nothing under
     reduced motion. -->
<script lang="ts">
  import { EASE, reduced } from '../../lib/motion';
  import { sparks, type Look, type Spark, type SparkKind } from './hypeLook';

  interface Props {
    kind: SparkKind;
    look: Look;
  }

  let { kind, look }: Props = $props();

  const list = $derived(reduced() ? [] : sparks(kind, look));
  const DURATION = { burst: 700, shower: 900, ring: 800 } as const;

  function fly(node: HTMLElement, s: Spark) {
    const twinkle = kind === 'shower';
    const a = node.animate(
      [
        { opacity: 0, transform: 'translate(-50%, -50%) scale(0.3) rotate(0deg)' },
        { opacity: 1, offset: twinkle ? 0.35 : 0.2 },
        { opacity: 0, transform: `translate(calc(-50% + ${s.dx}px), calc(-50% + ${s.dy}px)) scale(${twinkle ? 0.85 : 1}) rotate(${s.star ? 90 : 0}deg)` },
      ],
      { duration: DURATION[kind], delay: s.delay, easing: EASE, fill: 'both' },
    );
    return { destroy: () => a.cancel() };
  }
</script>

<span class="sparks" aria-hidden="true">
  {#each list as s, i (i)}
    <span class="spark" class:star={s.star} style:left="{s.x}%" style:top="{s.y}%" style:width="{s.size}px" style:height="{s.size}px" style:background={s.color} use:fly={s}></span>
  {/each}
</span>

<style>
  .sparks {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .spark {
    position: absolute;
    border-radius: 50%;
    opacity: 0;
  }
  .star {
    border-radius: 0;
    clip-path: polygon(50% 0, 62% 38%, 100% 50%, 62% 62%, 50% 100%, 38% 62%, 0 50%, 38% 38%);
    /* A star needs a little more room to read. */
    scale: 1.6;
  }
</style>
