<!-- V108-N04 A slim bar at the top of a live chat while a hype train runs: level, progress to
     the next level, time left and the top giver. Kept current by the train's PubSub topic, plus
     a read every minute while a train is known and once when the chat opens, unless the app
     knows the channel is offline (the topic is followed either way: it is cheap and reports a
     train that starts later). After the train
     it says how far it got for a few seconds, then goes. Setting "Hype train bar" (on). -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import type { HypeTrain } from '../../ipc/bindings/HypeTrain';
  import { hypeIpc } from '../../ipc/hypetrain';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { tabs } from '../tabs/tabs.svelte';
    import { endedText, formatLeft, mergeConductor, mergeProgress, progressPct, secondsLeft, topLine, trainOver } from './hypeTrain';

  interface Props {
    login: string;
    /** The chat's channel id; the topic needs it. */
    channelId: string | null;
  }

  let { login, channelId }: Props = $props();

  const POLL_MS = 60_000;
  const NOTE_MS = 6_000;

  let train = $state<HypeTrain | null>(null);
  /** The closing note, while it shows. */
  let note = $state<string | null>(null);
  let now = $state(Date.now());
  let seq = 0;
  /** When the topic last said something, or a read last found the train. */
  let heard = 0;
  let noteTimer: ReturnType<typeof setTimeout> | undefined;

  const on = $derived(settings.v.chat.hypeTrainBar);
  const left = $derived(train ? secondsLeft(train.expiresAt, now) : 0);
  const pct = $derived(train ? progressPct(train.progress, train.goal) : 0);
  const top = $derived(train ? topLine(train.conductors, formatNumber) : null);

  function finish(level: number) {
    train = null;
    clearTimeout(noteTimer);
    note = endedText(level);
    noteTimer = setTimeout(() => (note = null), NOTE_MS);
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
        finish(train.level);
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
        if (train) finish(train.level);
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
      if (train && train.expiresAt && secondsLeft(train.expiresAt, now) === 0) finish(train.level);
    }, 1000);
    return () => clearInterval(t);
  });
</script>

{#if train}
  <div class="hype" role="status" aria-label="Hype train, level {train.level}">
    <div class="row">
      <Icon name="zap" size={13} />
      <span class="level">Level {train.level}</span>
      {#if top}<span class="top">{top}</span>{/if}
      <span class="time">{formatLeft(left)}</span>
    </div>
    <div class="track" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct}>
      <div class="fill" style:transform="scaleX({pct / 100})"></div>
    </div>
  </div>
{:else if note}
  <div class="hype" role="status">
    <div class="row"><Icon name="zap" size={13} /><span class="level">{note}</span></div>
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
