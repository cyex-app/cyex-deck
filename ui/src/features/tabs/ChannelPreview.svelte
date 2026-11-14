<!-- V111-24 A quick look at the highlighted suggestion in Add a tab: Twitch's live thumbnail with title,
     category, viewers and uptime; offline, its banner or the picture. The 16:9 box is always there, so
     nothing jumps when it fills. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
  import { previewFacts } from './channelPreview';

  interface Props {
    /** The channel to show; null keeps the empty box. */
    channel: ChannelInfo | null;
  }

  let { channel }: Props = $props();

  /** The box is about this wide (css px) in the dialog. */
  const BOX = 190;
  const facts = $derived(channel ? previewFacts(channel, BOX, window.devicePixelRatio || 1) : null);
  /** The pictures that loaded or failed, by address. */
  let loaded = $state('');
  let failed = $state('');
  const image = $derived(facts?.image && failed !== facts.image ? facts.image : null);
</script>

<div class="preview" aria-live="polite">
  <div class="box">
    {#if channel && facts}
      {#if image}
        {#key image}
          <img
            class:in={loaded === image}
            src={image}
            alt=""
            decoding="async"
            referrerpolicy="no-referrer"
            onload={() => (loaded = image)}
            onerror={() => (failed = image)}
          />
        {/key}
      {:else if !facts.live}
        <span class="face"><Avatar src={channel.avatar} name={channel.name} size={48} /></span>
      {/if}
      {#if facts.live}
        <span class="badge" class:rerun={facts.rerun}>{facts.rerun ? 'Rerun' : 'Live'}</span>
      {/if}
    {/if}
  </div>
  <div class="facts">
    {#if channel && facts}
      <span class="who">{channel.name}</span>
      {#if facts.live}
        <span class="title" title={facts.title}>{facts.title || 'No title'}</span>
        {#if facts.game}<span class="line">{facts.game}</span>{/if}
        <span class="line">{[facts.viewers, facts.uptime && `up ${facts.uptime}`].filter(Boolean).join(' · ')}</span>
      {:else}
        <span class="line">Offline</span>
      {/if}
    {:else}
      <span class="line">Point at a result to preview it.</span>
    {/if}
  </div>
</div>

<style>
  .preview {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    margin-top: 8px;
  }
  .box {
    position: relative;
    flex: 0 1 190px;
    min-width: 120px;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity var(--fast) ease-out;
  }
  img.in {
    opacity: 1;
  }
  .face {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
  }
  .badge {
    position: absolute;
    top: 4px;
    left: 4px;
    padding: 1px 5px;
    border-radius: var(--radius);
    background: var(--live);
    color: #fff;
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
  }
  .badge.rerun {
    background: var(--yellow);
    color: #000;
  }
  .facts {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.82rem;
  }
  .who {
    font-weight: 700;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .title {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  .line {
    color: var(--muted);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
