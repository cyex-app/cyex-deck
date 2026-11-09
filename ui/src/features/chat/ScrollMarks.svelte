<!-- V110-C55 Ticks on the chat's scrollbar track, one per highlight, mention or event line, in
     that line's color. One canvas over the track; it is redrawn at most a few times a second,
     when lines or the list's height change, never per frame or per scroll. A press on a tick
     (MessageList asks `pick`) goes to that line. -->
<script lang="ts">
  import { onDestroy, untrack } from 'svelte';

  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import { settings } from '../../lib/settings.svelte';
  import { markOf, nearest, place, type Placed, type Tick } from './scrollMarks';

  interface Props {
    /** The scrolling list the ticks belong to. */
    list: HTMLElement | undefined;
    /** The element holding the drawn lines. */
    lines: HTMLElement | undefined;
    /** The drawn lines, oldest first. */
    messages: readonly ChatMessage[];
  }

  let { list, lines, messages }: Props = $props();

  /** The scrollbar's width (tokens.css). */
  const TRACK = 11;
  /** Most redraws per second. */
  const EVERY_MS = 150;

  let canvas = $state<HTMLCanvasElement>();
  let ticks: Tick[] = [];
  let timer = 0;
  let shown = false;
  /** V110-F84 A redraw skipped while the window was hidden. */
  let owed = false;

  /** `#rrggbbaa` is read as it is; `--name` from the theme. */
  function paint(token: string, style: CSSStyleDeclaration): string {
    return token.startsWith('--') ? style.getPropertyValue(token).trim() || 'currentColor' : token;
  }

  function draw() {
    timer = 0;
    if (document.hidden) return void (owed = true);
    const el = list;
    const cv = canvas;
    if (!el || !cv) return;
    const height = el.clientHeight;
    const content = el.scrollHeight;
    const show = content > height + 1;
    // Nothing to scroll, or nothing marked: leave the canvas alone once it is clear.
    const found: Placed[] = [];
    if (show) {
      const wanted = new Map<string, ReturnType<typeof markOf>>();
      for (const m of untrack(() => messages)) {
        const look = markOf(m);
        if (look) wanted.set(m.id, look);
      }
      if (wanted.size && lines) {
        const top = el.getBoundingClientRect().top - el.scrollTop;
        for (const row of lines.children) {
          const id = (row as HTMLElement).dataset.id;
          const look = id ? wanted.get(id) : undefined;
          if (!id || !look) continue;
          const r = row.getBoundingClientRect();
          if (r.height > 0) found.push({ id, mid: r.top + r.height / 2 - top, ...look });
        }
      }
    }
    ticks = place(found, content, height);
    if (!ticks.length && !shown) return;
    shown = ticks.length > 0;
    const dpr = devicePixelRatio || 1;
    const w = Math.round(TRACK * dpr);
    const h = Math.round(height * dpr);
    if (cv.width !== w) cv.width = w;
    if (cv.height !== h) cv.height = h;
    // V110-F21 The list's height, not the wrapper's: the pill's reserve makes the wrapper taller.
    cv.style.height = `${height}px`;
    const g = cv.getContext('2d');
    if (!g) return;
    g.clearRect(0, 0, w, h);
    const style = getComputedStyle(document.documentElement);
    for (const t of ticks) {
      g.fillStyle = paint(t.color, style);
      g.beginPath();
      g.roundRect(2 * dpr, Math.round((t.y - 1.5) * dpr), 7 * dpr, Math.max(2, 3 * dpr), dpr);
      g.fill();
    }
  }

  function schedule() {
    if (document.hidden) return void (owed = true);
    if (!timer) timer = window.setTimeout(draw, EVERY_MS);
  }
  onDestroy(() => clearTimeout(timer));

  // New or removed lines, the theme, or the setting turning on.
  $effect(() => {
    void messages;
    void settings.v.appearance.theme;
    schedule();
  });

  // Lines growing (emotes loading, wrapping) or the list resizing move the ticks without a new line.
  $effect(() => {
    const els = [list, lines].filter((x): x is HTMLElement => !!x);
    if (!els.length) return;
    const seen = new ResizeObserver(schedule);
    els.forEach((x) => seen.observe(x));
    return () => seen.disconnect();
  });

  /** The line whose tick is at this pointer position, when it is on the scrollbar. */
  export function pick(clientX: number, clientY: number): string | null {
    if (!list || !ticks.length) return null;
    const r = list.getBoundingClientRect();
    if (clientX < r.right - TRACK) return null;
    return nearest(ticks, clientY - r.top)?.id ?? null;
  }
</script>

<!-- V110-F21 A new pixel ratio (another monitor, zoom) comes with a resize. -->
<svelte:window onresize={schedule} />
<!-- V110-F84 Redrawn once when the window shows again. -->
<svelte:document
  onvisibilitychange={() => {
    if (!document.hidden && owed) {
      owed = false;
      schedule();
    }
  }}
/>

<canvas bind:this={canvas} class="marks" width={TRACK} aria-hidden="true"></canvas>

<style>
  .marks {
    position: absolute;
    top: 0;
    right: 0;
    width: 11px;
    height: 100%;
    pointer-events: none;
  }
</style>
