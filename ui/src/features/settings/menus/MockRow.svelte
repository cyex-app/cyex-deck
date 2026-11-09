<!-- V110-U67 One row of the menu mock in Settings › Menus. It looks like the real row (shared
     cx-item styles) with the manager's extras: a dim tag for conditions, eye-off when hidden,
     a dashed outline for the slots, an insertion line while something is dragged over it. -->
<script lang="ts">
  import { enterKey } from '../../../lib/keys';
  import Icon from '../../../components/Icon.svelte';
  import { tooltip } from '../../../components/tooltip.svelte';
  import { MAX_LABEL } from '../../../components/menus/ops.ts';
  import type { RowView } from './model.ts';

  interface Props {
    row: RowView;
    depth: 1 | 2;
    /** The one row in the tab order. */
    stop: boolean;
    selected: boolean;
    /** A group whose flyout is drawn. */
    open: boolean;
    /** The insertion line while dragging. */
    line: 'before' | 'after' | null;
    /** A dragged row would go into this group. */
    into: boolean;
    dragging: boolean;
    renaming: boolean;
    onpress: (e: PointerEvent) => void;
    onselect: () => void;
    onstartrename: () => void;
    /** `null`: Escape. */
    onrename: (label: string | null) => void;
    onmore: (el: HTMLElement) => void;
  }

  let { row, depth, stop, selected, open, line, into, dragging, renaming, onpress, onselect, onstartrename, onrename, onmore }: Props =
    $props();

  let el = $state<HTMLElement>();

  const aria = $derived(
    row.kind === 'sep'
      ? 'Line'
      : [row.label, row.tag, row.hidden ? 'hidden' : '', row.absent ? 'not in this preview' : '', row.slot ? 'filled in by the app' : '']
          .filter(Boolean)
          .join(', '),
  );

  function focusSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function renameKey(e: KeyboardEvent) {
    e.stopPropagation();
    if (enterKey(e)) {
      e.preventDefault();
      onrename((e.currentTarget as HTMLInputElement).value);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onrename(null);
    }
  }

  function down(e: PointerEvent) {
    if (renaming || (e.target as HTMLElement).closest('.more, input')) return;
    onpress(e);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events (the tree handles keys) -->
<div
  bind:this={el}
  class="cx-item row"
  class:sep={row.kind === 'sep'}
  class:slot={row.slot}
  class:hidden={row.hidden}
  class:absent={row.absent}
  class:danger={row.danger && !row.hidden}
  class:open
  class:selected
  class:into
  class:dragging
  class:before={line === 'before'}
  class:after={line === 'after'}
  role="treeitem"
  aria-level={depth}
  aria-selected={selected}
  aria-expanded={row.kind === 'group' ? open : undefined}
  aria-label={aria}
  tabindex={stop ? 0 : -1}
  data-key={row.key}
  data-kind={row.kind}
  data-index={row.path[row.path.length - 1]}
  onpointerdown={down}
  onclick={onselect}
  ondblclick={() => row.kind !== 'sep' && !renaming && onstartrename()}
  oncontextmenu={(e) => {
    e.preventDefault();
    if (el) onmore(el);
  }}
>
  {#if row.kind === 'sep'}
    <hr />
  {:else}
    {#if row.icon}<Icon name={row.icon} size={14} />{:else}<span class="cx-blank"></span>{/if}
    {#if renaming}
      <input
        class="rename"
        value={row.label}
        maxlength={MAX_LABEL}
        placeholder={row.builtIn}
        aria-label="Name of {row.builtIn}"
        spellcheck="false"
        autocomplete="off"
        use:focusSelect
        onkeydown={renameKey}
        onblur={(e) => onrename(e.currentTarget.value)}
        onpointerdown={(e) => e.stopPropagation()}
        onclick={(e) => e.stopPropagation()}
        ondblclick={(e) => e.stopPropagation()}
      />
    {:else}
      <span class="cx-text">{row.label}{#if row.tag}<span class="cx-tag">{row.tag}</span>{/if}</span>
    {/if}
    {#if row.renamed && !renaming}<span class="cx-hint was" use:tooltip={{ label: `Was “${row.builtIn}”` }}>renamed</span>{/if}
    {#if row.hint}<span class="cx-hint">{row.hint}</span>{/if}
    {#if row.checked}<span class="cx-check"><Icon name="check" size={14} /></span>{/if}
    {#if row.hidden}<span class="eye" aria-hidden="true"><Icon name="eyeOff" size={14} /></span>{/if}
    {#if row.kind === 'group'}<span class="cx-hint">{row.count}</span>{/if}
    {#if row.sub}<Icon name="chevronRight" size={14} />{/if}
    <button
      class="more"
      tabindex="-1"
      aria-label="Options for {row.label}"
      use:tooltip={{ label: 'Options' }}
      onpointerdown={(e) => e.stopPropagation()}
      onclick={(e) => {
        e.stopPropagation();
        if (el) onmore(el);
      }}><Icon name="more" size={14} /></button
    >
  {/if}
</div>

<style>
  .row {
    position: relative;
    cursor: default;
    user-select: none;
    touch-action: none;
  }
  .row.selected {
    background: var(--accent-dim);
  }
  .row.selected:hover {
    background: var(--accent-wash);
  }
  .row:active:not(.dragging) {
    background: color-mix(in srgb, var(--text) 10%, var(--surface-hover));
  }
  .row.sep {
    height: 11px;
    padding: 0 8px;
  }
  .row.sep hr {
    flex: 1;
    margin: 0;
  }
  .row.sep.selected {
    background: var(--accent-dim);
  }
  /* Rows the people previewed would not see, and rows switched off. */
  .row.absent .cx-text,
  .row.absent > :global(svg) {
    opacity: 0.5;
  }
  .row.hidden .cx-text,
  .row.hidden > :global(svg) {
    opacity: 0.45;
  }
  .row.hidden .cx-text {
    text-decoration: line-through;
    text-decoration-color: var(--border-strong);
  }
  .eye {
    display: flex;
    color: var(--dim);
  }
  /* Where other code fills in rows: one dashed line, never renamed. */
  .row.slot {
    margin: 2px 0;
    height: 28px;
    border: 1px dashed var(--border-strong);
    color: var(--muted);
  }
  .row.slot .cx-text {
    font-style: italic;
  }
  .was {
    font-style: italic;
  }
  .more {
    display: none;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    margin-right: -4px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .row:hover .more,
  .row.selected .more,
  .row:focus-visible .more,
  .more:focus-visible {
    display: flex;
  }
  .more:hover {
    background: var(--elevated-2);
    color: var(--text);
  }
  .rename {
    flex: 1;
    min-width: 0;
    height: 24px;
    padding: 0 6px;
    background: var(--surface);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius);
    color: var(--text);
    font: inherit;
  }
  .rename:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -1px;
  }
  /* While a row is dragged: it stays as a faint copy; the line shows where it lands. */
  .row.dragging {
    opacity: 0.4;
  }
  .row.before::before,
  .row.after::after {
    content: '';
    position: absolute;
    left: 4px;
    right: 4px;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
    pointer-events: none;
  }
  .row.before::before {
    top: -2px;
  }
  .row.after::after {
    bottom: -2px;
  }
  .row.into {
    background: var(--accent-wash);
    outline: 1px solid var(--accent-border);
    outline-offset: -1px;
  }
</style>
