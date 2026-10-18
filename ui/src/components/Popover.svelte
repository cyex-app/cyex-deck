<!-- Dropdown under (or above) its trigger. Put it next to the trigger inside a
     wrapper; clicks outside that wrapper and Escape close it. Flips to the other side and
     stays inside the window when there's no room (V101-04); fixed to the window, so a
     scrolling or clipped panel around it can't cut it off, and shorter when even that
     isn't enough (V102-03).
     Focus moves in on open and back to the trigger on close. `role="menu"` when the content
     is a list of actions: arrows, Home and End move through its buttons. Otherwise it's a
     non-modal dialog and Tab moves through it. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { layer } from '../lib/escape';
  import { focusScope, tabbables } from '../lib/focusScope';
  import { floating } from '../lib/place';
  import { moveFor } from './select.ts';

  interface Props {
    open: boolean;
    align?: 'start' | 'end';
    side?: 'below' | 'above';
    role?: 'menu' | 'dialog';
    /** What screen readers call it, e.g. "Notifications". */
    label?: string;
    children: Snippet;
  }

  let { open = $bindable(), align = 'end', side = 'below', role = 'dialog', label, children }: Props = $props();

  let box = $state<HTMLElement>();

  const ITEMS = '[role^="menuitem"]:not([aria-disabled="true"]), button:not(:disabled)';

  function items(): HTMLElement[] {
    return [...(box?.querySelectorAll<HTMLElement>(ITEMS) ?? [])].filter((el) => el.getClientRects().length > 0);
  }

  $effect(() => {
    if (!open || !box) return;
    const node = box;
    const wrapper = node.parentElement;
    // V8 Opened by mouse, a panel takes focus itself (no ring on its first tab); a text field
    // still gets it, and the keyboard goes to the first stop.
    const byKey = document.activeElement?.matches(':focus-visible') ?? false;
    const release = focusScope(node, {
      initial: () => {
        // A menu with a checked choice opens on it, so the keyboard starts where you are.
        if (role === 'menu') return items().find((el) => el.getAttribute('aria-checked') === 'true') ?? items()[0];
        const first = tabbables(node)[0];
        return byKey || first?.matches('input, textarea, select, [role="combobox"]') ? first : node;
      },
    });
    const off = layer(role === 'menu' ? 'menu' : 'popover', () => (open = false));
    // The shared menu, opened from inside, counts as inside.
    const onDown = (e: MouseEvent) => {
      const t = e.target as Element;
      if (!wrapper?.contains(t) && !t.closest?.('[data-shared-menu]')) open = false;
    };
    // c-2 Leaving the window keeps a typed name (a `data-draft` field, e.g. the picker's rename bar).
    const onBlur = () => {
      if (!node.querySelector('[data-draft]')) open = false;
    };
    addEventListener('mousedown', onDown, true);
    addEventListener('blur', onBlur);
    return () => {
      off();
      release();
      removeEventListener('mousedown', onDown, true);
      removeEventListener('blur', onBlur);
    };
  });

  function onkeydown(e: KeyboardEvent) {
    if (role !== 'menu' || e.ctrlKey || e.altKey || e.metaKey) return;
    const list = items();
    const at = list.indexOf(document.activeElement as HTMLElement);
    const to = at < 0 && (e.key === 'ArrowDown' || e.key === 'ArrowUp')
      ? (e.key === 'ArrowDown' ? 0 : list.length - 1)
      : moveFor(e.key, at, list.length);
    if (to === null || to < 0) return;
    e.preventDefault();
    list[to]?.focus();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="popover"
    bind:this={box}
    use:floating={{ anchor: box?.parentElement, side, align }}
    {role}
    aria-label={label}
    tabindex="-1"
    {onkeydown}
  >
    {@render children()}
  </div>
{/if}

<style>
  .popover {
    position: fixed;
    z-index: var(--z-popover);
    display: flex;
    flex-direction: column;
    min-width: 220px;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
  }
  .popover:focus {
    outline: none;
  }
  /* Shorter content scrolls inside instead of the whole box. */
  .popover > :global(*) {
    min-height: 0;
  }
</style>
