<!-- V106-03 The custom themes: one trigger, a menu under it. Picking keeps the menu open; New closes it. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
  import { scrollBehavior } from '../../lib/motion';
  import { scrollWithin } from '../../lib/scrollWithin';
  import { PLAIN_PREVIEW } from './themes';

  interface Props {
    themes: CustomTheme[];
    /** The theme in use (`appearance.themeId`); may be a built-in or empty. */
    activeId: string;
    onpick: (id: string) => void;
    onnew: () => Promise<void>;
    /** V106 ra-3 The accent as the UI paints it for that theme. */
    fill: (t: CustomTheme) => string;
  }

  let { themes, activeId, onpick, onnew, fill }: Props = $props();

  let open = $state(false);

  const active = $derived(themes.find((t) => t.id === activeId) ?? null);
  const bg = (t: CustomTheme) => t.colors.bg ?? PLAIN_PREVIEW[t.base].bg;

  /** Set after New; the editor appears once the new theme is the active one. */
  let pendingFrom: string | null = null;
  async function create() {
    open = false;
    pendingFrom = activeId;
    await onnew();
  }
  $effect(() => {
    if (pendingFrom === null || !active || active.id === pendingFrom) return;
    pendingFrom = null;
    void tick().then(() => {
      const editor = document.querySelector<HTMLElement>('[data-theme-editor]');
      if (editor) scrollWithin(editor, 'nearest', scrollBehavior());
    });
  });
</script>

<div class="wrap">
  <button class="field pick" class:on={!!active} aria-label={active ? `Custom themes: ${active.name}` : undefined}
    aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>
    {#if active}
      <span class="sw small" style:background={bg(active)}><u style:background={fill(active)}></u></span>
    {/if}
    <span class="name">{active ? active.name : 'Custom themes'}</span>
    <Icon name={open ? 'chevronUp' : 'chevronDown'} size={14} />
  </button>
  <Popover bind:open align="start" role="menu" label="Custom themes">
    <div class="panel">
      {#if themes.length}
        <div class="list">
          {#each themes as t (t.id)}
            {@const on = t.id === activeId}
            <button class="row" class:on role="menuitemradio" aria-checked={on} onclick={() => onpick(t.id)}>
              <span class="sw" style:background={bg(t)}><u style:background={fill(t)}></u></span>
              <span class="name">{t.name}</span>
              <span class="tick">{#if on}<Icon name="check" size={14} />{/if}</span>
            </button>
          {/each}
        </div>
        <div class="sep" role="separator"></div>
      {/if}
      <button class="row" role="menuitem" onclick={() => void create()}>
        <Icon name="plus" size={14} />
        <span class="name">New</span>
      </button>
    </div>
  </Popover>
</div>

<style>
  .wrap {
    position: relative;
    flex: none;
  }
  .pick {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    /* V106 a-3 A fixed width: picking a theme with a longer name doesn't move the row. */
    width: 180px;
    color: var(--text);
    cursor: pointer;
  }
  .pick:hover {
    border-color: var(--dim);
  }
  /* V106 s2-1 Text-colored edge, so it shows with any accent. */
  .pick.on {
    border-color: var(--text);
  }
  .pick :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .panel {
    display: flex;
    flex-direction: column;
    min-width: 220px;
    max-width: 320px;
  }
  .list {
    max-height: 300px;
    overflow-y: auto;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 10px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.95rem;
    text-align: left;
  }
  .row.on {
    color: var(--text);
  }
  .row:hover,
  .row:focus-visible {
    background: var(--surface-hover);
  }
  /* V106 ra-2 The ring sits inside the row: the scroller is as wide as the rows and clips it. */
  .row:focus-visible {
    outline-offset: -2px;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tick {
    flex: none;
    width: 14px;
    height: 14px;
  }
  .tick :global(svg) {
    color: var(--accent-text);
  }
  .sw {
    position: relative;
    flex: none;
    width: 22px;
    height: 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .sw.small {
    width: 16px;
    height: 12px;
  }
  .sw u {
    position: absolute;
    inset: 0 0 0 auto;
    width: 5px;
  }
  .sw.small u {
    width: 4px;
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }
</style>
