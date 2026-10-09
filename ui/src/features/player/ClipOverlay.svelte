<!-- 1.20 A clip from chat over the stream, inside the window. The stream is silent until
     it closes. A clip the user just made (P9) can be edited on Twitch from here. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { ClipPlayback } from '../../ipc/bindings/ClipPlayback';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { systemIpc } from '../../ipc/system';
  import { formatDate, formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import ClipPlayer from './ClipPlayer.svelte';

  interface Props {
    slug: string;
    tile: Tile | undefined;
    /** P9 The editor for a clip just made. */
    editUrl?: string | null;
    onclose: () => void;
  }

  let { slug, tile, editUrl = null, onclose }: Props = $props();

  let clip = $state<ClipPlayback | null>(null);

  function open(url: string) {
    systemIpc.openLink(url).catch((e) => toast.error(String(e)));
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window {onkeydown} />

<!-- The dimmed area closes the clip, like Escape; the box itself has the buttons. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="box" role="dialog" aria-label="Clip">
    <header>
      <div class="text">
        <b>{clip?.clip.title || 'Clip'}</b>
        {#if clip}
          <span>
            {clip.clip.channelName}{clip.clip.clippedBy ? ` · clipped by ${clip.clip.clippedBy}` : ''} ·
            {formatNumber(clip.clip.views)} views · {formatDate(new Date(clip.clip.createdAt))}
          </span>
        {/if}
      </div>
      {#if editUrl}
        <button class="link" onclick={() => open(editUrl!)}>Edit on Twitch</button>
      {/if}
      <button class="icon" aria-label="Open on Twitch" use:tooltip={'Open on Twitch'} onclick={() => open(`https://clips.twitch.tv/${slug}`)}>
        <Icon name="popout" size={15} />
      </button>
      <button class="icon" aria-label="Close" use:tooltip={'Close (Esc)'} onclick={onclose}><Icon name="x" size={16} /></button>
    </header>
    <div class="screen">
      <ClipPlayer {slug} {tile} onloaded={(c) => (clip = c)} />
    </div>
  </div>
</div>

<style>
  .overlay {
    position: absolute;
    inset: 0;
    z-index: 6;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(0, 0, 0, 0.6);
  }
  .box {
    display: flex;
    flex-direction: column;
    width: min(100%, 960px);
    max-height: 100%;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 8px 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .text b,
  .text span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .text span {
    color: var(--dim);
    font-size: 0.8rem;
  }
  .screen {
    display: flex;
    aspect-ratio: 16 / 9;
    min-height: 0;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .icon:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .link {
    padding: 4px 10px;
    background: var(--accent-dim);
    border: 0;
    border-radius: var(--radius);
    color: var(--accent-text);
    font-weight: 600;
  }
</style>
