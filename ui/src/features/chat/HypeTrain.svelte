<!-- V108-N04 A slim bar at the top of a live chat while a hype train runs: level, the level's
     points ("1.234 / 2.000") and a bar to the next level, time left and the top giver. Kept current by the train's PubSub topic, plus
     a read every minute while a train is known and once when the chat opens, unless the app
     knows the channel is offline (the topic is followed either way: it is cheap and reports a
     train that starts later). After the train
     it says how far it got and the subs and bits given (one more read of the ended train) for
     30 s, or until its x. Subs and bits come from GQL reads only; without them it names the
     top giver. Setting "Hype train bar" (on). -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { HypeTrain } from '../../ipc/bindings/HypeTrain';
  import { hypeIpc } from '../../ipc/hypetrain';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { tabs } from '../tabs/tabs.svelte';
  import {
    closing,
    endedText,
    formatLeft,
    givenLine,
    mergeConductor,
    mergeProgress,
    progressPct,
    progressText,
    secondsLeft,
    topLine,
    trainOver,
  } from './hypeTrain';

  interface Props {
    login: string;
    /** The chat's channel id; the topic needs it. */
    channelId: string | null;
  }

  let { login, channelId }: Props = $props();

  const POLL_MS = 60_000;
  const NOTE_MS = 30_000;
  /** The closing read waits this long, so Twitch has the last gifts counted. */
  const LAST_READ_MS = 2_000;

  let train = $state<HypeTrain | null>(null);
  /** The train as it closed, while its stats show. */
  let note = $state<HypeTrain | null>(null);
  let now = $state(Date.now());
  let seq = 0;
  /** When the topic last said something, or a read last found the train. */
  let heard = 0;
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  let lastTimer: ReturnType<typeof setTimeout> | undefined;
  /** Counts closing notes, so a late read lands only on its own. */
  let noteSeq = 0;

  const on = $derived(settings.v.chat.hypeTrainBar);
  const left = $derived(train ? secondsLeft(train.expiresAt, now) : 0);
  const pct = $derived(train ? progressPct(train.progress, train.goal) : 0);
  const top = $derived(train ? topLine(train.conductors, formatNumber) : null);
  const points = $derived(train ? progressText(train.progress, train.goal, formatNumber) : null);
  const given = $derived(note ? (givenLine(note.subs, note.bits, formatNumber) ?? topLine(note.conductors, formatNumber)) : null);

  function dismiss() {
    noteSeq++;
    clearTimeout(noteTimer);
    clearTimeout(lastTimer);
    note = null;
  }

  function finish(held: HypeTrain) {
    train = null;
    dismiss();
    note = held;
    noteTimer = setTimeout(() => (note = null), NOTE_MS);
    // One more read for the final subs and bits; it lands only while this note shows.
    const mine = seq;
    const own = noteSeq;
    lastTimer = setTimeout(() => {
      void hypeIpc
        .last(login)
        .then((last) => {
          if (mine === seq && own === noteSeq && note) note = closing(note, last);
        })
        .catch(() => {});
    }, LAST_READ_MS);
  }

  async function load() {
    const mine = ++seq;
    try {
      const next = await hypeIpc.read(login);
      if (mine !== seq) return;
      if (next) {
        train = next;
        note = null;
        heard = Date.now();
      } else if (train && trainOver(train.expiresAt, Date.now(), heard)) {
        // A read can lag behind the topic: only end a train the topic can't vouch for.
        finish(train);
      }
    } catch {
      // A failed read leaves what is shown; the next message or poll tries again.
    }
  }

  $effect(() => {
    const id = channelId;
    void login;
    train = null;
    note = null;
    seq++;
    heard = 0;
    if (!on || !id) return;
    // V108-N04 No read for a channel known to be offline; one outside the tabs is read.
    const known = untrack(() => tabs.live[id]);
    if (!known || known.stream) void load();
    void hypeIpc.watch(id, true).catch(() => {});
    const off = hypeIpc.onChange((e) => {
      if (e.channelId !== id) return;
      const c = e.change;
      heard = Date.now();
      if (c.kind === 'progress') {
        const fresh = !train;
        // An older read still in flight must not land over this.
        seq++;
        train = mergeProgress(train, c.train);
        note = null;
        // A train joined midway has no givers yet: read it once.
        if (fresh) void load();
      } else if (c.kind === 'conductor') {
        if (train) train = { ...train, conductors: mergeConductor(train.conductors, c.conductor) };
      } else if (c.kind === 'end') {
        if (train) finish(train);
      } else {
        void load();
      }
    });
    const poll = setInterval(() => {
      if (train) void load();
    }, POLL_MS);
    return () => {
      clearInterval(poll);
      clearTimeout(noteTimer);
      clearTimeout(lastTimer);
      void off.then((f) => f());
      void hypeIpc.watch(id, false).catch(() => {});
    };
  });

  // The countdown ticks only while there is a train on screen.
  $effect(() => {
    if (!train) return;
    now = Date.now();
    const t = setInterval(() => {
      now = Date.now();
      if (train && train.expiresAt && secondsLeft(train.expiresAt, now) === 0) finish(train);
    }, 1000);
    return () => clearInterval(t);
  });
</script>

{#if train}
  <div class="hype" role="status" aria-label="Hype train, level {train.level}">
    <div class="row">
      <Icon name="zap" size={13} />
      <span class="level">Level {train.level}</span>
      {#if points}<span class="points">{points}</span>{/if}
      {#if top}<span class="top">{top}</span>{/if}
      <span class="time">{formatLeft(left)}</span>
    </div>
    <div class="track" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct}>
      <div class="fill" style:transform="scaleX({pct / 100})"></div>
    </div>
  </div>
{:else if note}
  <div class="hype" role="status">
    <div class="row">
      <Icon name="zap" size={13} />
      <span class="level">{endedText(note.level)}</span>
      {#if given}<span class="top">{given}</span>{/if}
      <button class="close" type="button" aria-label="Close" use:tooltip={{ label: 'Close' }} onclick={dismiss}>
        <Icon name="x" size={12} />
      </button>
    </div>
  </div>
{/if}

<style>
  .hype {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0 8px 6px;
    padding: 5px 8px 6px;
    border-left: 3px solid var(--twitch);
    background: color-mix(in srgb, var(--twitch) 8%, var(--surface));
    border-radius: var(--radius);
    color: var(--text);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .level {
    font-weight: 600;
    white-space: nowrap;
  }
  .top {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
  }
  .points {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .close {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    /* Keeps the note as slim as the bar. */
    margin: -4px -4px -4px auto;
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
  .close:active {
    opacity: 0.6;
    transition: none;
  }
  .close:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .time {
    margin-left: auto;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .track {
    height: 3px;
    border-radius: var(--radius);
    background: var(--border);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--twitch);
    /* Scaled, not resized: no layout per frame next to a playing stream. */
    transform-origin: left;
    transition: transform 0.3s ease-out;
  }
  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
</style>
