<!-- Buttons in a tile's header: account (1.12), follow (1.11, not on your own channel),
     pop out (1.51), and for tiled tabs a menu to edit, hide the chat or stream (V101-20),
     split off or close the tile (U3). A chat puts those entries in its own menu instead
     (`menu` off, V101-21). Feed tiles (F4) have no channel, so only the last two. -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import type { Tile } from '../../ipc/bindings/Tile';
  import AccountChip from './AccountChip.svelte';
  import FollowButton from './FollowButton.svelte';
  import { tileEntries } from './menus';
  import { tabs } from './tabs.svelte';

  interface Props {
    tile: Tile;
    tab: Tab;
    tiled: boolean;
    popout: boolean;
    /** Show the tile's ⋯ menu here. */
    menu?: boolean;
  }

  let { tile, tab, tiled, popout, menu = true }: Props = $props();
</script>

{#if !tile.feed}
  <AccountChip {tile} />
  {#if tile.channel.id !== tile.account}<FollowButton channelId={tile.channel.id} name={tile.channel.name} account={tile.account} />{/if}
{/if}
{#if tiled}
  {#if menu}
    <IconButton
      icon="more"
      label="More"
      hint="Edit, move to its own tab or close."
      size={15}
      onclick={(e) => openMenu(e.currentTarget as HTMLElement, tileEntries(tab, tile), 'end')}
    />
  {/if}
{:else if !popout}
  <IconButton icon="popout" label="Pop out" hint="Opens this tab in its own window." size={14} onclick={() => tabs.popOut(tab)} />
{/if}
