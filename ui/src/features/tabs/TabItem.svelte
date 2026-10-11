<!-- One channel tab (1.10): name or alias (1.22), live dot (1.29), close button (1.25),
     lit by unread chat (U9). Double click edits (1.18), middle click closes, drag reorders (U5).
     The tab is the whole cell of its row, up to the window's top edge (V101-19); the pill
     inside is what shows. A thin divider sits between two plain tabs (V101-11). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import { accounts } from '../../lib/accounts.svelte';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { drag } from './drag.svelte';
  import { label, tabs, tiles } from './tabs.svelte';

  interface Props {
    tab: Tab;
    active: boolean;
    selected: boolean;
    vertical: boolean;
    /** Color of the tab's group, if any. */
    groupColor?: string;
    onselect: (e: MouseEvent) => void;
    onedit: () => void;
    onclose: () => void;
    onmenu: (e: MouseEvent) => void;
  }

  let { tab, active, selected, vertical, groupColor, onselect, onedit, onclose, onmenu }: Props = $props();

  const name = $derived(label(tab));
  const live = $derived(tabs.liveState(tab));
  const unread = $derived(tabs.unreadState(tab));
  const popped = $derived(tabs.popped.has(tab.id));
  /** V102-15 Still playing while something else shows; crossed out when every stream in it is muted. */
  const held = $derived(tabs.held === tab.id);
  const heldMuted = $derived(held && tiles(tab).filter((t) => t.show.player).every((t) => t.muted));
  const closeButton = $derived(settings.v.appearance.tabCloseButton);

  /** Drop marker while something is dragged over this tab. */
  const mark = $derived.by(() => {
    const t = drag.target;
    if (!drag.item || t?.kind !== 'tab' || t.id !== tab.id) return null;
    if (drag.item.kind !== 'tab') return 'into';
    if (drag.item.id === tab.id) return null;
    return t.after ? 'after' : 'before';
  });

  const tip = $derived.by(() => {
    const parts: string[] = [];
    let hint = '';
    for (const tile of tiles(tab)) {
      const stream = tabs.live[tile.channel.id]?.stream;
      if (stream) {
        const kind = stream.rerun ? 'Rerun' : 'Live';
        parts.push(`${tile.channel.name}: ${kind}, ${formatNumber(stream.viewers)} watching`);
        hint ||= stream.game ? `${stream.title} · ${stream.game}` : stream.title;
      }
    }
    const first = tiles(tab)[0];
    const account = first?.account ? accounts.list.find((a) => a.userId === first.account) : undefined;
    const who = account ? `as ${account.displayName}` : 'anonymous';
    const label = parts.length ? parts.join(' · ') : `${name} · offline`;
    const playing = heldMuted ? ' · playing muted' : held ? ' · playing' : '';
    return { label: `${label}${playing}`, hint: hint ? `${hint} (${who})` : who };
  });

  function onauxclick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault();
      onclose();
    }
  }
</script>

<div
  class="tab"
  class:active
  class:selected
  class:vertical
  class:grouped={!!groupColor}
  class:message={unread === 'message'}
  class:mention={unread === 'mention'}
  data-tab-id={tab.id}
  data-vertical={vertical ? '1' : undefined}
  data-mark={mark}
  style:--group={groupColor}
  role="tab"
  tabindex="0"
  aria-selected={active}
  onclick={onselect}
  ondblclick={onedit}
  {onauxclick}
  onmousedown={(e) => e.button === 1 && e.preventDefault()}
  oncontextmenu={(e) => {
    e.preventDefault();
    onmenu(e);
  }}
  onkeydown={(e) => e.key === 'Enter' && onselect(e as unknown as MouseEvent)}
  onpointerdown={(e) => drag.begin(e, { kind: 'tab', id: tab.id }, name)}
  use:tooltip={tip}
>
  <span class="pill">
    {#if live}<span class="dot {live}"></span>{/if}
    <span class="name">{name}</span>
    {#if popped}<span class="popped"><Icon name="popout" size={12} /></span>{/if}
    {#if held}<span class="popped"><Icon name={heldMuted ? 'volumeOff' : 'volume'} size={12} /></span>{/if}
    {#if closeButton}
      <button
        class="x"
        aria-label="Close {name}"
        onclick={(e) => {
          e.stopPropagation();
          onclose();
        }}
        onpointerdown={(e) => e.stopPropagation()}
      >
        <Icon name="x" size={12} />
      </button>
    {/if}
  </span>
</div>

<style>
  /* The cell fills its row: one row is as tall as the title bar, so it meets the top edge. */
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    flex: none;
    max-width: 200px;
    min-height: calc(var(--tab-h) + 2 * var(--row-g, 0px));
  }
  .tab.vertical {
    max-width: none;
    min-height: 0;
    height: var(--tab-h);
  }
  .pill {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: var(--tab-h);
    padding: 0 var(--tab-pad);
    border-radius: var(--radius);
    font-size: var(--tab-font);
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
  }
  .vertical .pill {
    flex: 1;
  }
  .tab:focus-visible {
    outline: none;
  }
  .tab:focus-visible .pill {
    outline: 2px solid var(--accent-border);
    outline-offset: -2px;
  }
  .tab:hover .pill {
    background: var(--surface-hover);
    color: var(--text);
  }
  .active .pill {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  /* U9 New messages brighten the name; mentions also get an accent edge. */
  .message .pill,
  .mention .pill {
    color: var(--text);
  }
  .mention .pill {
    box-shadow: inset 0 2px var(--accent);
  }
  .vertical.mention .pill {
    box-shadow: inset -2px 0 var(--accent);
  }
  .selected:not(.active) .pill {
    background: var(--surface-hover);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  /* V101-11 In the 2px gap before the tab. The bar clips the one at a row's start. The
     tab before is another instance, hence global. */
  :global(.tab) + .tab::before {
    content: '';
    position: absolute;
    top: 50%;
    left: -1.5px;
    width: 1px;
    height: calc(var(--tab-h) * 0.5);
    translate: 0 -50%;
    background: var(--border-strong);
    pointer-events: none;
  }
  :global(.vertical) + .vertical::before {
    top: -1.5px;
    left: 25%;
    width: 50%;
    height: 1px;
    translate: none;
  }
  .tab:is(:hover, .active, .selected)::before,
  :global(.tab:is(:hover, .active, .selected)) + .tab::before {
    opacity: 0;
  }
  .grouped .pill::after {
    content: '';
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: 1px;
    height: 2px;
    border-radius: 1px;
    background: var(--group);
  }
  .vertical.grouped .pill::after {
    left: 1px;
    right: auto;
    top: 6px;
    bottom: 6px;
    width: 2px;
    height: auto;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* 1.29 Glowing, not pulsing. */
  .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
  }
  .dot.live {
    background: #ff3b3b;
    box-shadow: 0 0 6px 1px rgba(255, 59, 59, 0.75);
  }
  .dot.rerun {
    background: #f5b83d;
    box-shadow: 0 0 6px 1px rgba(245, 184, 61, 0.6);
  }
  .popped {
    display: flex;
    color: var(--dim);
  }
  .x {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-right: -4px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
  }
  .x:hover {
    background: var(--surface);
    color: var(--text);
  }
  /* Drop marks sit just inside the pill, so a row's first tab shows one too. */
  [data-mark='before'] .pill::before,
  [data-mark='after'] .pill::before {
    content: '';
    position: absolute;
    top: 3px;
    bottom: 3px;
    width: 2px;
    border-radius: 1px;
    background: var(--accent);
  }
  [data-mark='before'] .pill::before {
    left: 0;
  }
  [data-mark='after'] .pill::before {
    right: 0;
  }
  .vertical[data-mark='before'] .pill::before,
  .vertical[data-mark='after'] .pill::before {
    left: 3px;
    right: 3px;
    width: auto;
    height: 2px;
  }
  .vertical[data-mark='before'] .pill::before {
    top: 0;
    bottom: auto;
  }
  .vertical[data-mark='after'] .pill::before {
    top: auto;
    bottom: 0;
  }
  [data-mark='into'] .pill {
    box-shadow: inset 0 0 0 2px var(--accent-border);
  }
</style>
