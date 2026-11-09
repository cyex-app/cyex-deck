<!-- Draws the shared menu at the pointer or under its button, kept inside the window
     (V101-04). Arrow keys move through it, Right opens a submenu, Left goes back (V101-30).
     V107-P5 An item with `confirm` shows its callout over itself and waits for a second pick.
     V110-U69 A submenu opens beside its row on hover (or Right / Enter) and the lists stay side by
     side; a window too narrow for two menus swaps the list in place with a back row, as before. -->
<script lang="ts">
  import { tick } from 'svelte';

  import { layer } from '../lib/escape';
  import { focusScope } from '../lib/focusScope';
  import { byKeyboard } from '../lib/motion';
  import { floating, type Rect } from '../lib/place';
  import { settings } from '../lib/settings.svelte';
  import Avatar from './Avatar.svelte';
  import ConfirmCallout from './ConfirmCallout.svelte';
  import GateFlag from './GateFlag.svelte';
  import Icon from './Icon.svelte';
  import './menu.css';
  import { back, closeMenu, menu, menuDisarm, menuHold, menuMoveTo, menuOpener, menuOwns, pick, showSub, trim, type MenuItem } from './menu.svelte';
  import { createIntent, headingToward, type Pt } from './menus/intent.ts';
  import { tooltip } from './tooltip.svelte';
  import { matchTyped } from './select.ts';

  /** The wrapper of every list; it has no box of its own. */
  let root = $state<HTMLElement>();
  /** Lists on screen: all of them side by side, or only the last one (narrow window). */
  const shown = $derived(menu.flyout ? menu.stack.map((list, d) => ({ list, d, key: d })) : menu.stack.slice(-1).map((list) => ({ list, d: menu.stack.length - 1, key: 0 })));
  /** The element key of list `d`: one per list side by side, one in total when swapping. */
  const kd = (d: number) => (menu.flyout ? d : 0);
  /** The lists' boxes, by `kd`. */
  let panels = $state<Record<number, HTMLElement | null>>({});
  /** The items' buttons, by `kd:index`, for the armed one's callout and the keyboard. */
  let rows = $state<Record<string, HTMLElement | null>>({});
  const rowAt = (d: number, i: number) => rows[`${kd(d)}:${i}`] ?? undefined;
  let head = $state<HTMLElement>();
  const armedAt = $derived.by(() => {
    const armed = menu.armed;
    if (!armed) return null;
    const d = menu.stack.findIndex((l) => l.items.includes(armed));
    return d < 0 ? null : { d, i: menu.stack[d].items.indexOf(armed) };
  });
  const armedRow = $derived(armedAt ? rowAt(armedAt.d, armedAt.i) : undefined);

  /** V107 What a menu field said no to, shown under it until the list changes. */
  let fieldError = $state('');
  $effect(() => {
    menu.stack;
    fieldError = '';
  });

  function fieldKey(e: KeyboardEvent, item: MenuItem) {
    if (e.key !== 'Enter' || !item.field) return;
    e.preventDefault();
    const said = item.field.submit((e.currentTarget as HTMLInputElement).value);
    if (!said) return closeMenu();
    fieldError = said;
    // A short window scrolls the menu: keep the message in sight.
    const row = (e.currentTarget as HTMLElement).parentElement;
    const panel = (e.currentTarget as HTMLElement).closest<HTMLElement>('.cx-menu');
    // V107-R22 The message itself: it hangs below the row's box, so the row always looked
    // in sight. Two frames: the menu's height is capped by a ResizeObserver after the first.
    const reveal = () => {
      const said = row?.querySelector('.cx-field-error');
      if (!said || !panel) return;
      const over = said.getBoundingClientRect().bottom - panel.getBoundingClientRect().bottom;
      if (over > 0) panel.scrollTop += over + 6;
    };
    void tick().then(() => requestAnimationFrame(() => requestAnimationFrame(reveal)));
  }

  /** The note shown on hover, when hints are on. */
  const noteOf = (item: MenuItem) => (item.note && settings.v.general.tooltips ? item.note : '');
  /** V108-F55 A label cut off at the menu's width shows in full on hover. */
  function cut(row: HTMLElement | undefined): boolean {
    const text = row?.querySelector<HTMLElement>('.cx-text');
    return !!text && text.scrollWidth > text.clientWidth + 1;
  }

  /** The list the keyboard is in: the one holding the focus, else the newest. */
  function activeDepth(): number {
    const now = document.activeElement;
    for (let d = menu.stack.length - 1; d >= 0; d--) if (now && panels[kd(d)]?.contains(now)) return d;
    return menu.stack.length - 1;
  }

  /** Moves the keyboard focus through the items that can be picked. */
  function move(step: number) {
    const items = [...(panels[kd(activeDepth())]?.querySelectorAll<HTMLElement>('.cx-item:not(:disabled), .cx-menu-field input') ?? [])];
    if (!items.length) return;
    const at = items.indexOf(document.activeElement as HTMLElement);
    const next = at < 0 ? (step > 0 ? 0 : items.length - 1) : Math.min(Math.max(at + step, 0), items.length - 1);
    items[next].focus();
  }

  /** V108-F55 Type-ahead: letters jump to the next item that starts with them. */
  let typed = '';
  let typedAt = 0;
  function typeAhead(ch: string) {
    const items = [...(panels[kd(activeDepth())]?.querySelectorAll<HTMLElement>('.cx-item:not(:disabled):not(.head)') ?? [])];
    const now = Date.now();
    typed = now - typedAt < 700 ? typed + ch : ch;
    typedAt = now;
    const choices = items.map((el) => ({ value: '', label: el.querySelector('.cx-text')?.textContent?.trim() ?? '' }));
    const hit = matchTyped(choices, typed, items.indexOf(document.activeElement as HTMLElement));
    if (hit >= 0) items[hit].focus();
  }

  /** V108-F55 Which item opened each submenu in the swapping layout, so going back lands on it. */
  let opener: number[] = [];
  $effect.pre(() => {
    if (menu.open) opener = [];
  });
  /** After the list changed: the keyboard lands on row `at` (or the first item) of list `d`; the pointer on the menu. */
  function land(d: number, at: number) {
    void tick().then(() => {
      const box = panels[kd(d)];
      if (!box) return;
      if (!byKeyboard()) return box.focus({ preventScroll: true });
      const target = rowAt(d, at);
      if (target?.isConnected && !(target as HTMLButtonElement).disabled) target.focus();
      else box.querySelector<HTMLElement>('.cx-item:not(:disabled):not(.head)')?.focus();
    });
  }
  function goBack() {
    if (menu.stack.length < 2) return;
    const at = opener.pop() ?? 0;
    back();
    land(menu.stack.length - 1, at);
  }

  /** Where list `d + 1` goes: right of list `d`, level with row `i`. */
  function flyAnchor(d: number, i: number): Rect | null {
    const panel = panels[kd(d)];
    const row = rowAt(d, i);
    if (!panel || !row) return null;
    const p = panel.getBoundingClientRect();
    const r = row.getBoundingClientRect();
    // The list's padding and border sit above its first row: the first row lines up with `i`.
    return { left: p.left, right: p.right, top: r.top - 5, bottom: r.bottom + 5 };
  }
  function openSub(d: number, i: number): boolean {
    const anchor = flyAnchor(d, i);
    if (!anchor) return false;
    showSub(d, i, anchor);
    return true;
  }
  /** Closes the lists deeper than `d`; the focus they held goes to the row that opened them. */
  function closeChild(d: number) {
    const child = menu.stack[d + 1];
    if (!child) return;
    const held = panels[kd(d + 1)]?.contains(document.activeElement);
    const from = child.from;
    trim(d);
    if (held) land(d, from);
  }

  function choose(entry: MenuItem, d: number, i: number, viaKey: boolean) {
    if (entry.items && !entry.disabled && !entry.lock) {
      intent.cancel();
      if (menu.flyout) {
        if (!openSub(d, i)) return;
        // The keyboard starts on the first item of the new list.
        if (viaKey) land(d + 1, -1);
        return;
      }
      opener.push(i);
      pick(entry, viaKey);
      land(menu.stack.length - 1, -1);
      return;
    }
    pick(entry, viaKey);
  }

  // V110-U69 Hover: the pointer's last two spots decide whether it is heading for the flyout.
  let at: Pt = { x: 0, y: 0 };
  let leftRow: Pt = { x: 0, y: 0 };
  const intent = createIntent(
    { show: (d, i) => void openSub(d, i), hide: (d) => closeChild(d) },
    { set: (fn, ms) => window.setTimeout(fn, ms), clear: (id) => window.clearTimeout(id) },
  );
  /** Something in a flyout is in use (a typed field, an armed item): it stays. */
  function busyBelow(d: number): boolean {
    if (armedAt && armedAt.d > d) return true;
    const a = document.activeElement;
    return a instanceof HTMLInputElement && menu.stack.some((_, e) => e > d && !!panels[kd(e)]?.contains(a));
  }
  function hover(e: PointerEvent, d: number, i: number, entry: MenuItem) {
    menuMoveTo(entry);
    if (!menu.flyout || e.pointerType === 'touch') return;
    const child = menu.stack[d + 1];
    const box = panels[kd(d + 1)];
    const side = box?.dataset.side === 'left' ? 'left' : 'right';
    intent.enter({
      depth: d,
      index: i,
      sub: !!entry.items && !entry.disabled && !entry.lock,
      open: child ? child.from : null,
      toward: !!box && headingToward(leftRow, at, box.getBoundingClientRect(), side),
      busy: busyBelow(d),
    });
  }
  function focused(d: number, i: number, entry: MenuItem) {
    menuMoveTo(entry);
    // Keyboard onto another row: the flyout of the row it left shuts.
    const child = menu.stack[d + 1];
    if (menu.flyout && child && child.from !== i && byKeyboard()) trim(d);
  }

  // Keys go to the menu while it's open; focus goes back where it was when it closes.
  $effect(() => {
    if (!menu.open || !root) return;
    const release = focusScope(root, { fallback: menuOpener, initial: () => panels[0] ?? null });
    // Escape closes the newest list of a flyout menu only (then the menu), not what it sits on (e.g. a clip).
    // V107-P5 An armed item lets go first.
    const off = layer('menu', () => menuDisarm() || (menu.flyout && menu.stack.length > 1 ? closeChild(menu.stack.length - 2) : closeMenu()));
    return () => {
      intent.cancel();
      off();
      release();
    };
  });

  $effect(() => {
    if (!menu.open) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Element;
      if (!root?.contains(t) && !t.closest?.('[data-menu-confirm]') && !menuOwns(t)) closeMenu();
    };
    const onMove = (e: PointerEvent) => {
      at = { x: e.clientX, y: e.clientY };
      // The last spot on the row that owns an open flyout: the safe triangle starts there.
      if ((e.target as Element | null)?.closest?.('.cx-item.open')) leftRow = at;
    };
    const onKey = (e: KeyboardEvent) => {
      const typing = document.activeElement instanceof HTMLInputElement;
      const steps: Record<string, number> = typing ? { ArrowDown: 1, ArrowUp: -1 } : { ArrowDown: 1, ArrowUp: -1, Home: -999, End: 999 };
      if (e.key in steps) move(steps[e.key]);
      else if (typing) return;
      // V108-F55 Tab leaves the menu: it closes and the focus goes back to what opened it.
      else if (e.key === 'Tab') closeMenu();
      else if (e.key.length === 1 && e.key !== ' ' && !e.ctrlKey && !e.altKey && !e.metaKey) typeAhead(e.key);
      else if (e.key === 'ArrowLeft' && menu.flyout && menu.stack.length > 1) {
        const d = activeDepth();
        if (d < 1) return;
        closeChild(d - 1);
        // Pointer-opened and focus still in the parent: nothing moves, the newest list just shuts.
      } else if (e.key === 'ArrowLeft' && !menu.flyout && menu.stack.length > 1) goBack();
      else if (e.key === 'ArrowRight' && document.activeElement instanceof HTMLElement && document.activeElement.dataset.sub) {
        document.activeElement.click();
      } else return;
      e.preventDefault();
      e.stopPropagation();
    };
    addEventListener('mousedown', onDown, true);
    addEventListener('pointermove', onMove, true);
    addEventListener('keydown', onKey, true);
    addEventListener('blur', closeMenu);
    addEventListener('resize', closeMenu);
    return () => {
      removeEventListener('mousedown', onDown, true);
      removeEventListener('pointermove', onMove, true);
      removeEventListener('keydown', onKey, true);
      removeEventListener('blur', closeMenu);
      removeEventListener('resize', closeMenu);
    };
  });

  /** A button's menu sits just under it; one at the pointer starts right there. */
  const gap = $derived(menu.anchor.bottom > menu.anchor.top ? 4 : 0);
  /** Where list `d` goes. */
  function placed(d: number, anchor: Rect | null) {
    return menu.flyout && d > 0 && anchor ? { anchor, side: 'right' as const, align: 'start' as const, gap: 2, edge: 6 } : { anchor: menu.anchor, align: menu.align, gap, edge: 6 };
  }
  /** V108-F55 Opened from the keyboard: there at once, no scale-in. */
  let keyed = $state(false);
  $effect.pre(() => {
    if (menu.open) keyed = byKeyboard();
  });
</script>

{#if menu.open && menu.stack.length}
  <div class="menus" bind:this={root}>
    {#each shown as { list, d, key } (key)}
      {@const fly = menu.flyout && d > 0}
      <div
        class="menu cx-menu"
        class:float-in={!keyed && !fly}
        class:fly={fly && !byKeyboard()}
        data-shared-menu
        role="menu"
        aria-label={list.title || 'Menu'}
        tabindex="-1"
        bind:this={panels[key]}
        use:floating={placed(d, list.anchor)}
        onpointerenter={() => fly && intent.panel()}
        oncontextmenu={(e) => e.preventDefault()}
      >
        {#if !menu.flyout && d > 0}
          <button class="cx-item head" role="menuitem" onclick={goBack} use:tooltip={{ label: list.title, when: () => cut(head) }} bind:this={head}>
            <Icon name="chevronLeft" size={14} />
            <span class="cx-text">{list.title}</span>
          </button>
          <hr />
        {/if}
        {#each list.items as entry, i (i)}
          {#if entry === 'separator'}
            <hr />
          {:else if entry.field}
            <div class="cx-menu-field">
              <input
                type="text"
                class:bad={!!fieldError}
                aria-label={entry.label}
                aria-invalid={!!fieldError}
                placeholder={entry.field.placeholder}
                spellcheck="false"
                autocomplete="off"
                oninput={() => (fieldError = '')}
                onkeydown={(e) => fieldKey(e, entry)}
              />
              {#if fieldError}<div class="cx-field-error" role="alert">{fieldError}</div>{/if}
            </div>
          {:else}
            {@const isOpen = menu.flyout && menu.stack[d + 1]?.from === i}
            <button
              class="cx-item"
              class:danger={entry.danger}
              class:locked={!!entry.lock}
              class:armed={menu.armed === entry}
              class:open={isOpen}
              aria-label={menu.armed === entry ? entry.confirm : undefined}
              bind:this={rows[`${key}:${i}`]}
              onpointerenter={(e) => hover(e, d, i, entry)}
              onfocus={() => focused(d, i, entry)}
              role={entry.checked === undefined ? 'menuitem' : 'menuitemcheckbox'}
              aria-checked={entry.checked}
              disabled={entry.disabled}
              data-sub={entry.items ? '1' : undefined}
              aria-haspopup={entry.items ? 'menu' : undefined}
              aria-expanded={entry.items ? isOpen : undefined}
              onclick={(e) => choose(entry, d, i, e.detail === 0)}
              use:tooltip={{ label: noteOf(entry) || entry.label, when: () => !!noteOf(entry) || cut(rowAt(d, i)) }}
            >
              {#if entry.swatch}<span class="cx-swatch" style:background={entry.swatch}></span>
              {:else if entry.avatar !== undefined}<Avatar src={entry.avatar} name={entry.label} size={18} />
              {:else if entry.icon}<Icon name={entry.icon} size={14} />
              {:else}<span class="cx-blank"></span>{/if}
              <span class="cx-text">{entry.label}{#if entry.tag}<span class="cx-tag">{entry.tag}</span>{/if}</span>
              {#if entry.hint}<span class="cx-hint">{entry.hint}</span>{/if}
              {#if entry.lock}<GateFlag kind={entry.lock} still />{/if}
              {#if entry.warn}<span class="cx-warn"><Icon name="alert" size={14} /></span>{/if}
              {#if entry.checked}<span class="cx-check"><Icon name="check" size={14} /></span>{/if}
              {#if entry.items}<Icon name="chevronRight" size={14} />{/if}
            </button>
          {/if}
        {/each}
      </div>
    {/each}
  </div>
  {#if menu.armed?.confirm && armedRow}
    {@const armed = menu.armed}
    <ConfirmCallout over anchor={armedRow} label={armed.confirm!} onconfirm={() => pick(armed, true)} ondismiss={menuDisarm} onhold={menuHold} />
  {/if}
{/if}

<style>
  .menus {
    display: contents;
  }
</style>
