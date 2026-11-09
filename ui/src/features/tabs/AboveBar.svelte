<!-- V110-U54 The one tab bar of "Above chat" (V110-U52). Mounted once by the window and moved,
     never redrawn: into the title bar or the narrow-window row (where Top puts it), into the
     slot at the top of the shown tile's chat column, or fixed at the box it had while the
     pointer is on it (V110-F11). Moving the node keeps the bar's state (selection, wheel steps)
     and, with `moveBefore`, keyboard focus. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import type { Tab } from '../../ipc/bindings/Tab';
  import { scrollWithin } from '../../lib/scrollWithin';
  import { settings } from '../../lib/settings.svelte';
  import { aboveChat, type BarSlots } from './aboveChat.svelte';
  import { type AboveHold, type AboveSpot, aboveOneRow, aboveSpot, holdAt, inHold, linesUp, type Rect, startsHold } from './barLayout';
  import TabBar from './TabBar.svelte';

  interface Props {
    current: string | null;
    onopen: (id: string) => void;
    /** The tab on screen, if it may have the bar over its chat (not while "Reopen your tabs?" shows). */
    tab: Tab | undefined;
    slots: BarSlots;
    /** Where Top puts the bar: in the title bar, the narrow-window row. */
    chromeSlot: HTMLElement | undefined;
    rowSlot: HTMLElement | undefined;
    /** Where it waits while floating or hidden. */
    park: HTMLElement | undefined;
    theatre: boolean;
    width: number;
    height: number;
    inert: boolean;
  }

  let { current, onopen, tab, slots, chromeSlot, rowSlot, park, theatre, width, height, inert }: Props = $props();

  let host = $state<HTMLElement>();
  let spot = $state<AboveSpot>('none');
  let held = $state.raw<AboveHold | null>(null);
  let hostW = $state(0);
  /** Bumped when the bar steps out to the park, so it's placed again. */
  let parked = $state(0);
  /** Where it floats: the held box, or its last box while the new slot isn't drawn yet. */
  let floatAt = $state.raw<Rect | null>(null);
  /** Its box when it last stepped out of a slot. */
  let lastBox: Rect | null = null;
  /** Bumped when the slot gets its size; `gaveUp`: a slot that never got one. */
  let slotSized = $state(0);
  let gaveUp = $state.raw<HTMLElement | null>(null);

  const slotEl = $derived(tab && aboveChat.applies(tab) ? slots.slots.get(tab.id) : undefined);

  const box = (el: Element): Rect => {
    const r = el.getBoundingClientRect();
    return { x: r.left, y: r.top, w: r.width, h: r.height };
  };

  /** Moves the bar without redrawing it; focus stays where `moveBefore` exists, else comes back. */
  function put(parent: HTMLElement) {
    const el = host;
    if (!el || el.parentElement === parent) return;
    const focused = el.contains(document.activeElement) ? (document.activeElement as HTMLElement) : null;
    try {
      if (el.isConnected && parent.isConnected && 'moveBefore' in parent) (parent as HTMLElement & { moveBefore(n: Node, c: Node | null): void }).moveBefore(el, null);
      else parent.appendChild(el);
    } catch {
      parent.appendChild(el);
    }
    if (focused && document.activeElement !== focused) focused.focus({ preventScroll: true });
    // V110-F80 A moved bar starts at the top of its rows (and a one-row bar is shorter): the next
    // frame, after the page has updated, the shown tab is brought back into view.
    requestAnimationFrame(() => {
      const shown = el.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');
      if (shown) scrollWithin(shown, 'nearest');
    });
  }

  // Before the page updates (this sits before the tab views in the window): a tab change with
  // the pointer on the bar holds it (V110-F11), and a bar in a tile's slot steps out to the park,
  // so the old tile going away never takes it (or its focus) along.
  let lastTab: string | null = untrack(() => current);
  $effect.pre(() => {
    const now = current;
    void slotEl;
    void theatre;
    untrack(() => {
      const el = host;
      if (!el) return;
      const changed = now !== lastTab;
      if (changed) {
        lastTab = now;
        if (!held) {
          const bar = el.querySelector('[data-tab-bar]');
          const hovered = el.matches(':hover') || !!bar?.matches(':hover');
          // In the title bar the host has no box of its own (display: contents).
          const rect = spot === 'chrome' || spot === 'row' ? (bar ? box(bar) : null) : box(el);
          const at = holdAt(spot);
          if (at && startsHold(settings.v.appearance.tabBarSide, hovered, rect) && rect) held = { at, rect };
        }
      }
      const inSlot = el.parentElement?.closest('[data-chat-col]') ? el.parentElement : null;
      if (park && inSlot && (changed || theatre || inSlot !== slotEl)) {
        lastBox = box(el);
        put(park);
        parked++;
      }
    });
  });

  // Where it goes now, once the page has drawn. Same frame, so it's never missing or doubled.
  $effect(() => {
    void parked;
    void slotSized;
    const h = held;
    const slot = slotEl;
    // V110-F80 The places it goes to are watched too: one that mounts after this ran (the narrow
    // window's row after "Reopen your tabs?") left the bar parked, hidden, until a tab with a slot.
    void [host, chromeSlot, rowSlot, park];
    // A new tile lays its panels out a frame after it mounts (PanelLayout measures first).
    const drawn = !!slot && slot.isConnected && slot.checkVisibility() && box(slot).w > 0;
    const lined = !!(h?.at === 'chat' && drawn && linesUp(box(slot!), h.rect));
    let next = aboveSpot({ theatre, width, slot: drawn, held: h?.at ?? null, lined });
    let at = h?.rect ?? null;
    untrack(() => {
      // Meanwhile it stays where it was: at the top, or at its last box over a chat.
      if (!h && slot && !drawn && !theatre && gaveUp !== slot) {
        if (spot === 'chrome' || spot === 'row') next = spot;
        else if (lastBox) {
          next = 'float';
          at = lastBox;
        }
        const waiting = slot;
        setTimeout(() => {
          if (slotEl === waiting && !waiting.checkVisibility()) gaveUp = waiting;
        }, 300);
      }
      const parent = next === 'slot' ? slot : next === 'chrome' ? chromeSlot : next === 'row' ? rowSlot : park;
      if (parent) put(parent);
      // V110-F80 Its width is known now, so the first frame in a narrow slot is already one row
      // (it drew all its rows for a frame before the resize came in).
      if (parent && (next === 'slot' || next === 'float')) hostW = next === 'float' && at ? at.w : parent.clientWidth;
      floatAt = next === 'float' ? at : null;
      spot = parent ? next : 'none';
    });
  });

  // The slot got its size: place again (same frame, before paint).
  $effect(() => {
    const slot = slotEl;
    if (!slot) return;
    const ro = new ResizeObserver(() => slotSized++);
    ro.observe(slot);
    return () => ro.disconnect();
  });

  // V110-F11 Released once the pointer leaves the held box (or the window, or it's resized).
  $effect(() => {
    const h = held;
    if (!h) return;
    const move = (e: PointerEvent) => {
      if (!inHold(h.rect, e.clientX, e.clientY)) held = null;
    };
    const off = () => (held = null);
    const root = document.documentElement;
    addEventListener('pointermove', move);
    root.addEventListener('pointerleave', off);
    addEventListener('blur', off);
    addEventListener('resize', off);
    return () => {
      removeEventListener('pointermove', move);
      root.removeEventListener('pointerleave', off);
      removeEventListener('blur', off);
      removeEventListener('resize', off);
    };
  });
  // V110-F19 Theatre drops a hold.
  $effect(() => {
    if (theatre) untrack(() => (held = null));
  });

  // The bar's height becomes the slots' room; its width picks one row (V110-F42).
  $effect(() => {
    const el = host;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      if (spot !== 'slot' && spot !== 'float') return;
      hostW = el.clientWidth;
      if (spot === 'slot' && el.offsetHeight > 0) slots.height = el.offsetHeight;
    });
    ro.observe(el);
    return () => ro.disconnect();
  });
  $effect(() => {
    // A move into a slot of the same size fires no resize.
    if ((spot === 'slot' || spot === 'float') && host) untrack(() => (hostW = host!.clientWidth));
  });

  // V110-F19 Keyboard focus that was in the bar and got lost on a switch (its tile went away
  // first) goes to the shown tab's item; focus that moved on, to the chat, stays.
  let focusTab: string | null = untrack(() => current);
  $effect.pre(() => {
    const now = current;
    if (now === focusTab) return;
    focusTab = now;
    const inBar = !!host?.contains(document.activeElement);
    if (!inBar) return;
    requestAnimationFrame(() => {
      const lost = !document.activeElement || document.activeElement === document.body;
      if (lost) host?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]')?.focus({ preventScroll: true });
    });
  });
</script>

<div
  class="above {spot}"
  bind:this={host}
  data-bar-above={spot === 'slot' || spot === 'float' ? '' : undefined}
  inert={inert && spot !== 'chrome'}
  style:left={spot === 'float' && floatAt ? `${floatAt.x}px` : undefined}
  style:top={spot === 'float' && floatAt ? `${floatAt.y}px` : undefined}
  style:width={spot === 'float' && floatAt ? `${floatAt.w}px` : undefined}
>
  <TabBar chrome={spot === 'chrome'} {current} {onopen} oneRow={(spot === 'slot' || spot === 'float') && aboveOneRow(hostW, height)} />
</div>

<style>
  /* At the top the bar is laid out as Top's (the title bar's flex row, the row's padding). */
  .chrome,
  .row {
    display: contents;
  }
  .none {
    display: none;
  }
  /* V110-U52 A normal row of the chat column: not a drag area, as wide as the chat. Not
     stretched by the slot's kept height, so a bar that loses a row shrinks and says so. */
  .slot,
  .float {
    display: flex;
    flex-direction: column;
    flex: none;
    box-sizing: border-box;
    min-width: 0;
    padding: 0 6px;
    background: var(--page);
    border-bottom: 1px solid var(--border);
  }
  /* V110-F41 Held over the stream it needs an edge and a lift (floating surface). The edge is a
     ring outside the box, so nothing inside moves by a pixel when it starts floating. */
  .float {
    position: fixed;
    z-index: calc(var(--z-popover) - 1);
    border-bottom-color: var(--border-strong);
    box-shadow:
      0 0 0 1px var(--border-strong),
      var(--shadow-pop);
  }
</style>
