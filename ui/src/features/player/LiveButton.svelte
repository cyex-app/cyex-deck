<!-- V1061-02 Live, or how far behind it; a click goes back. -->
<script lang="ts">
  import { tooltip } from '../../components/tooltip.svelte';
  import { behindText } from './liveKeys';

  interface Props {
    behind: number | null;
    atLive: boolean;
    /** An ad break is cut out. */
    ad: boolean;
    /** Q21 a The stream is over while rewound. */
    ended: boolean;
    /** rd-3 Stuck and reopening: it isn't live, whatever the play head says. */
    stalled?: boolean;
    hint: string;
    onlive: () => void;
  }

  let { behind, atLive, ad, ended, stalled = false, hint, onlive }: Props = $props();

  const live = $derived(!stalled && (atLive || ad));
  const secs = $derived(behind === null ? null : Math.round(behind));

  const plural = (n: number, w: string) => `${n} ${w}${n === 1 ? '' : 's'}`;
  /** "42 seconds", "1 minute 12 seconds", "2 hours 3 minutes". */
  function spoken(s: number): string {
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    if (h) return m ? `${plural(h, 'hour')} ${plural(m, 'minute')}` : plural(h, 'hour');
    if (m) return s % 60 ? `${plural(m, 'minute')} ${plural(s % 60, 'second')}` : plural(m, 'minute');
    return plural(s, 'second');
  }

  const label = $derived(ended ? 'Stream offline' : stalled ? 'Stalled. Go to live' : live ? 'Live' : secs === null ? 'Behind live. Go to live' : `${spoken(secs)} behind. Go to live`);
  // rd-3 Stalled: the behind look (a click goes to live), and it doesn't say "Live".
  const text = $derived(ended ? 'Offline' : stalled ? 'Stalled' : live || secs === null ? 'Live' : behindText(secs));
  const tip = $derived(ended ? '' : live ? 'Live' : hint ? `Go to live (${hint})` : 'Go to live');
</script>

<button
  class="live-btn"
  class:behind={!live && !ended}
  class:ended
  aria-label={label}
  aria-disabled={live || ended}
  use:tooltip={tip}
  onclick={() => !live && !ended && onlive()}
>
  <span class="dot" aria-hidden="true"></span>
  <span class="txt">{text}</span>
</button>

<style>
  .live-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    min-width: 52px;
    /* Between play and volume: even room on both sides, so the volume slider opening moves nothing here. */
    margin: 0 4px 0 6px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--on-video-dim);
    font-size: 0.75rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    cursor: default;
    transition:
      background-color var(--fast) var(--ease),
      color var(--fast) var(--ease),
      transform 100ms var(--ease);
  }
  .dot {
    width: 6px;
    height: 6px;
    flex: none;
    box-sizing: border-box;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--on-video) 45%, transparent);
  }
  .behind {
    background: color-mix(in srgb, var(--on-video) 14%, transparent);
    color: var(--on-video);
    cursor: pointer;
  }
  .behind:hover {
    background: color-mix(in srgb, var(--on-video) 22%, transparent);
  }
  /* d-6 Press feedback. */
  .behind:active {
    background: color-mix(in srgb, var(--on-video) 30%, transparent);
    transform: scale(0.97);
  }
  .behind .dot {
    background: transparent;
    border: 1.5px solid var(--on-video-dim);
    box-shadow: none;
  }
  .ended .dot {
    background: var(--on-video-dim);
  }
</style>
