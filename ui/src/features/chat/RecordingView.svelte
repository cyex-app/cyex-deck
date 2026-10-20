<!-- X6 Plays a chat recording back in its own window, at its own pace, with the pictures
     saved next to it. Route: `recording/<id>`. -->
<script lang="ts">
  import { onDestroy, tick } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { Gone } from '../../ipc/bindings/Gone';
  import type { Recording } from '../../ipc/bindings/Recording';
  import type { RoomEvent } from '../../ipc/bindings/RoomEvent';
  import { chatxIpc } from '../../ipc/chatx';
  import { formatDate, formatDuration, formatTime } from '../../lib/format';
  import { lineClick } from './clicks';
  import { markGone } from './gone';
  import Line from './Line.svelte';
  import { dropSaved, useSaved } from './saved';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  let { route }: { route: string; popout?: boolean } = $props();

  const KEEP = 1000;
  const SPEEDS = [
    { value: 1, label: '1×' },
    { value: 2, label: '2×' },
    { value: 4, label: '4×' },
  ] as const;

  let rec = $state.raw<Recording | null>(null);
  let error = $state('');
  let at = $state(0);
  let playing = $state(false);
  let speed = $state<1 | 2 | 4>(1);
  let lines = $state.raw<ChatMessage[]>([]);
  let list = $state<HTMLElement>();
  /** Events applied so far. */
  let next = 0;

  const events = $derived((rec?.events ?? []) as [number, RoomEvent][]);
  const length = $derived(rec ? Math.max(rec.meta.endedMs - rec.meta.startedMs, events.at(-1)?.[0] ?? 0) : 0);

  // svelte-ignore state_referenced_locally
  const id = route.split('/')[1] ?? '';
  chatxIpc
    .loadRecording(id)
    .then((r) => {
      useSaved(id, r.assets);
      rec = r;
      playing = true;
    })
    .catch((e) => (error = errorText(e)));
  onDestroy(() => dropSaved(id));

  function apply(e: RoomEvent, out: ChatMessage[]): ChatMessage[] {
    // V107-R34 With why, for the note.
    const mark = (hit: (m: ChatMessage) => boolean, gone?: Gone) => out.map((m) => (hit(m) ? (markGone(m, gone) ?? m) : m));
    switch (e.type) {
      case 'message':
        return [...out, e.message];
      case 'history':
      case 'gap':
        return [...out, ...e.messages];
      case 'update':
        return out.map((m) => (m.id === e.message.id ? e.message : m));
      case 'delete':
        return mark((m) => m.id === e.id, e.gone);
      case 'clearUser':
        return mark((m) => !m.notice && m.sender?.id === e.userId, e.gone);
      case 'remove':
        return out.filter((m) => m.id !== e.id);
      case 'wipe':
        return [];
      default:
        return out;
    }
  }

  /** Brings the lines to `to` ms, from the start when going back. */
  async function seek(to: number) {
    // Scrolled up to read: playback leaves the view alone. A jump back or forward snaps down.
    const stick = to < at || to - at > 1000 || !list || list.scrollHeight - list.scrollTop - list.clientHeight < 40;
    let out = lines;
    if (to < at) {
      out = [];
      next = 0;
    }
    while (next < events.length && events[next][0] <= to) out = apply(events[next++][1], out);
    lines = out.length > KEEP ? out.slice(-KEEP) : out;
    at = to;
    await tick();
    if (stick) list?.scrollTo({ top: list.scrollHeight });
  }

  $effect(() => {
    if (!playing || !rec) return;
    let last = performance.now();
    const t = setInterval(() => {
      const now = performance.now();
      const to = Math.min(length, at + (now - last) * speed);
      last = now;
      void seek(to);
      if (to >= length) playing = false;
    }, 100);
    return () => clearInterval(t);
  });

  function toggle() {
    if (!playing && at >= length) void seek(0);
    playing = !playing;
  }

  const clock = (ms: number) => formatDuration(Math.floor(ms / 1000));
  const started = $derived(rec ? new Date(rec.meta.startedMs) : null);
</script>

<section class="replay">
  {#if error}
    <p class="note">{error}</p>
  {:else if !rec}
    <Loading />
  {:else}
    <header>
      <strong>#{rec.meta.channel}</strong>
      {#if started}<span class="when">{formatDate(started)} {formatTime(started)}</span>{/if}
    </header>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="lines" bind:this={list} onclick={lineClick}>
      {#each lines as m (m.id)}
        <div data-channel={rec.meta.channel}><Line {m} channel={rec.meta.channel} /></div>
      {/each}
    </div>
    <footer>
      <button class="play" aria-label={playing ? 'Pause' : 'Play'} onclick={toggle}>
        <Icon name={playing ? 'pause' : 'play'} size={16} />
      </button>
      <input
        type="range"
        min="0"
        max={length}
        step="1000"
        value={at}
        aria-label="Position"
        oninput={(e) => void seek(Number(e.currentTarget.value))}
      />
      <span class="time">{clock(at)} / {clock(length)}</span>
      <Segmented label="Speed" options={SPEEDS} value={speed} onchange={(v) => (speed = v)} />
    </footer>
  {/if}
</section>

<style>
  .replay {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--page);
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }
  .when {
    color: var(--dim);
    font-size: 0.8rem;
  }
  .lines {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-top: 1px solid var(--border);
  }
  footer input {
    flex: 1;
    min-width: 0;
  }
  .play {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .time {
    color: var(--muted);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .note {
    padding: 12px;
    color: var(--dim);
  }
</style>
