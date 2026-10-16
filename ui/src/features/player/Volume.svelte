<!-- Mute button and volume slider. `onchange` runs while dragging; `commit` is true once
     it's let go, when the tile saves it. V105-10 Live and VOD players get a Boost box: the
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
    /** V105-10 Shown when given: the boost checkbox and its state. */
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
    <label class="boost" use:tooltip={{ label: 'Boost', hint: 'Lets the volume go up to 500 %.' }}>
      <input type="checkbox" checked={boost} onchange={(e) => onboost(e.currentTarget.checked)} />
      <span class="box">{#if boost}<Icon name="check" size={10} />{/if}</span>
      Boost
    </label>
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
    border-radius: 2px;
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
  .boost {
    position: relative;
    display: none;
    align-items: center;
    gap: 4px;
    margin-right: 6px;
    color: color-mix(in srgb, var(--on-video) 85%, transparent);
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
  }
  /* Keyboard: Tab from the slider reaches the box, so it shows while either has focus. */
  .volume:hover .boost,
  .volume:has(:focus-visible) .boost {
    display: inline-flex;
  }
  .boost input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .box {
    display: grid;
    place-items: center;
    width: 12px;
    height: 12px;
    border: 1.5px solid color-mix(in srgb, var(--on-video) 80%, transparent);
    border-radius: 2px;
    color: #000;
  }
  .boost input:checked + .box {
    background: var(--on-video);
    border-color: var(--on-video);
  }
  .boost input:focus-visible + .box {
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
