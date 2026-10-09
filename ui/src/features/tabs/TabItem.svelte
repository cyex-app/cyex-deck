<!-- One channel tab (1.10): name or alias (1.22), live dot (1.29), close button (1.25).
     Double click edits (1.18), middle click closes, drag reorders (U5). -->
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
  const popped = $derived(tabs.popped.has(tab.id));
  const closeButton = $derived(settings.v.appearance.tabCloseButton);

  /** Drop marker while something is dragged over this tab. */
  const mark = $derived.by(() => {
    const t = drag.target;
    if (!drag.item || t?.kind !== 'tab' || t.id !== tab.id) return null;
    if (drag.item.kind === 'tile') return 'into';
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
    return { label: parts.length ? parts.join(' · ') : `${name} · offline`, hint: hint ? `${hint} (${who})` : who };
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
  {#if live}<span class="dot {live}"></span>{/if}
  <span class="name">{name}</span>
  {#if popped}<span class="popped"><Icon name="popout" size={12} /></span>{/if}
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
</div>

<style>
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    max-width: 200px;
    height: var(--tab-h);
    padding: 0 var(--tab-pad);
    border-radius: var(--radius);
    font-size: var(--tab-font);
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
  }
  .tab.vertical {
    max-width: none;
  }
  .tab:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .tab.active {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .tab.selected:not(.active) {
    background: var(--surface-hover);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  .grouped::after {
    content: '';
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: 1px;
    height: 2px;
    border-radius: 1px;
    background: var(--group);
  }
  .vertical.grouped::after {
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
  [data-mark='before']::before,
  [data-mark='after']::before {
    content: '';
    position: absolute;
    top: 3px;
    bottom: 3px;
    width: 2px;
    background: var(--accent);
  }
  [data-mark='before']::before {
    left: -2px;
  }
  [data-mark='after']::before {
    right: -2px;
  }
  .vertical[data-mark='before']::before,
  .vertical[data-mark='after']::before {
    left: 3px;
    right: 3px;
    width: auto;
    height: 2px;
  }
  .vertical[data-mark='before']::before {
    top: -2px;
    bottom: auto;
  }
  .vertical[data-mark='after']::before {
    top: auto;
    bottom: -2px;
  }
  [data-mark='into'] {
    box-shadow: inset 0 0 0 2px var(--accent-border);
  }
</style>
