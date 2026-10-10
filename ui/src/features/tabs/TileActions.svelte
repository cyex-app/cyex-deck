<!-- Buttons in a tile's header: account (1.12), follow (1.11), pop out (1.51), and for
     tiled tabs a menu to edit, split off or close the tile (U3). Feed tiles (F4) have no
     channel, so only the last two. -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import type { Tile } from '../../ipc/bindings/Tile';
  import AccountChip from './AccountChip.svelte';
  import { openEdit } from './dialog.svelte';
  import FollowButton from './FollowButton.svelte';
  import { tabs } from './tabs.svelte';

  interface Props {
    tile: Tile;
    tab: Tab;
    tiled: boolean;
    popout: boolean;
  }

  let { tile, tab, tiled, popout }: Props = $props();

  function more(e: MouseEvent) {
    openMenu(e.currentTarget as HTMLElement, [
      ...(tile.feed ? [] : [{ label: 'Edit', icon: 'pencil' as const, run: () => openEdit(tab.id, tile.id) }]),
      {
        label: 'Move to its own tab',
        icon: 'popout',
        run: () => void tabs.apply({ op: 'detach', tile: tile.id, before: null }),
      },
      'separator',
      { label: 'Close', icon: 'x', run: () => void tabs.apply({ op: 'closeTile', tile: tile.id }) },
    ], 'end');
  }
</script>

{#if !tile.feed}
  <AccountChip {tile} />
  <FollowButton channelId={tile.channel.id} account={tile.account} />
{/if}
{#if tiled}
  <IconButton icon="more" label="More" hint="Edit, move to its own tab or close." size={15} onclick={more} />
{:else if !popout}
  <IconButton icon="popout" label="Pop out" hint="Opens this tab in its own window." size={14} onclick={() => tabs.popOut(tab)} />
{/if}
