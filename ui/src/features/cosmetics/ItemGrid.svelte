<!-- V1051-03 Paints or badges in a windowed grid; the first cell is None. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import VirtualGrid from '../../components/VirtualGrid.svelte';
  import { density } from '../../lib/density.svelte';
  import { focus } from '../../lib/focus.svelte';
  import { activeIndex } from './activeCell';
  import type { BadgeRow, PaintRow } from './cosmetics.svelte';
  import { badgePicture } from './pictures';

  type Row = PaintRow | BadgeRow;

  interface Props {
    kind: 'paint' | 'badge';
    rows: readonly Row[];
    selected: string | null;
    /** The account's display name, drawn in each paint. */
    name: string;
    label: string;
    /** `usable` is false for subscriber-only items. */
    onpick: (id: string | null, usable: boolean) => void;
  }

  let { kind, rows, selected, name, label, onpick }: Props = $props();

  const NONE = null;
  const items = $derived<(Row | null)[]>([NONE, ...rows]);
  const idOf = (r: Row) => ('style' in r.item ? r.item.style.id : r.item.def.id);
  // V105-08 Re-read when the window moves to another screen (density watches it).
  const dpr = $derived(density.dpr());
  const NONE_KEY = '__none';
  const keyOf = (r: Row | null) => (r ? idOf(r) : NONE_KEY);
  // By id, so a filter doesn't move it; it starts on the worn one (V1051-03).
  let moved = $state<string>();
  const active = $derived(activeIndex(items.map(keyOf), moved, selected ?? NONE_KEY));

  function tip(r: Row | null) {
    if (!r) return { label: 'None' };
    return {
      label: r.animated ? `${r.name} · Animated` : r.name,
      hint: r.usable ? '' : 'For 7TV subscribers',
      // The name is on the card already; the tip adds only what isn't.
      when: () => r.animated || !r.usable,
    };
  }
</script>

<VirtualGrid
  {items}
  key={keyOf}
  {label}
  cellWidth={kind === 'paint' ? 176 : 72}
  cellHeight={kind === 'paint' ? 48 : 76}
  bind:active={() => active, (i) => (moved = keyOf(items[i]))}
  onpick={(i) => {
    const r = items[i];
    onpick(r ? idOf(r) : null, r ? r.usable : true);
  }}
>
  {#snippet cell(r)}
    {@const on = (r ? idOf(r) : null) === selected}
    <div class="item {kind}" class:on class:lock={r && !r.usable} aria-disabled={r && !r.usable ? 'true' : undefined} use:tooltip={tip(r)}>
      {#if kind === 'paint'}
        {#if r}
          <span class="back"><span class="painted paint-{idOf(r)} who">{name}</span></span>
          <span class="nm">{r.name}</span>
        {:else}
          <span class="back"><span class="who">{name}</span></span>
          <span class="nm">None</span>
        {/if}
      {:else if r && 'def' in r.item}
        <img
          src={badgePicture(r.item.images, r.item.stills, 32, dpr, focus.idle)}
          width="32"
          height="32"
          alt=""
          loading="lazy"
          decoding="async"
        />
        <span class="nm two">{r.name}</span>
      {:else}
        <span class="none">–</span>
        <span class="nm two">None</span>
      {/if}
      {#if on}<span class="check"><Icon name="check" size={12} /></span>{/if}
    </div>
  {/snippet}
</VirtualGrid>

<style>
  .item {
    position: relative;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 6px 8px;
    overflow: hidden;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: border-color var(--fast);
  }
  .item:hover {
    border-color: var(--border-strong);
  }
  .item.on {
    border: 2px solid var(--accent);
    padding: 5px 7px;
  }
  .item.lock {
    opacity: 0.4;
  }
  .paint {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
  }
  .badge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px 4px;
    text-align: center;
  }
  .who {
    padding-block: 0.25em;
    margin-block: -0.25em;
    min-width: 0;
    overflow: hidden;
    font-size: 14px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* V1051-03 Light: pale paints sit on a soft grey, not on white. The paint is the name's own
     background, so the grey goes on a wrapper. */
  .back {
    display: flex;
    align-self: flex-start;
    max-width: 100%;
    padding: 0 6px;
    background: var(--paint-back);
    border-radius: var(--radius);
  }
  .nm {
    overflow: hidden;
    color: var(--dim);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
    /* Room for descenders under the clip. */
    padding-bottom: 2px;
    margin-bottom: -2px;
  }
  .nm.two {
    display: -webkit-box;
    line-height: 1.2;
    white-space: normal;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
  .none {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    color: var(--dim);
  }
  img {
    flex: none;
    object-fit: contain;
  }
  .check {
    position: absolute;
    top: 3px;
    right: 3px;
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    background: var(--accent);
    border-radius: 50%;
    color: var(--accent-ink);
  }
</style>
