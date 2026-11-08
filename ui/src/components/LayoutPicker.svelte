<!-- V108-S2 One small window drawing for both layout settings: click a screen edge to move the tab bar, the middle to move chat. The drawing is the live preview. -->
<script lang="ts">
  import {
    aboveSplit,
    barSplit,
    chatSplit,
    chatZone,
    H,
    sayPick,
    sideForKey,
    SIDES,
    tabZone,
    TITLE,
    toClip,
    toPoints,
    W,
    type Rect,
    type Side,
  } from './layoutPicker';

  type Kind = 'tabs' | 'chat';
  interface Props {
    /** V110-U52 `aboveChat` is drawn over the chat column; the Top radio stands for it. */
    tabs: Side | 'aboveChat';
    chat: Side;
    onTabs: (side: Side) => void;
    onChat: (side: Side) => void;
  }

  let { tabs, chat, onTabs, onChat }: Props = $props();

  const NAMES: Record<Kind, string> = { tabs: 'Tab bar position', chat: 'Chat position' };

  const above = $derived(tabs === 'aboveChat');
  /** The radio the tab bar's pick lights. */
  const tabSide = $derived<Side>(tabs === 'aboveChat' ? 'top' : tabs);
  const over = $derived(above ? aboveSplit(chat) : null);
  const split = $derived(over ?? barSplit(tabSide));
  const parts = $derived(over ?? chatSplit(chat, split.rest));
  const vertical = $derived(tabSide === 'left' || tabSide === 'right');

  /** What the pointer or the keyboard is on right now. */
  let hover = $state<{ kind: Kind; side: Side } | null>(null);
  let focus = $state<{ kind: Kind; side: Side } | null>(null);
  const aim = $derived(hover ?? focus);

  /** The block the aimed pick would draw. */
  const ghost = $derived.by((): Rect | null => {
    if (!aim) return null;
    return aim.kind === 'tabs' ? barSplit(aim.side).bar : chatSplit(aim.side, split.rest).chat;
  });

  const caption = $derived(aim ? sayPick(aim.kind, aim.side) : `${above ? 'Tabs above chat' : sayPick('tabs', tabSide)} · ${sayPick('chat', chat).replace('Chat', 'chat')}`);

  const value = (kind: Kind) => (kind === 'tabs' ? tabSide : chat);
  const pick = (kind: Kind, side: Side) => (kind === 'tabs' ? onTabs(side) : onChat(side));
  const zone = (kind: Kind, side: Side) => toClip(kind === 'tabs' ? tabZone(side) : chatZone(side));

  const groups: Kind[] = ['tabs', 'chat'];
  let root = $state<HTMLElement>();

  function onkeydown(e: KeyboardEvent, kind: Kind) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    const side = sideForKey(e.key);
    if (!side) return;
    e.preventDefault();
    hover = null; // the keyboard leads now, even with the pointer resting on the drawing
    pick(kind, side);
    root?.querySelector<HTMLElement>(`[data-kind="${kind}"][data-side="${side}"]`)?.focus();
  }

  function onfocus(e: FocusEvent, kind: Kind, side: Side) {
    if ((e.currentTarget as HTMLElement).matches(':focus-visible')) focus = { kind, side };
  }

  // Tab pills in the bar, chat lines in the chat: a hint of what each is, nothing more.
  const PILL = 5;
  const pills = $derived.by(() => {
    const { bar } = split;
    return [0, 1, 2].map((i) =>
      vertical
        ? { x: bar.x + 6, y: bar.y + 8 + i * 17, w: bar.w - 12, h: 11 }
        : bar.w < 200
          ? { x: bar.x + 6 + i * 30, y: bar.y + 6, w: 24, h: bar.h - 12 }
          : { x: bar.x + 8 + i * 44, y: bar.y + 6, w: 38, h: bar.h - 12 },
    );
  });
  const LINES = [0.78, 0.5, 0.66, 0.4, 0.58];
  const lines = $derived.by(() => {
    const c = parts.chat;
    const n = Math.min(LINES.length, Math.max(Math.floor((c.h - 10) / 14), 1));
    return Array.from({ length: n }, (_, i) => ({ x: c.x + 24, y: c.y + 12 + i * 14, w: Math.max((c.w - 34) * LINES[i], 8) }));
  });
  const play = $derived.by(() => {
    const p = parts.player;
    const cx = p.x + p.w / 2;
    const cy = p.y + p.h / 2;
    return toPoints([
      [cx - 8, cy - 11],
      [cx - 8, cy + 11],
      [cx + 12, cy],
    ]);
  });
</script>

<div class="lp" bind:this={root}>
  <div class="stage">
    <svg viewBox="0 0 {W} {H}" aria-hidden="true">
      <rect class="frame" x="0.5" y="0.5" width={W - 1} height={H - 1} rx="4.5" />
      <rect class="title" x="1" y="1" width={W - 2} height={TITLE - 1} />
      <rect class="dot" x="9" y="7" width="22" height="4" rx="2" />
      <rect class="player" x={parts.player.x} y={parts.player.y} width={parts.player.w} height={parts.player.h} />
      <polygon class="play" points={play} />
      <rect class="chat" x={parts.chat.x} y={parts.chat.y} width={parts.chat.w} height={parts.chat.h} />
      {#each lines as l, i (i)}
        <circle class="avatar" cx={l.x - 9} cy={l.y + 2} r="3.5" />
        <rect class="line" x={l.x} y={l.y} width={l.w} height="4" rx="2" />
      {/each}
      <rect class="bar" x={split.bar.x} y={split.bar.y} width={split.bar.w} height={split.bar.h} />
      {#each pills as p, i (i)}
        <rect class="pill" class:on={i === 0} x={p.x} y={p.y} width={p.w} height={p.h} rx={PILL / 2} />
      {/each}
      <rect class="edge" x="0.5" y="0.5" width={W - 1} height={H - 1} rx="4.5" />
      {#if ghost}
        <rect
          class="ghost"
          class:key={!hover}
          x={ghost.x + 1}
          y={ghost.y + 1}
          width={ghost.w - 2}
          height={ghost.h - 2}
          rx="2"
        />
      {/if}
    </svg>

    {#each groups as kind (kind)}
      <div class="zones" role="radiogroup" aria-label={NAMES[kind]}>
        {#each SIDES as side (side)}
          <button
            type="button"
            role="radio"
            aria-checked={value(kind) === side}
            aria-label={sayPick(kind, side)}
            data-kind={kind}
            data-side={side}
            tabindex={value(kind) === side ? 0 : -1}
            style:clip-path={zone(kind, side)}
            onclick={() => pick(kind, side)}
            onkeydown={(e) => onkeydown(e, kind)}
            onpointerenter={() => (hover = { kind, side })}
            onpointerleave={() => (hover = null)}
            onfocus={(e) => onfocus(e, kind, side)}
            onblur={() => (focus = null)}
          ></button>
        {/each}
      </div>
    {/each}
  </div>
  <p class="cap" class:live={!!aim} aria-hidden="true">{caption}</p>
</div>

<style>
  .lp {
    width: 100%;
    max-width: 360px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .stage {
    position: relative;
    aspect-ratio: 360 / 220;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
  }
  .zones {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  button {
    position: absolute;
    inset: 0;
    padding: 0;
    border: 0;
    background: none;
    pointer-events: auto;
    cursor: pointer;
    outline: none;
  }

  .frame {
    fill: var(--surface);
  }
  .edge {
    fill: none;
    stroke: var(--border-strong);
  }
  .title {
    fill: var(--elevated-2);
  }
  .dot {
    fill: var(--border-strong);
  }
  .player {
    fill: var(--bg);
  }
  /* On light the page tone is too close to the frame: the player gets the chip tone. */
  :global(:root[data-theme='light']) .player {
    fill: var(--neutral-chip);
  }
  .play {
    fill: var(--dim);
    opacity: 0.55;
  }
  .chat {
    fill: var(--surface);
    stroke: var(--border);
  }
  .avatar,
  .line {
    fill: var(--border-strong);
  }
  .bar {
    fill: var(--elevated-2);
    stroke: var(--border);
  }
  .pill {
    fill: var(--border-strong);
  }
  .pill.on {
    fill: var(--accent-text);
  }

  /* Where a pick would land: a soft tint with an outline. The keyboard's version is stronger. */
  .ghost {
    fill: var(--accent-dim);
    stroke: var(--accent-text);
    stroke-width: 1.5;
    pointer-events: none;
    animation: lp-in var(--fast) ease-out;
  }
  .ghost.key {
    stroke: var(--focus-ring);
    stroke-width: 2;
  }
  @keyframes lp-in {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .ghost {
      animation: none;
    }
  }

  .cap {
    margin: 0;
    min-height: 1.2em;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  .cap.live {
    color: var(--text);
    font-weight: 600;
  }

  @media (prefers-contrast: more) {
    .ghost {
      stroke: var(--text);
    }
  }
  @media (forced-colors: active) {
    .frame,
    .title,
    .chat,
    .bar {
      fill: Canvas;
      stroke: ButtonText;
    }
    .player {
      fill: Canvas;
    }
    .dot,
    .avatar,
    .line,
    .pill,
    .play {
      fill: ButtonText;
    }
    .pill.on {
      fill: Highlight;
    }
    .edge {
      stroke: ButtonText;
    }
    .ghost,
    .ghost.key {
      fill: none;
      stroke: Highlight;
    }
  }
</style>
