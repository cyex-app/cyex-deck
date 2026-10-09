<!-- Mute button and volume slider. `onchange` runs while dragging; `commit` is true once
     it's let go, when the tile saves it. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { bindingOf } from '../../lib/hotkeys.svelte';

  interface Props {
    volume: number;
    muted: boolean;
    onchange: (volume: number, muted: boolean, commit: boolean) => void;
  }

  let { volume, muted, onchange }: Props = $props();

  const silent = $derived(muted || volume === 0);
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
    max="1"
    step="0.01"
    aria-label="Volume"
    value={silent ? 0 : volume}
    style:--fill="{(silent ? 0 : volume) * 100}%"
    oninput={(e) => onchange(Number(e.currentTarget.value), false, false)}
    onchange={(e) => onchange(Number(e.currentTarget.value), false, true)}
  />
</div>

<style>
  .volume {
    display: flex;
    align-items: center;
  }
  input {
    width: 0;
    height: 4px;
    margin: 0;
    opacity: 0;
    appearance: none;
    background: linear-gradient(to right, #fff var(--fill), rgba(255, 255, 255, 0.3) var(--fill));
    border-radius: 2px;
    transition:
      width 0.15s var(--ease),
      opacity 0.15s;
  }
  .volume:hover input,
  input:focus-visible {
    width: 72px;
    margin: 0 8px 0 2px;
    opacity: 1;
  }
  input::-webkit-slider-thumb {
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #fff;
  }
</style>
