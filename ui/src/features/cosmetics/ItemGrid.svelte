<!-- V1051-03 Paints or badges in a windowed grid; the first cell is None.
     V109-B05 Also the cyex badges: any row of that shape.
     V109-B08 `rows`: compact row cards (dot, 28 px picture, name and a second line), as many
     columns of 190 px or more as fit; the pick is a tint and a filled dot, nothing scales. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import VirtualGrid from '../../components/VirtualGrid.svelte';
  import { density } from '../../lib/density.svelte';
  import { focus } from '../../lib/focus.svelte';
  import { MIN_PAINT_CELL, measureName, paintCellWidth } from '../../lib/paintCell';
  import { settings } from '../../lib/settings.svelte';
  import { activeIndex } from './activeCell';
  import type { BadgeRow, PaintRow } from './cosmetics.svelte';
  import { badgePicture } from './pictures';

  /** V109-B05 What a badge cell reads; 7TV's rows and the cyex picker's both have it. */
  type GridBadge = Pick<BadgeRow, 'name' | 'animated' | 'usable'> & {
    /** V109-B08 The second line of a row card, e.g. `#37 · One-time`. */
    sub?: string;
    item: { def: { id: string }; images: readonly string[]; stills: readonly string[] | null };
  };
  type Row = PaintRow | GridBadge;

  interface Props {
    kind: 'paint' | 'badge';
    rows: readonly Row[];
    selected: string | null;
    /** The account's display name, drawn in each paint. */
    name: string;
    label: string;
    /** The grid's width, for the header above it. */
    box?: number;
    /** `usable` is false for subscriber-only items. */
    onpick: (id: string | null, usable: boolean) => void;
    /** V109-B05 Badges one per row, picture and name side by side. */
    asRows?: boolean;
  }

  let { kind, rows, selected, name, label, box = $bindable(0), onpick, asRows = false }: Props = $props();

  // V1051-03 Wide enough for the name in its paint, once per name and chat font; re-read when
  // a font finishes loading.
  let fontTick = $state(0);
  $effect(() => {
    const bump = () => fontTick++;
    document.fonts.addEventListener('loadingdone', bump);
    return () => document.fonts.removeEventListener('loadingdone', bump);
  });
  const paintW = $derived.by(() => {
    void fontTick;
    void settings.v.general.chatFontFamily;
    return kind === 'paint' ? paintCellWidth(measureName(name)) : MIN_PAINT_CELL;
  });

  /** V109-B08 Row cards: at least this wide, and this tall. */
  const ROW_W = 190;
  const ROW_H = 44;
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

  /** "7TV Subscriber - 1 Year, 6 Months" → ["Subscriber", "1 Year, 6 Months"]: in a badge
   *  cell the part that tells badges apart gets its own line instead of being cut off. */
  function badgeLines(name: string): [string, string] | null {
    // "CS2 Event Viewer (10 Hours)" splits the same way, so "10 Hours" isn't the part that's cut.
    const m = /^(.+?)\s+[-–—]\s+(.+)$/.exec(name) ?? /^(.+?)\s+\((.+)\)$/.exec(name);
    // Everything here is 7TV's, so "7TV Subscriber" reads "Subscriber" and fits the cell.
    return m ? [m[1].replace(/^7TV\s+/i, ''), m[2]] : null;
  }

  /** V107-R5 The hovered paint's name is cut off by its ellipsis. */
  function paintCut(): boolean {
    const nm = document.querySelector<HTMLElement>('.item.paint:hover .nm');
    return !!nm && nm.scrollWidth > nm.clientWidth;
  }

  /** V109-B13 A row card's name cut by its ellipsis: the tip carries the whole of it. */
  function rowTip(r: Row | null) {
    return {
      label: r ? r.name : 'None',
      when: () => {
        const rn = document.querySelector<HTMLElement>('.row:hover .rn');
        return !!rn && rn.scrollWidth > rn.clientWidth;
      },
    };
  }

  function tip(r: Row | null) {
    if (!r) return { label: 'None' };
    return {
      label: r.animated ? `${r.name} · Animated` : r.name,
      hint: r.usable ? '' : 'For 7TV subscribers',
      // The name is on the card already; the tip adds only what isn't, or the whole of a long one.
      when: () => r.animated || !r.usable || (kind === 'badge' && r.name.length > 12) || (kind === 'paint' && paintCut()),
    };
  }
</script>

<VirtualGrid
  {items}
  key={keyOf}
  {label}
  bind:box
  stretch
  flat={asRows}
  chosen={(r) => keyOf(r) === (selected ?? NONE_KEY)}
  cellWidth={kind === 'paint' ? paintW : asRows ? ROW_W : 72}
  cellHeight={kind === 'paint' ? 48 : asRows ? ROW_H : 76}
  bind:active={() => active, (i) => (moved = keyOf(items[i]))}
  onpick={(i) => {
    const r = items[i];
    onpick(r ? idOf(r) : null, r ? r.usable : true);
  }}
>
  {#snippet cell(r)}
    {@const on = (r ? idOf(r) : null) === selected}
    {#if asRows}
      <div class="row" class:on use:tooltip={rowTip(r)}>
        <span class="dot" aria-hidden="true"></span>
        {#if r && 'def' in r.item}
          <img src={badgePicture(r.item.images, r.item.stills, 28, dpr, focus.idle)} width="28" height="28" alt="" decoding="async" />
        {:else}
          <span class="none"><Icon name="ban" size={18} /></span>
        {/if}
        <span class="txt">
          <span class="rn">{r ? r.name : 'None'}</span>
          {#if r && 'sub' in r && r.sub}<span class="rs">{r.sub}</span>{:else if !r}<span class="rs">No badge</span>{/if}
        </span>
      </div>
    {:else}
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
        {@const lines = badgeLines(r.name)}
        {#if lines && lines[1].length > 10}
          <!-- Too long for one line under the shared start: the telling part alone, on two. -->
          <span class="nm two">{lines[1]}</span>
        {:else if lines}
          <span class="nm lines"><span>{lines[0]}</span><span>{lines[1]}</span></span>
        {:else}
          <span class="nm two">{r.name}</span>
        {/if}
      {:else}
        <span class="none"><Icon name="ban" size={20} /></span>
        <span class="nm two">None</span>
      {/if}
      {#if on}<span class="check"><Icon name="check" size={12} /></span>{/if}
    </div>
    {/if}
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
    border: 2px solid var(--accent-line);
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
    font-family: var(--chat-font);
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
  .nm.lines {
    display: flex;
    flex-direction: column;
    max-width: 100%;
    line-height: 1.2;
  }
  .nm.lines span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  /* V109-B08 Row cards. */
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 0 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition:
      background var(--fast),
      border-color var(--fast);
  }
  .row:hover {
    background: var(--row-hover);
    border-color: var(--border-strong);
  }
  .row.on {
    background: var(--accent-dim);
    border-color: var(--accent-border);
  }
  .dot {
    flex: none;
    box-sizing: border-box;
    width: 14px;
    height: 14px;
    border: 1px solid var(--border-strong);
    border-radius: 50%;
    transition:
      border-color var(--fast),
      box-shadow var(--fast);
  }
  .row.on .dot {
    border: 4px solid var(--accent-text);
  }
  .row img,
  .row .none {
    flex: none;
    width: 28px;
    height: 28px;
  }
  .txt {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.25;
  }
  .rn {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row.on .rn {
    color: var(--accent-text);
  }
  .rs {
    overflow: hidden;
    color: var(--dim);
    font-size: var(--text-hint);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (forced-colors: active) {
    .row.on .dot {
      border-color: Highlight;
    }
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
