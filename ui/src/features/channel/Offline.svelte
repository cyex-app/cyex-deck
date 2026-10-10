<!-- The stream side while the channel is offline: its videos and clips are a click away. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
  import type { Tile } from '../../ipc/bindings/Tile';
  import type { Page } from '../player/players.svelte';
  import StoryRing from '../player/StoryRing.svelte';

  interface Props {
    tile: Tile;
    info: ChannelInfo | undefined;
    onpage: (p: Page) => void;
    onstories: () => void;
  }

  let { tile, info, onpage, onstories }: Props = $props();
</script>

<div class="offline">
  <StoryRing
    channelId={tile.channel.id}
    account={tile.account}
    avatar={info?.avatar}
    name={tile.channel.name}
    login={tile.channel.login}
    size={72}
    onopen={onstories}
  />
  <span>{tile.channel.name} is offline</span>
  <div class="go">
    <button onclick={() => onpage({ kind: 'videos' })}><Icon name="film" size={14} />Videos</button>
    <button onclick={() => onpage({ kind: 'clips' })}><Icon name="clapper" size={14} />Clips</button>
    <button onclick={() => onpage({ kind: 'about' })}><Icon name="info" size={14} />About</button>
  </div>
</div>

<style>
  .offline {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    background: #000;
    color: #c8c9ce;
  }
  .go {
    display: flex;
    gap: 6px;
  }
  button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    background: rgba(255, 255, 255, 0.08);
    border: 0;
    border-radius: var(--radius);
    color: #e8e8ea;
  }
  button:hover {
    background: rgba(255, 255, 255, 0.16);
  }
</style>
