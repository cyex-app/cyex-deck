<!-- The stream side of a tile until the player lands (phase 5): Twitch's live preview
     picture, refreshed each minute, or the channel's avatar when offline. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { formatNumber } from '../../lib/format';
  import { tabs } from './tabs.svelte';

  interface Props {
    tile: Tile;
    /** Header buttons, when the chat isn't there to carry them. */
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
  }

  let { tile, actions, onheaderdown }: Props = $props();

  const info = $derived(tabs.live[tile.channel.id]);
  const stream = $derived(info?.stream);

  let minute = $state(Math.floor(Date.now() / 60_000));
  $effect(() => {
    const timer = setInterval(() => (minute = Math.floor(Date.now() / 60_000)), 60_000);
    return () => clearInterval(timer);
  });

  const preview = $derived(
    `https://static-cdn.jtvnw.net/previews-ttv/live_user_${tile.channel.login}-640x360.jpg?t=${minute}`,
  );
</script>

<section class="stream">
  {#if actions}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <header onpointerdown={onheaderdown}>
      <span class="name">{tile.channel.name}</span>
      {@render actions()}
    </header>
  {/if}
  <div class="screen">
    {#if stream}
      <img class="preview" src={preview} alt="" />
      <div class="info">
        <span class="badge" class:rerun={stream.rerun}>{stream.rerun ? 'RERUN' : 'LIVE'}</span>
        <span class="title">{stream.title}</span>
        <span class="meta">{[stream.game, `${formatNumber(stream.viewers)} watching`].filter(Boolean).join(' · ')}</span>
      </div>
    {:else}
      <div class="offline">
        <Avatar src={info?.avatar} name={tile.channel.name} size={64} />
        <span>{tile.channel.name} is offline</span>
      </div>
    {/if}
  </div>
</section>

<style>
  .stream {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: #000;
  }
  header {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 36px;
    padding: 0 6px 0 12px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .name {
    flex: 1;
    font-weight: 700;
  }
  .screen {
    position: relative;
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  .preview {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .info {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 24px 14px 12px;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.75));
    color: #fff;
  }
  .badge {
    align-self: flex-start;
    padding: 0 5px;
    background: #e91916;
    border-radius: var(--radius);
    font-size: 0.7rem;
    font-weight: 700;
  }
  .badge.rerun {
    background: #b07a12;
  }
  .title {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: 0.8rem;
    opacity: 0.85;
  }
  .offline {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    color: #c8c9ce;
  }
</style>
