<!-- V107-R29 The closed panels as a row of icons under the layout, locked or not. A click
     opens that panel in a small menu above its icon (Esc or a click outside closes it), with
     a pop-out button in its header. Unlocked, an icon drags onto a side of any panel, as from
     the tray (V103-06); closed panels without an icon wait in the tray's chip then. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { drag } from '../features/tabs/drag.svelte';
  import { clicked, splitDock, stillOpen } from './dock';
  import Icon from './Icon.svelte';
  import type { TrayItem } from './panel';
  import PanelBox from './PanelBox.svelte';
  import PanelTray from './PanelTray.svelte';
  import Popover from './Popover.svelte';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    /** The layout's key. */
    layout: string;
    items: TrayItem[];
    unlocked: boolean;
    /** A panel's content for its menu. */
    quick: Snippet<[string]>;
    onpopout?: (id: string) => void;
    onrestore: (id: string) => void;
  }

  let { layout, items, unlocked, quick, onpopout, onrestore }: Props = $props();

  const split = $derived(splitDock(items));
  let open = $state<string | null>(null);
  /** A press that turned into a drag isn't a click. */
  let dragged = false;

  $effect(() => {
    const next = stillOpen(open, split.icons.map((i) => i.id));
    if (next !== open) open = next;
  });

  function down(e: PointerEvent, item: TrayItem) {
    dragged = false;
    if (!unlocked) return;
    drag.begin(e, { kind: 'panel', layout, panel: item.id, tray: true }, item.title);
    const watch = () => {
      if (drag.item) {
        dragged = true;
        open = null;
      }
    };
    const end = (ev: Event) => {
      removeEventListener('pointermove', watch);
      removeEventListener('pointerup', end);
      removeEventListener('pointercancel', end);
      removeEventListener('blur', end);
      if (ev.type === 'pointerup') setTimeout(() => (dragged = false));
      else dragged = false;
    };
    addEventListener('pointermove', watch);
    addEventListener('pointerup', end);
    addEventListener('pointercancel', end);
    addEventListener('blur', end);
  }

  function popout(id: string) {
    open = null;
    onpopout?.(id);
  }
</script>

{#if split.icons.length || (unlocked && split.rest.length)}
  <div class="dock">
    {#each split.icons as item (item.id)}
      <div class="wrap">
        <button
          class="icon"
          class:open={open === item.id}
          class:grab={unlocked}
          aria-label={item.title}
          aria-haspopup="dialog"
          aria-expanded={open === item.id}
          use:tooltip={{ label: item.title, hint: unlocked ? 'Drag it onto a panel to put it back.' : item.hint }}
          onpointerdown={(e) => down(e, item)}
          onclick={() => (open = clicked(open, item.id, dragged))}
        >
          {#if item.icon}<Icon name={item.icon} size={15} />{/if}
        </button>
        <Popover
          bind:open={() => open === item.id, (v) => !v && open === item.id && (open = null)}
          side="above"
          label={item.title}
        >
          <div class="quick">
            <PanelBox ctl={{ unlocked: false, quick: true, popout: onpopout ? () => popout(item.id) : undefined }}>
              {@render quick(item.id)}
            </PanelBox>
          </div>
        </Popover>
      </div>
    {/each}
    {#if unlocked && split.rest.length}
      <PanelTray {layout} items={split.rest} {onrestore} inline />
    {/if}
  </div>
{/if}

<style>
  .dock {
    flex: none;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 2px;
    padding: 4px 8px;
  }
  .wrap {
    position: relative;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--muted);
    transition:
      background var(--fast),
      color var(--fast);
  }
  .icon:hover,
  .icon.open {
    background: var(--surface-hover);
    color: var(--text);
  }
  .icon.open {
    border-color: var(--border-strong);
  }
  /* Feedback on press, not on release. */
  .icon:active {
    background: var(--accent-wash);
    color: var(--text);
  }
  .icon.grab {
    cursor: grab;
  }
  .quick {
    display: flex;
    width: min(360px, calc(100vw - 32px));
    height: min(440px, calc(100vh - 96px));
    min-height: 0;
  }
  /* The menu is the frame. */
  .quick :global(.card) {
    border: 0;
  }
</style>
