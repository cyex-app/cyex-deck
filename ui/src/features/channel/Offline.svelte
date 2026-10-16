<!-- The stream side while the channel is offline: its videos and clips are a click away,
     over the channel's offline picture, dimmed (V101-22). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
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

<div class="offline" style:--picture={info?.offlineImage ? `url("${info.offlineImage}")` : null}>
  <div class="card">
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
      <button aria-label="Videos" use:tooltip={'Videos'} onclick={() => onpage({ kind: 'videos' })}><Icon name="film" size={14} /><span class="t">Videos</span></button>
      <button aria-label="Clips" use:tooltip={'Clips'} onclick={() => onpage({ kind: 'clips' })}><Icon name="clapper" size={14} /><span class="t">Clips</span></button>
      <button aria-label="About" use:tooltip={'About'} onclick={() => onpage({ kind: 'about' })}><Icon name="info" size={14} /><span class="t">About</span></button>
    </div>
  </div>
</div>

<style>
  .offline {
    flex: 1;
    container-type: size;
    display: flex;
    overflow: hidden;
    background:
      linear-gradient(var(--scrim-strong), var(--scrim-strong)),
      var(--picture, none) center / cover no-repeat,
      #000;
    color: var(--on-video-dim);
    text-align: center;
  }
  .card {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }
  /* A slim card (a small tile, or a narrow column): icons only. */
  @container (max-height: 190px) or (max-width: 230px) {
    .t {
      display: none;
    }
    .card {
      gap: 6px;
    }
  }
  .go {
    display: flex;
    gap: 6px;
  }
  /* Too short for the buttons: the tab header's Videos, Clips and About still are. */
  @container (max-height: 124px) {
    .go {
      display: none;
    }
  }
  button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    background: color-mix(in srgb, var(--on-video) 8%, transparent);
    border: 0;
    border-radius: var(--radius);
    color: var(--on-video);
  }
  button:hover {
    background: color-mix(in srgb, var(--on-video) 16%, transparent);
  }
</style>
