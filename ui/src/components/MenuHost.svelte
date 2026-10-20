<!-- Draws the shared menu at the pointer or under its button, kept inside the window
     (V101-04). Arrow keys move through it, Right opens a submenu, Left goes back (V101-30).
     V107-P5 An item with `confirm` shows its callout over itself and waits for a second pick. -->
<script lang="ts">
  import { tick } from 'svelte';

  import { layer } from '../lib/escape';
  import { focusScope } from '../lib/focusScope';
  import { floating } from '../lib/place';
  import { settings } from '../lib/settings.svelte';
  import Avatar from './Avatar.svelte';
  import ConfirmCallout from './ConfirmCallout.svelte';
  import GateFlag from './GateFlag.svelte';
  import Icon from './Icon.svelte';
  import { back, closeMenu, menu, menuDisarm, menuMoveTo, pick, type MenuItem } from './menu.svelte';
  import { tooltip } from './tooltip.svelte';

  let box = $state<HTMLElement>();
  const list = $derived(menu.stack.at(-1));
  /** A button's menu sits just under it; one at the pointer starts right there. */
  const gap = $derived(menu.anchor.bottom > menu.anchor.top ? 4 : 0);
  /** V107-P5 The items' buttons, for the armed one's callout. */
  let rows = $state<HTMLElement[]>([]);
  const armedRow = $derived(menu.armed && list ? rows[list.items.indexOf(menu.armed)] : undefined);

  /** V107 What a menu field said no to, shown under it until the list changes. */
  let fieldError = $state('');
  $effect(() => {
    list;
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
    // V107-R22 The message itself: it hangs below the row's box, so the row always looked
    // in sight. Two frames: the menu's height is capped by a ResizeObserver after the first.
    const reveal = () => {
      const said = row?.querySelector('.field-error');
      if (!said || !box) return;
      const over = said.getBoundingClientRect().bottom - box.getBoundingClientRect().bottom;
      if (over > 0) box.scrollTop += over + 6;
    };
    void tick().then(() => requestAnimationFrame(() => requestAnimationFrame(reveal)));
  }

  /** Moves the keyboard focus through the items that can be picked. */
  function move(step: number) {
    const items = [...(box?.querySelectorAll<HTMLElement>('.item:not(:disabled), .field input') ?? [])];
    if (!items.length) return;
    const at = items.indexOf(document.activeElement as HTMLElement);
    const next = at < 0 ? (step > 0 ? 0 : items.length - 1) : Math.min(Math.max(at + step, 0), items.length - 1);
    items[next].focus();
  }

  // Keys go to the menu while it's open; focus goes back where it was when it closes.
  $effect(() => {
    if (!menu.open || !box) return;
    const release = focusScope(box);
    // Escape closes the menu only, not what it sits on (e.g. a clip).
    // V107-P5 An armed item lets go first.
    const off = layer('menu', () => menuDisarm() || closeMenu());
    return () => {
      off();
      release();
    };
  });

  $effect(() => {
    if (!menu.open) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Element;
      if (!box?.contains(t) && !t.closest?.('[data-menu-confirm]')) closeMenu();
    };
    const onKey = (e: KeyboardEvent) => {
      const typing = document.activeElement instanceof HTMLInputElement;
      const steps: Record<string, number> = typing ? { ArrowDown: 1, ArrowUp: -1 } : { ArrowDown: 1, ArrowUp: -1, Home: -999, End: 999 };
      if (e.key in steps) move(steps[e.key]);
      else if (typing) return;
      else if (e.key === 'ArrowLeft' && menu.stack.length > 1) back();
      else if (e.key === 'ArrowRight' && document.activeElement instanceof HTMLElement && document.activeElement.dataset.sub) {
        document.activeElement.click();
      } else return;
      e.preventDefault();
      e.stopPropagation();
    };
    addEventListener('mousedown', onDown, true);
    addEventListener('keydown', onKey, true);
    addEventListener('blur', closeMenu);
    addEventListener('resize', closeMenu);
    return () => {
      removeEventListener('mousedown', onDown, true);
      removeEventListener('keydown', onKey, true);
      removeEventListener('blur', closeMenu);
      removeEventListener('resize', closeMenu);
    };
  });
</script>

{#if menu.open && list}
  <div
    class="menu"
    data-shared-menu
    role="menu"
    tabindex="-1"
    bind:this={box}
    use:floating={{ anchor: menu.anchor, align: menu.align, gap, edge: 6 }}
    oncontextmenu={(e) => e.preventDefault()}
  >
    {#if menu.stack.length > 1}
      <button class="item head" role="menuitem" onclick={back}>
        <Icon name="chevronLeft" size={14} />
        <span class="text">{list.title}</span>
      </button>
      <hr />
    {/if}
    {#each list.items as entry, i (i)}
      {#if entry === 'separator'}
        <hr />
      {:else if entry.field}
        <div class="field">
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
          {#if fieldError}<div class="field-error" role="alert">{fieldError}</div>{/if}
        </div>
      {:else}
        <button
          class="item"
          class:danger={entry.danger}
          class:locked={!!entry.lock}
          class:armed={menu.armed === entry}
          aria-label={menu.armed === entry ? entry.confirm : undefined}
          bind:this={rows[i]}
          onpointerenter={() => menuMoveTo(entry)}
          onfocus={() => menuMoveTo(entry)}
          role={entry.checked === undefined ? 'menuitem' : 'menuitemcheckbox'}
          aria-checked={entry.checked}
          disabled={entry.disabled}
          data-sub={entry.items ? '1' : undefined}
          onclick={() => pick(entry)}
          use:tooltip={{ label: entry.note && settings.v.general.tooltips ? entry.note : '' }}
        >
          {#if entry.swatch}<span class="swatch" style:background={entry.swatch}></span>
          {:else if entry.avatar !== undefined}<Avatar src={entry.avatar} name={entry.label} size={18} />
          {:else if entry.icon}<Icon name={entry.icon} size={14} />
          {:else}<span class="blank"></span>{/if}
          <span class="text">{entry.label}{#if entry.tag}<span class="tag">{entry.tag}</span>{/if}</span>
          {#if entry.hint}<span class="hint">{entry.hint}</span>{/if}
          {#if entry.lock}<GateFlag kind={entry.lock} still />{/if}
          {#if entry.warn}<span class="warn"><Icon name="alert" size={14} /></span>{/if}
          {#if entry.checked}<span class="check"><Icon name="check" size={14} /></span>{/if}
          {#if entry.items}<Icon name="chevronRight" size={13} />{/if}
        </button>
      {/if}
    {/each}
  </div>
  {#if menu.armed?.confirm && armedRow}
    {@const armed = menu.armed}
    <ConfirmCallout over anchor={armedRow} label={armed.confirm!} onconfirm={() => pick(armed)} ondismiss={menuDisarm} />
  {/if}
{/if}

<style>
  .menu {
    position: fixed;
    z-index: var(--z-menu);
    min-width: 200px;
    max-width: 300px;
    max-height: calc(100vh - 12px);
    overflow-y: auto;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
  }
  .item:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  /* Keyboard focus is a ring, so it never reads as the hovered item. */
  .item:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .menu:focus {
    outline: none;
  }
  .item:disabled {
    opacity: 0.45;
  }
  /* V102-20 Greyed out, but the flag still takes you to the sign-in. */
  .item.locked .text,
  .item.locked .swatch,
  .item.locked > :global(svg) {
    opacity: 0.45;
  }
  .item.danger {
    color: var(--red);
  }
  /* V107-P5 Waiting for the second pick: a red wash under its callout. */
  .item.armed,
  .item.armed:hover:not(:disabled) {
    background: color-mix(in srgb, var(--red) 14%, transparent);
    color: var(--red);
  }
  .head {
    color: var(--muted);
    font-weight: 600;
  }
  .text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* V107-R1 Same look as the Add-tab account picker's "default". */
  .tag {
    margin-left: 6px;
    color: var(--dim);
    font-size: 0.78rem;
  }
  .hint {
    color: var(--dim);
    font-size: 0.76rem;
  }
  .swatch {
    width: 12px;
    height: 12px;
    margin: 0 1px;
    border-radius: 50%;
  }
  .blank {
    width: 14px;
  }
  .check {
    display: flex;
    color: var(--accent-text);
  }
  .warn {
    display: flex;
    color: var(--yellow);
  }
  /* V107 A typed value, e.g. a custom pin length. */
  .field {
    padding: 2px 4px;
  }
  .field input {
    width: 100%;
    height: 28px;
    padding: 0 8px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    font-variant-numeric: tabular-nums;
  }
  .field input:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -1px;
  }
  .field input.bad,
  .field input.bad:focus-visible {
    border-color: var(--red);
    outline-color: var(--red);
  }
  .field-error {
    padding: 4px 4px 2px;
    color: var(--red);
    font-size: 0.76rem;
  }
  hr {
    margin: 4px 0;
    border: 0;
    border-top: 1px solid var(--border);
  }
</style>
