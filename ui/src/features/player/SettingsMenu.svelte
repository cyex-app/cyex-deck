<!-- The gear in a player's bar: quality (and audio only, P1), speed for VODs, video stats
     (P3). The choice holds for this player; Settings → Player sets where new ones start. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Level } from './engine.svelte';
  import type { Choice } from './quality';

  interface Props {
    levels: Level[];
    /** Level playing now. */
    level: number;
    choice: Choice;
    onchoose: (c: Choice) => void;
    stats: boolean;
    onstats: (on: boolean) => void;
    /** VODs and clips: playback speed. */
    speed?: number;
    onspeed?: (s: number) => void;
  }

  let { levels, level, choice, onchoose, stats, onstats, speed, onspeed }: Props = $props();

  let open = $state(false);
  const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 2];

  const playing = $derived(levels.find((l) => l.index === level));

  /** Fixed choices: the height each level caps at. */
  function choiceOf(height: number): Choice {
    const fixed = ['1080', '720', '480', '360', '160'] as const;
    return fixed.find((h) => Number(h) <= height) ?? '160';
  }

  function pick(c: Choice) {
    onchoose(c);
    open = false;
  }
</script>

<div class="wrap">
  <button class="ctl" aria-label="Settings" use:tooltip={{ label: 'Quality and more', hint: 'Quality, audio only, speed and video stats.' }} onclick={() => (open = !open)}>
    <Icon name="settings" size={17} />
  </button>
  <Popover bind:open side="above">
    <div class="menu">
      <p class="head">Quality</p>
      <button class:on={choice === 'auto'} onclick={() => pick('auto')}>
        Auto{#if choice === 'auto' && playing}<span class="note">{playing.name}</span>{/if}
      </button>
      {#each levels.filter((l) => l.height > 0) as l, i (l.index)}
        {@const c = i === 0 ? 'source' : choiceOf(l.height)}
        <button class:on={choice === c} onclick={() => pick(c)}>
          {l.name}{#if i === 0}<span class="note">Source</span>{/if}
        </button>
      {/each}
      <button class:on={choice === 'audio'} onclick={() => pick('audio')}>Audio only</button>
      {#if speed !== undefined && onspeed}
        <p class="head">Speed</p>
        <div class="speeds">
          {#each SPEEDS as s (s)}
            <button class:on={speed === s} onclick={() => onspeed(s)}>{s}×</button>
          {/each}
        </div>
      {/if}
      <div class="sep"></div>
      <button class:on={stats} onclick={() => ((open = false), onstats(!stats))}>
        <Icon name="activity" size={14} />Video stats
      </button>
    </div>
  </Popover>
</div>

<style>
  .wrap {
    position: relative;
  }
  .menu {
    display: flex;
    flex-direction: column;
    min-width: 180px;
    color: var(--text);
  }
  .head {
    margin: 6px 8px 2px;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
  }
  button:not(.ctl) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
  }
  button:not(.ctl):hover {
    background: var(--surface-hover);
  }
  button.on {
    color: var(--accent-text);
    font-weight: 600;
  }
  .note {
    margin-left: auto;
    color: var(--dim);
    font-size: 0.75rem;
    font-weight: 400;
  }
  .speeds {
    display: flex;
    flex-wrap: wrap;
  }
  .speeds button {
    padding: 5px 7px;
  }
  .sep {
    height: 1px;
    margin: 4px 0;
    background: var(--border);
  }
</style>
