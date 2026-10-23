<!-- V107-Mo The last settings searches, under the search box while it's focused and empty.
     A click or Enter runs one again; × drops it; Clear drops them all. Arrows in the box move
     the pick and Delete drops it (SettingsView), announced through the box's
     aria-activedescendant; mouse-down keeps the focus in the box. Clear is the one Tab stop. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';

  interface Props {
    /** Id of the list; row i is `{id}-{i}` (the box points at it with aria-activedescendant). */
    id: string;
    items: readonly string[];
    /** The one the arrow keys are on, -1 for none. */
    active: number;
    onpick: (query: string) => void;
    onremove: (query: string) => void;
    onclear: () => void;
  }

  let { id, items, active, onpick, onremove, onclear }: Props = $props();

  const keep = (e: MouseEvent) => e.preventDefault();
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="recent" role="group" aria-label="Recent searches" onmousedown={keep}>
  <div class="head">
    <span>Recent</span>
    <button class="clear" onclick={onclear}>Clear</button>
  </div>
  <div role="listbox" {id} aria-label="Recent searches">
    {#each items as q, i (q)}
      <div class="row" class:on={i === active} role="option" id="{id}-{i}" aria-selected={i === active}>
        <button class="pick" tabindex="-1" onclick={() => onpick(q)}><Icon name="search" size={13} /><span>{q}</span></button>
        <button class="x" tabindex="-1" aria-label="Remove {q}" onclick={() => onremove(q)}><Icon name="x" size={12} /></button>
      </div>
    {/each}
  </div>
</div>

<style>
  /* Drawn like an open Select list. */
  .recent {
    position: absolute;
    z-index: var(--z-popover);
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    padding-left: 8px;
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .row {
    display: flex;
    align-items: center;
    border-radius: var(--radius-sm);
  }
  .row:hover,
  .row.on {
    background: var(--row-hover);
  }
  /* The arrow keys' pick, as in Select. */
  .row.on {
    box-shadow: inset 0 0 0 1px var(--focus-ring);
  }
  button {
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--muted);
  }
  .row:hover .pick,
  .row.on .pick,
  button:hover {
    color: var(--text);
  }
  .clear {
    height: 32px;
    padding: 0 8px;
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .clear:hover {
    background: var(--row-hover);
  }
  .pick {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 32px;
    padding: 0 8px;
    text-align: left;
  }
  .pick > :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .pick span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* 32 px to hit, drawn 28. */
  .x {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 32px;
    height: 32px;
    color: var(--dim);
  }
  .x::before {
    content: '';
    position: absolute;
    inset: 2px;
    border-radius: var(--radius-sm);
  }
  .x:hover::before {
    background: var(--row-hover);
  }
  .x > :global(svg) {
    position: relative;
  }
</style>
