<!-- V111-23 Latency to the streamer, small and muted, after the Live button. Reads the same
     value as the video stats (engine.stats().latency), once a second, only while the bar shows. -->
<script lang="ts">
  import { tooltip } from '../../components/tooltip.svelte';
  import { everyShown } from '../../lib/awake';
  import { latencyHigh, latencyText } from './latencyStat';

  let { read, on }: { read: () => number | null; on: boolean } = $props();

  let box = $state<HTMLElement>();
  // svelte-ignore state_referenced_locally
  let secs = $state<number | null>(read());

  $effect(() => {
    return everyShown(() => {
      // The bar is faded out when the mouse is away: nothing to refresh then.
      const shown = (box as (HTMLElement & { checkVisibility?: (o: object) => boolean }) | undefined)?.checkVisibility?.({ opacityProperty: true }) ?? true;
      if (shown && on) secs = read();
    }, 1000);
  });

  const text = $derived(on ? latencyText(secs) : '');
</script>

<span class="lat" class:high={latencyHigh(secs)} bind:this={box} use:tooltip={text ? 'Latency to the streamer' : ''}>{text}</span>

<style>
  .lat {
    flex: none;
    min-width: 6ch;
    margin-right: 4px;
    font: 500 0.7rem var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: color-mix(in srgb, var(--on-video) 75%, transparent);
    white-space: nowrap;
    text-align: left;
  }
  .high {
    color: var(--yellow);
  }
  /* Nothing but the Live button fits a very narrow bar. */
  @container bar (max-width: 339px) {
    .lat {
      display: none;
    }
  }
  @media (forced-colors: active) {
    .lat {
      color: CanvasText;
    }
  }
</style>
