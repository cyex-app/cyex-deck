<!-- Mute button and volume slider. `onchange` runs while dragging; `commit` is true once
     it's let go, when the tile saves it. V105-10 Live and VOD players get a Boost toggle: the
     slider then runs to 500 %. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { bindingOf } from '../../lib/hotkeys.svelte';
  import { MAX_BOOST } from './boost';

  interface Props {
    volume: number;
    muted: boolean;
    onchange: (volume: number, muted: boolean, commit: boolean) => void;
    /** V105-10 Shown when given: the Boost toggle and its state. */
    boost?: boolean;
    onboost?: (on: boolean) => void;
  }

  let { volume, muted, onchange, boost = false, onboost }: Props = $props();

  const silent = $derived(muted || volume === 0);
  const max = $derived(boost ? MAX_BOOST : 1);
  const key = $derived(bindingOf('player.mute'));
</script>

<div class="volume">
  <button
    class="ctl"
    aria-label={silent ? 'Unmute' : 'Mute'}
    use:tooltip={{ label: `${silent ? 'Unmute' : 'Mute'}${key ? ` (${key})` : ''}` }}
    onclick={() => onchange(volume || 0.5, !silent, true)}
  >
    <Icon name={silent ? 'volumeOff' : volume < 0.5 ? 'volumeLow' : 'volume'} size={18} />
  </button>
  <input
    type="range"
    min="0"
    {max}
    step="0.01"
    aria-label="Volume"
    aria-valuetext="{Math.round((silent ? 0 : volume) * 100)} %"
    class:boosted={boost}
    value={silent ? 0 : volume}
    style:--fill="{((silent ? 0 : volume) / max) * 100}%"
    oninput={(e) => onchange(Number(e.currentTarget.value), false, false)}
    onchange={(e) => onchange(Number(e.currentTarget.value), false, true)}
  />
  {#if onboost}
    <button
      class="boost"
      class:on={boost}
      role="switch"
      aria-checked={boost}
      use:tooltip={{ label: 'Boost', hint: 'Lets the volume go up to 500 %.' }}
      onclick={() => onboost(!boost)}
    >
      Boost
    </button>
    {#if boost && !silent}<span class="pct">{Math.round(volume * 100)} %</span>{/if}
  {/if}
</div>

<style>
  .volume {
    display: flex;
    align-items: center;
  }
  .volume > input[type='range'] {
    width: 0;
    height: 4px;
    margin: 0;
    opacity: 0;
    appearance: none;
    background: linear-gradient(to right, var(--on-video) var(--fill), var(--on-video-faint) var(--fill));
    border-radius: var(--radius-sm);
    transition:
      width 0.15s var(--ease),
      opacity 0.15s;
  }
  .volume:hover > input[type='range'],
  .volume:has(:focus-visible) > input[type='range'] {
    width: 72px;
    margin: 0 8px 0 2px;
    opacity: 1;
  }
  .volume > input[type='range']::-webkit-slider-thumb {
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--on-video);
  }
  /* d-9 Fades in with the slider; hidden it takes no room and no focus. */
  .boost {
    display: inline-flex;
    align-items: center;
    height: 20px;
    max-width: 0;
    margin-right: 0;
    padding: 0;
    overflow: hidden;
    background: none;
    border: 0 solid color-mix(in srgb, var(--on-video) 60%, transparent);
    border-radius: var(--radius);
    color: color-mix(in srgb, var(--on-video) 85%, transparent);
    font-size: 0.72rem;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
    opacity: 0;
    visibility: hidden;
    transition:
      max-width 0.15s var(--ease),
      margin 0.15s var(--ease),
      padding 0.15s var(--ease),
      opacity 0.15s,
      visibility 0s 0.15s;
  }
  /* Keyboard: Tab from the slider reaches the toggle, so it shows while either has focus. */
  .volume:hover .boost,
  .volume:has(:focus-visible) .boost {
    max-width: 80px;
    margin-right: 6px;
    padding: 0 9px;
    border-width: 1.5px;
    opacity: 1;
    visibility: visible;
    transition-delay: 0s;
  }
  .boost.on {
    background: var(--on-video);
    border-color: var(--on-video);
    color: #000;
  }
  .boost:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  .pct {
    margin-right: 6px;
    color: var(--on-video);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .volume:hover > input.boosted,
  .volume:has(:focus-visible) > input.boosted {
    width: 120px;
  }
  .volume > input.boosted {
    background:
      linear-gradient(to right, transparent calc(20% - 1px), color-mix(in srgb, var(--on-video) 70%, transparent) calc(20% - 1px) calc(20% + 1px), transparent calc(20% + 1px)),
      linear-gradient(to right, var(--on-video) var(--fill), var(--on-video-faint) var(--fill));
  }
</style>
