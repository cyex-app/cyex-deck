<!-- V108-N04 A slim bar at the top of a live chat while a hype train runs: level, progress to
     the next level, time left and the top giver. Kept current by the train's PubSub topic, plus
     a read every minute while a train is known and once when the chat opens, unless the app
     knows the channel is offline (the topic is followed either way: it is cheap and reports a
     train that starts later). After the train
     it says how far it got and the subs and bits given (one more read of the ended train) for
     30 s, or until its x. Subs and bits come from GQL reads only; without them it names the
     top giver. Setting "Hype train bar" (on).
     V108-C35 A short celebration when a train starts, levels up and ends: the bar's tint flashes
     and its bolt pops (opacity and transform only, about 0,9 s; a new one cuts the last one
     short). Reduced motion: the tint just shows for a moment. Reads never celebrate.
     V108-C35b Progress is a percentage ("45 %": subs and bits both count) and the bar's own
     background fills with it, a washed pastel half rainbow (gold for a Golden Kappa train,
     with its Kappa for the bolt). A 2 px line along the bottom drains with the level's time
     (element.animate, linear). The fill sweeps by animating its clip-path. Celebrations: the start sweeps the fill in, a sheen crosses
     and sparkles burst from the bolt; a level up sheens, pops the level and bursts; the end
     (V108-C35h no sheen) keeps where the last level got to (V108-C35c) and lets a few sparkles rise along it. 0,7 to 1,3 s, transform, opacity and
     the fill's clip only; reduced motion keeps the tint flash and drops the rest.
     V109-C02 The end is bigger (hypeTrain.ts burstParts): the fill sweeps in to where the train got
     to, the sheen crosses, the result text pops and a dozen confetti bits rain inside the bar
     (clipped to it); about 1,3 s, composite properties and the fill's clip only. Text is
     --text on the washed fill: at least 5,2:1 in both themes. -->
<script lang="ts">
  import { tick, untrack } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { HypeTrain } from '../../ipc/bindings/HypeTrain';
  import { hypeIpc } from '../../ipc/hypetrain';
  import { everyShown } from '../../lib/awake';
  import { density } from '../../lib/density.svelte';
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
    burstOf,
    burstParts,
    type Burst,
    secondsLeft,
    topLine,
    topParts,
    trainOver,
  } from './hypeTrain';
  import { GOLDEN_KAPPA_ID, gradient, levelSpan, lookOf, msLeft, pctText, timerFraction } from './hypeLook';
  import HypeSparks from './HypeSparks.svelte';
  import { emoteImg } from './urls';

  interface Props {
    login: string;
    /** The chat's channel id; the topic needs it. */
    channelId: string | null;
    /** V108-C35 The chat's account (user id): its hype train rewards are followed too. */
    account?: string | null;
    /** V108-C35h Closing the note by hand hands focus back (to the message box) if it was on the note. */
    onclosed?: () => void;
  }

  let { login, channelId, account = null, onclosed }: Props = $props();

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
  /** V108-C35 The celebration showing; `n` restarts it. */
  let burst = $state<{ kind: Burst; n: number } | null>(null);
  let burstN = 0;
  /** C35g The last burst's number, kept after it ends: the mark and level restart on a new
   * burst, not when one ends. */
  let lastN = $state(0);
  let burstTimer: ReturnType<typeof setTimeout> | undefined;
  /** The longest celebration (the end's sparkles) and a little air. */
  const BURST_MS = 1_400;

  function celebrate(kind: Burst) {
    clearTimeout(burstTimer);
    burst = { kind, n: ++burstN };
    lastN = burstN;
    burstTimer = setTimeout(() => (burst = null), BURST_MS);
  }

  const on = $derived(settings.v.chat.hypeTrainBar);
  const left = $derived(train ? secondsLeft(train.expiresAt, now) : 0);
  const pct = $derived(train ? progressPct(train.progress, train.goal) : 0);
  const top = $derived(train ? topParts(train.conductors, formatNumber) : null);
  /** V108-C35b The train on screen, running or just closed. */
  const shown = $derived(train ?? note);
  const look = $derived(lookOf(shown));
  /** V108-C35c The closing note keeps where the last level got to (full only when its goal was met). */
  const fill = $derived(train ? pct : note ? progressPct(note.progress, note.goal) : 0);
  const parts = $derived(burstParts(burst?.kind ?? 'level'));
  const theme = $derived(settings.v.appearance.theme);
  const kappa = $derived(look === 'gold' ? emoteImg('twitch', GOLDEN_KAPPA_ID, theme, { scale: density.scale }).src : '');

  /** V108-C35b The timer is full at the level's time, fixed per end time: a later message
   * with the same end time must not rescale it. */
  let spanKey: string | null = null;
  let spanMs = 0;
  function spanOf(expiresAt: string): number {
    if (expiresAt !== spanKey) {
      spanKey = expiresAt;
      spanMs = levelSpan(msLeft(expiresAt, Date.now()));
    }
    return spanMs;
  }
  const timer = $derived(train?.expiresAt ? { expiresAt: train.expiresAt, span: spanOf(train.expiresAt) } : null);

  /** V108-C35b Drains the timer line from what is left to nothing, linear, off the main
   * thread; started again only for a new end time. */
  function drain(node: HTMLElement, t: { expiresAt: string; span: number } | null) {
    let a: Animation | undefined;
    let key = '';
    const run = (t: { expiresAt: string; span: number } | null) => {
      const k = t ? `${t.expiresAt}|${t.span}` : '';
      if (k === key) return;
      key = k;
      a?.cancel();
      const left = t ? msLeft(t.expiresAt, Date.now()) : 0;
      const from = t ? timerFraction(left, t.span) : 0;
      node.style.transform = `scaleX(${from})`;
      if (left > 0) a = node.animate([{ transform: `scaleX(${from})` }, { transform: 'scaleX(0)' }], { duration: left, easing: 'linear', fill: 'forwards' });
    };
    run(t);
    return { update: run, destroy: () => a?.cancel() };
  }
  const given = $derived(note ? (givenLine(note.subs, note.bits, formatNumber) ?? topLine(note.conductors, formatNumber)) : null);

  /** V108-F57c Drops the note and its timers, no focus move: a train ending on its own must not pull focus. */
  function clearNote() {
    noteSeq++;
    clearTimeout(noteTimer);
    clearTimeout(lastTimer);
    note = null;
  }

  /** Closed by hand: focus goes back to the message box if it was lost. */
  function dismiss() {
    clearNote();
    void tick().then(() => {
      const at = document.activeElement;
      if (!at || at === document.body) onclosed?.();
    });
  }

  /** V108-C35i Escape on the note's x closes it, the same way the x does. */
  function onKey(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    dismiss();
  }

  function finish(held: HypeTrain) {
    train = null;
    clearNote();
    note = held;
    celebrate('end');
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
    const who = account;
    void login;
    train = null;
    note = null;
    seq++;
    heard = 0;
    if (!on || !id) return;
    // V108-N04 No read for a channel known to be offline; one outside the tabs is read.
    const known = untrack(() => tabs.live[id]);
    if (!known || known.stream) void load();
    void hypeIpc.watch(id, who, true).catch(() => {});
    const off = hypeIpc.onChange((e) => {
      if (e.channelId !== id) return;
      const c = e.change;
      heard = Date.now();
      const b = burstOf(train, c);
      if (c.kind === 'start') {
        seq++;
        train = c.train;
        note = null;
        // The start message has no givers: read it once.
        void load();
      } else if (c.kind === 'progress') {
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
      if (b) celebrate(b);
    });
    const poll = setInterval(() => {
      if (train) void load();
    }, POLL_MS);
    return () => {
      clearInterval(poll);
      clearTimeout(burstTimer);
      burst = null;
      clearTimeout(noteTimer);
      clearTimeout(lastTimer);
      void off.then((f) => f());
      void hypeIpc.watch(id, who, false).catch(() => {});
    };
  });

  // The countdown ticks only while there is a train on screen. C35g It keys on "is there a
  // train", not the train object, so a message doesn't rebuild the interval.
  const running = $derived(!!train);
  $effect(() => {
    if (!running) return;
    now = Date.now();
    // V110-F84 Not while the window is hidden; it catches up once when it shows again.
    return everyShown(() => {
      now = Date.now();
      if (train && train.expiresAt && secondsLeft(train.expiresAt, now) === 0) finish(train);
    }, 1000);
  });
</script>

{#if shown}
  <div
    class="hype"
    class:gold={look === 'gold'}
    class:in={burst?.kind === 'start'}
    role="status"
    aria-label={train ? `${look === 'gold' ? 'Golden Kappa train' : 'Hype train'}, level ${train.level}, ${pctText(pct)}` : undefined}
  >
    <div class="bg" aria-hidden="true">
      <div class="fill" class:sweep={!!burst && parts.sweep} style:background={gradient(look)} style:clip-path="inset(0 {100 - fill}% 0 0)"></div>
      {#key burst?.n}{#if burst}<span class="flash"></span>{#if parts.sheen}<span class="sheen"></span>{/if}{/if}{/key}
      {#if timer}<div class="timer" use:drain={timer}></div>{/if}
    </div>
    {#key burst?.n}
      {#if burst}
        <span class="fx" class:bolt={burst.kind !== 'end'} class:clip={burst.kind === 'end'}><HypeSparks kind={burst.kind === 'end' ? 'shower' : 'burst'} {look} /></span>
      {/if}
    {/key}
    <div class="row">
      {#key lastN}
        <span class="mark"class:pop={!!burst}>
          {#if kappa}
            <img src={kappa} alt="" use:tooltip={{ label: 'Golden Kappa train' }} />
          {:else}
            <Icon name="zap" size={13} />
          {/if}
        </span>
      {/key}
      {#if train}
        <span class="level"
          >Level {#key lastN}<span class="lv" class:pop={!!burst && parts.popLevel}>{train.level}</span>{/key}</span
        >
        <span class="pct" role="progressbar" aria-label="Progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct}>{pctText(pct)}</span>
        {#if top}<span class="top"><span class="who">{top.who}</span>{#if top.amount}<span class="amount">{top.amount}</span>{/if}</span>{/if}
        <!-- No countdown when Twitch sent no end time. -->
        {#if train.expiresAt}<span class="time" aria-hidden="true">{formatLeft(left)}</span>{/if}
      {:else if note}
        {#key lastN}<span class="level" class:res={!!burst && parts.popResult}>{endedText(note.level)}</span>{/key}
        {#if given}<span class="top"><span class="who">{given}</span></span>{/if}
        <button class="close" type="button" aria-label="Close" use:tooltip={{ label: 'Close' }} onclick={dismiss} onkeydown={onKey}>
          <Icon name="x" size={12} />
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .hype {
    position: relative;
    isolation: isolate;
    display: flex;
    flex-direction: column;
    margin: 0 8px 6px;
    padding: 6px 8px 7px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    /* The washed fill: weak on dark so --text reads, stronger on light. */
    --wash: 0.3;
    --sheen: var(--hype-sheen);
    --drain: color-mix(in srgb, var(--text) 45%, transparent);
  }
  :global(:root[data-theme='light']) .hype,
  :global(.theme-light) .hype {
    --wash: 0.6;
  }
  .hype.gold {
    border-color: color-mix(in srgb, var(--hype-gold) 55%, var(--border));
    --drain: var(--hype-gold);
  }
  /* V108-C35b Fill, sheen and timer, clipped to the bar's corners; the sparkles may leave it. */
  .bg {
    position: absolute;
    inset: 0;
    z-index: -1;
    overflow: hidden;
    border-radius: inherit;
    pointer-events: none;
  }
  .fill {
    position: absolute;
    inset: 0;
    opacity: var(--wash);
    /* Clipped, not resized: the colours stay put and the fill reveals them. */
    transition: clip-path 0.6s var(--ease);
  }
  .fill.sweep {
    animation: sweep 0.9s var(--ease);
  }
  @keyframes sweep {
    from {
      clip-path: inset(0 100% 0 0);
    }
  }
  .sheen {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 40%;
    background: linear-gradient(100deg, transparent, var(--sheen), transparent);
    transform: translateX(-100%);
    animation: sheen 0.9s cubic-bezier(0.77, 0, 0.175, 1) 0.15s both;
  }
  @keyframes sheen {
    to {
      transform: translateX(260%);
    }
  }
  .timer {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--drain);
    transform-origin: left;
  }
  .fx {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  /* The burst leaves from the bolt's middle. */
  /* V109-C02 The end's confetti stays inside the bar. */
  .fx.clip {
    overflow: hidden;
    border-radius: inherit;
  }
  .fx.bolt {
    left: 15px;
    right: auto;
    width: 0;
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
  .lv {
    display: inline-block;
    font-variant-numeric: tabular-nums;
  }
  .lv.pop {
    animation: lv 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  @keyframes lv {
    from {
      opacity: 0.4;
      transform: scale(0.6) translateY(3px);
    }
  }
  /* V109-C02 The result text pops in as the sweep lands. */
  .level.res {
    display: inline-block;
    animation: res 0.55s cubic-bezier(0.34, 1.56, 0.64, 1) 0.2s backwards;
  }
  @keyframes res {
    from {
      opacity: 0;
      transform: scale(0.8) translateY(3px);
    }
  }
  .pct {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .top {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 4px;
    white-space: nowrap;
  }
  /* V108-C35h The name gives way first, then the amount (a narrow chat); the countdown stays. */
  .who {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .amount {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .close {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    /* Keeps the note as slim as the bar. */
    margin: -5px -4px -5px auto;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--text);
    cursor: pointer;
    transition: background-color var(--fast) ease-out;
  }
  .close:hover {
    background: var(--row-hover);
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
    flex: none;
    margin-left: auto;
    font-variant-numeric: tabular-nums;
  }
  /* V108-C35 The tint that comes and goes; with reduced motion it is the whole celebration. */
  .flash {
    position: absolute;
    inset: 0;
    background: color-mix(in srgb, var(--twitch) 22%, transparent);
    opacity: 0;
    animation: flash 0.9s ease-out;
  }
  .gold .flash {
    background: color-mix(in srgb, var(--hype-gold) 26%, transparent);
  }
  @keyframes flash {
    15% {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }
  .mark {
    display: inline-flex;
    flex: none;
  }
  .mark img {
    width: 16px;
    height: 16px;
    margin: -2px 0;
    object-fit: contain;
  }
  .mark.pop {
    animation: pop 0.45s var(--ease);
  }
  @keyframes pop {
    35% {
      transform: scale(1.4) rotate(-8deg);
    }
    to {
      transform: none;
    }
  }
  /* A train that starts comes in, as a popover does. */
  .hype.in {
    animation: enter var(--base) var(--ease);
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
    .hype.in,
    .fill.sweep,
    .mark.pop,
    .level.res,
    .lv.pop {
      animation: none;
    }
    .sheen {
      display: none;
    }
    /* A static highlight for the moment instead. */
    .flash {
      opacity: 0.7;
      animation: none;
    }
  }
</style>
