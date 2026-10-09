<!-- One channel in a tab (U3): stream, chat or both, split by a draggable divider (3.11)
     on the side set in Settings (2.1). In tiled tabs the header drags the tile. -->
<script lang="ts">
  import SplitPane from '../../components/SplitPane.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { settings } from '../../lib/settings.svelte';
  import ChatView from '../chat/ChatView.svelte';
  import { drag } from './drag.svelte';
  import StreamStub from './StreamStub.svelte';
  import TileActions from './TileActions.svelte';
  import { tabs, tiles } from './tabs.svelte';

  interface Props {
    tile: Tile;
    tab: Tab;
    popout: boolean;
    onsignin: () => void;
  }

  let { tile, tab, popout, onsignin }: Props = $props();

  const tiled = $derived(tiles(tab).length > 1);
  const position = $derived(settings.v.general.chatPosition);
  const chatFirst = $derived(position === 'left' || position === 'top');

  const onheaderdown = $derived(
    tiled ? (e: PointerEvent) => drag.begin(e, { kind: 'tile', id: tile.id }, tile.channel.name) : undefined,
  );

  /** Drop zone shown while something that can land here is dragged over. */
  const zone = $derived.by(() => {
    const item = drag.item;
    const t = drag.target;
    if (!item || t?.kind !== 'tile' || t.id !== tile.id) return null;
    if (item.kind === 'tile') return item.id === tile.id ? null : t.zone;
    const from = tabs.tab(item.id);
    return from && from.id !== tab.id && tiles(from).length === 1 ? t.zone : null;
  });

  function setRatio(first: number) {
    const ratio = chatFirst ? first : 1 - first;
    void tabs.apply({ op: 'ratio', tile: tile.id, ratio });
  }
</script>

{#snippet actions()}
  <TileActions {tile} {tab} {tiled} {popout} />
{/snippet}

{#snippet chat()}
  {#key `${tile.channel.login}/${tile.account}`}
    <ChatView
      login={tile.channel.login}
      name={tile.channel.name}
      account={tile.account}
      readKey={tile.id}
      {actions}
      {onheaderdown}
      {onsignin}
    />
  {/key}
{/snippet}

{#snippet stream()}
  <StreamStub {tile} actions={tile.show.chat ? undefined : actions} {onheaderdown} />
{/snippet}

<div class="tile" data-tile-id={tile.id}>
  {#if tile.show.player && tile.show.chat}
    <SplitPane
      direction={position === 'left' || position === 'right' ? 'row' : 'column'}
      ratio={chatFirst ? tile.ratio : 1 - tile.ratio}
      onchange={setRatio}
      first={chatFirst ? chat : stream}
      second={chatFirst ? stream : chat}
    />
  {:else if tile.show.player}
    {@render stream()}
  {:else}
    {@render chat()}
  {/if}
  {#if zone}<div class="drop {zone}"></div>{/if}
</div>

<style>
  .tile {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  .drop {
    position: absolute;
    z-index: 5;
    background: var(--accent-wash);
    border: 2px solid var(--accent-border);
    border-radius: var(--radius);
    pointer-events: none;
  }
  .center {
    inset: 20%;
  }
  .left {
    inset: 0 50% 0 0;
  }
  .right {
    inset: 0 0 0 50%;
  }
  .top {
    inset: 0 0 50% 0;
  }
  .bottom {
    inset: 50% 0 0 0;
  }
</style>
