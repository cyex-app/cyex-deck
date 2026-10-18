<!-- One channel tab (1.10): name or alias (1.22), live dot (1.29), close button (1.25),
     lit by unread chat (U9), a small sound badge while it plays or is muted (V106-25; muting is in
     the right-click menu).
     Double click edits (1.18), middle click closes, drag reorders (U5).
     The tab is the whole cell of its row, up to the window's top edge (V101-19); the pill
     inside is what shows. A thin divider sits between two plain tabs (V101-11); in a side bar it spans the bar (V106-05). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import { accounts } from '../../lib/accounts.svelte';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { keyFocused } from '../player/focusOrigin';
  import { drag } from './drag.svelte';
  import { soundTip } from './tabSound';
  import { label, tabs, tiles } from './tabs.svelte';

  interface Props {
    tab: Tab;
    active: boolean;
    selected: boolean;
    /** The one tab Tab reaches (roving tabindex): the shown tab, else the first. */
    stop?: boolean;
    vertical: boolean;
    /** Color of the tab's group, if any. */
    groupColor?: string;
    onselect: (e: MouseEvent) => void;
    onedit: () => void;
    onclose: () => void;
    onmenu: (e: MouseEvent) => void;
  }

  let { tab, active, selected, stop = false, vertical, groupColor, onselect, onedit, onclose, onmenu }: Props = $props();

  const name = $derived(label(tab));
  const live = $derived(tabs.liveState(tab));
  const unread = $derived(tabs.unreadState(tab));
  const popped = $derived(tabs.popped.has(tab.id));
  /** V102-15 Still playing while something else shows. */
  /** V106-25 What the sound badge shows. */
  const sound = $derived(tabs.sound(tab));
  const closeButton = $derived(settings.v.appearance.tabCloseButton);

  /** Drop marker while something is dragged over this tab. */
  const mark = $derived.by(() => {
    const t = drag.target;
    if (!drag.item || t?.kind !== 'tab' || t.id !== tab.id) return null;
    if (drag.item.kind !== 'tab') return 'into';
    if (drag.item.id === tab.id) return null;
    return t.after ? 'after' : 'before';
  });

  const tip = $derived.by(() => {
    const parts: string[] = [];
    let hint = '';
    for (const tile of tiles(tab)) {
      const stream = tabs.live[tile.channel.id]?.stream;
      if (stream) {
        const kind = stream.rerun ? 'Rerun' : 'Live';
        parts.push(`${tile.channel.name} · ${kind} · ${formatNumber(stream.viewers)} watching`);
        hint ||= stream.game ? `${stream.title} · ${stream.game}` : stream.title;
      }
    }
    const first = tiles(tab)[0];
    const account = first?.account ? accounts.list.find((a) => a.userId === first.account) : undefined;
    const who = account ? `as ${account.displayName}` : 'anonymous';
    const label = parts.length ? parts.join(' · ') : `${name} · Offline`;
    const playing = soundTip(sound, tabs.muted.has(tab.id));
    // No tip while something is dragged: it would sit on the ghost.
    return { label: `${label}${playing}`, hint: hint ? `${hint} (${who})` : who, when: () => !drag.item };
  });

  /** Arrows move between tabs (Up/Down in a vertical bar), Home and End jump, Space and Enter open. */
  function onkey(e: KeyboardEvent) {
    if (e.target !== e.currentTarget || e.ctrlKey || e.altKey || e.metaKey) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onselect(e as unknown as MouseEvent);
      return;
    }
    const prev = vertical ? 'ArrowUp' : 'ArrowLeft';
    const next = vertical ? 'ArrowDown' : 'ArrowRight';
    if (![prev, next, 'Home', 'End'].includes(e.key)) return;
    // V1061-04 rd-1 After a mouse click and with Shift held, the arrows go to the player.
    if (e.shiftKey || !keyFocused(e.currentTarget)) return;
    const all = [...(e.currentTarget as HTMLElement).closest('[data-tab-bar]')!.querySelectorAll<HTMLElement>('[role="tab"]')];
    const at = all.indexOf(e.currentTarget as HTMLElement);
    const to = e.key === 'Home' ? 0 : e.key === 'End' ? all.length - 1 : (at + (e.key === next ? 1 : -1) + all.length) % all.length;
    e.preventDefault();
    all[to]?.focus();
  }

  function onauxclick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault();
      onclose();
    }
  }
</script>

<div
  class="tab"
  class:active
  class:selected
  class:vertical
  class:grouped={!!groupColor}
  class:message={unread === 'message'}
  class:mention={unread === 'mention'}
  data-tab-id={tab.id}
  data-vertical={vertical ? '1' : undefined}
  data-mark={mark}
  style:--group={groupColor}
  role="tab"
  tabindex={stop ? 0 : -1}
  aria-selected={active}
  onclick={onselect}
  ondblclick={onedit}
  {onauxclick}
  onmousedown={(e) => e.button === 1 && e.preventDefault()}
  oncontextmenu={(e) => {
    e.preventDefault();
    onmenu(e);
  }}
  onkeydown={onkey}
  onpointerdown={(e) => drag.begin(e, { kind: 'tab', id: tab.id }, name)}
  use:tooltip={tip}
>
  <span class="pill">
    {#if sound !== 'silent'}
      <!-- V106-25 The badge takes the live dot's slot (decision 2026/10/18). -->
      <span class="slot" aria-hidden="true">
        <span class="snd {live ?? ''}" class:muted={sound === 'muted'}>
          <Icon name={sound === 'muted' ? 'volumeOff' : 'volume'} size={9} />
        </span>
      </span>
    {:else if live}<span class="dot {live}"></span>{/if}
    <span class="name">{name}</span>
    <!-- V106 a-5, s3-7 The badge is only a picture; screen readers hear it here. -->
    {#if tabs.muted.has(tab.id)}<span class="sr-only">, muted</span>{/if}
    {#if popped}<span class="popped"><Icon name="popout" size={12} /></span>{/if}
    {#if closeButton}
      <button
        class="x"
        tabindex="-1"
        aria-label="Close {name}"
        onclick={(e) => {
          e.stopPropagation();
          onclose();
        }}
        onpointerdown={(e) => e.stopPropagation()}
      >
        <Icon name="x" size={12} />
      </button>
    {/if}
  </span>
</div>

<style>
  /* The cell fills its row: one row is as tall as the title bar, so it meets the top edge. */
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    flex: none;
    /* Never wider than the bar: in a narrow window the name ends in "…" instead of being cut. */
    max-width: min(200px, 100%);
    min-height: calc(var(--tab-h) + 2 * var(--row-g, 0px));
  }
  .tab.vertical {
    width: 100%;
    min-width: 0;
    max-width: none;
    min-height: 0;
    height: var(--tab-h);
  }
  .pill {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: var(--tab-h);
    padding: 0 var(--tab-pad);
    border-radius: var(--radius);
    font-size: var(--tab-font);
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
  }
  .vertical .pill {
    flex: 1;
  }
  /* V106-26 Only the name gives way; dot, speaker, icons and the close button keep their size. */
  .pill > :not(.name) {
    flex: none;
  }
  .tab:focus-visible {
    outline: none;
  }
  .tab:focus-visible .pill {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .tab:hover .pill {
    background: var(--surface-hover);
    color: var(--text);
  }
  .active .pill {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  /* U9 New messages brighten the name; mentions also get an accent edge. */
  .message .pill,
  .mention .pill {
    color: var(--text);
  }
  .mention .pill {
    box-shadow: inset 0 2px var(--accent);
  }
  .vertical.mention .pill {
    box-shadow: inset -2px 0 var(--accent);
  }
  .selected:not(.active) .pill {
    background: var(--surface-hover);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  /* V106 s2-11 Selected keeps its mention edge. */
  .selected.mention:not(.active) .pill {
    box-shadow:
      inset 0 2px var(--accent),
      inset 0 0 0 1px var(--border-strong);
  }
  .selected.vertical.mention:not(.active) .pill {
    box-shadow:
      inset -2px 0 var(--accent),
      inset 0 0 0 1px var(--border-strong);
  }
  /* V101-11 In the 2px gap before the tab. The bar clips the one at a row's start. The
     tab before is another instance, hence global. */
  :global(.tab) + .tab::before {
    content: '';
    position: absolute;
    top: 50%;
    left: -1.5px;
    width: 1px;
    height: calc(var(--tab-h) * 0.5);
    translate: 0 -50%;
    background: var(--border-strong);
    pointer-events: none;
  }
  /* V106-05 In a side bar: a line across the whole bar, padding included. V106 a-7 On a whole
     px (-1px, not -1.5px): a half px landed between device pixels at 150 %. V106 ra-5 A border,
     not a 1px fill: the browser snaps border widths to whole device pixels, so at 125/150 % zoom
     every line is the same 1 device px (a fill came out 2 or 3). */
  :global(.tab.vertical) + .tab.vertical::before {
    top: -1px;
    left: calc(-1 * var(--bar-pad, 6px));
    right: calc(-1 * var(--bar-pad, 6px));
    width: auto;
    height: 0;
    translate: none;
    background: none;
    border-top: 1px solid var(--border);
  }
  .tab:is(.active, .selected)::before,
  :global(.tab:is(.active, .selected)) + .tab::before,
  .tab:not(.vertical):hover::before,
  :global(.tab:not(.vertical):hover) + .tab::before {
    opacity: 0;
  }
  .grouped .pill::after {
    content: '';
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: 1px;
    height: 2px;
    border-radius: 1px;
    background: var(--group);
  }
  .vertical.grouped .pill::after {
    left: 1px;
    right: auto;
    top: 6px;
    bottom: 6px;
    width: 2px;
    height: auto;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* 1.29 Glowing, not pulsing. */
  .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
  }
  .dot.live {
    background: var(--live);
    box-shadow: 0 0 6px 1px color-mix(in srgb, var(--live) 75%, transparent);
  }
  .dot.rerun {
    background: var(--yellow);
    box-shadow: 0 0 6px 1px color-mix(in srgb, var(--yellow) 60%, transparent);
  }
  /* V106-25 Sound badge (decision 2026/10/18): it replaces the live dot. Its slot is the dot's
     7 px, so the width never changes; the 12 px chip spills 2.5 px into the padding and the gap,
     never onto the name or past the pill. Info only (muting is in the menu). Red while live,
     yellow edge on a rerun, else a neutral chip with a hairline; crossed and brighter while the
     tab is muted (or held with every player muted, V102-15). */
  .slot {
    position: relative;
    width: 7px;
    height: 7px;
  }
  .snd {
    position: absolute;
    top: 50%;
    left: 50%;
    translate: -50% -50%;
    display: grid;
    place-items: center;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--elevated);
    box-shadow: 0 0 0 1px var(--border);
    color: var(--muted);
    pointer-events: none;
    animation: snd-in 160ms var(--ease, ease-out);
  }
  /* Thin icon strokes would vanish at 9 px. */
  .snd :global(svg) {
    stroke-width: 2.6;
  }
  .snd.muted {
    color: var(--text);
  }
  .snd.live {
    background: var(--live);
    box-shadow: 0 0 5px color-mix(in srgb, var(--live) 60%, transparent);
    color: var(--on-solid);
  }
  .snd.rerun {
    box-shadow: 0 0 0 1px var(--yellow);
    color: var(--yellow);
  }
  @keyframes snd-in {
    from {
      opacity: 0;
      scale: 0.6;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .snd {
      animation: none;
    }
  }
  .popped {
    display: flex;
    color: var(--dim);
  }
  .x {
    position: relative;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-right: -4px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
  }
  .x:hover {
    background: var(--surface);
    color: var(--text);
  }
  /* 24 px to hit, 18 px to see. */
  .x::after {
    content: '';
    position: absolute;
    inset: -3px;
  }
  /* Drop marks sit just inside the pill, so a row's first tab shows one too. */
  [data-mark='before'] .pill::before,
  [data-mark='after'] .pill::before {
    content: '';
    position: absolute;
    top: 3px;
    bottom: 3px;
    width: 2px;
    border-radius: 1px;
    /* V106 s2-3 Accent text is kept readable on the page; the fill can be near the page color. */
    background: var(--accent-text);
  }
  [data-mark='before'] .pill::before {
    left: 0;
  }
  [data-mark='after'] .pill::before {
    right: 0;
  }
  /* V106-05 In a side bar the mark is a full-width line in the gap. */
  .vertical[data-mark='before'] .pill::before,
  .vertical[data-mark='after'] .pill::before {
    content: none;
  }
  .vertical:is([data-mark='before'], [data-mark='after'])::after {
    content: '';
    position: absolute;
    left: calc(-1 * var(--bar-pad, 6px));
    right: calc(-1 * var(--bar-pad, 6px));
    height: 2px;
    border-radius: 1px;
    background: var(--accent-text);
    pointer-events: none;
  }
  .vertical[data-mark='before']::after {
    top: -2px;
  }
  .vertical[data-mark='after']::after {
    bottom: -2px;
  }
  [data-mark='into'] .pill {
    box-shadow: inset 0 0 0 2px var(--accent-border);
  }
</style>
