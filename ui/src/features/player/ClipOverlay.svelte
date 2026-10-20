<!-- 1.20 A clip from chat over the stream, inside the window. The stream is silent until
     it closes. V103-09 A clip just made without Sign in with Twitch shows here as made (the
     last 30 seconds), with a note that the sign-in lets you pick the start and end. -->
<script lang="ts">
  import GateFlag from '../../components/GateFlag.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import type { ClipPlayback } from '../../ipc/bindings/ClipPlayback';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { systemIpc } from '../../ipc/system';
  import { layer } from '../../lib/escape';
  import { focusScope } from '../../lib/focusScope';
  import { formatDate, formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import ClipPlayer from './ClipPlayer.svelte';
  import { errorText } from '../../lib/errors';

  interface Props {
    slug: string;
    tile: Tile | undefined;
    /** V103-09 Just made, without Twitch's editor. */
    made?: boolean;
    onclose: () => void;
  }

  let { slug, tile, made = false, onclose }: Props = $props();

  let clip = $state<ClipPlayback | null>(null);

  function open(url: string) {
    systemIpc.openLink(url).catch((e) => toast.error(errorText(e)));
  }

  let box = $state<HTMLElement>();

  // The shared layer: Escape closes only the top one (a menu or dialog over the clip goes
  // first), Tab stays inside, and focus goes back to where it was.
  $effect(() => {
    if (!box) return;
    const release = focusScope(box, { trap: true });
    const off = layer('dialog', onclose);
    return () => {
      off();
      release();
    };
  });
</script>

<!-- The dimmed area closes the clip, like Escape (the layer); the box itself has the buttons. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="box" bind:this={box} role="dialog" aria-modal="true" aria-label="Clip" tabindex="-1">
    <header>
      <div class="text">
        <b>{clip?.clip.title || 'Clip'}</b>
        {#if clip}
          <span>
            {clip.clip.channelName}{clip.clip.clippedBy ? ` · clipped by ${clip.clip.clippedBy}` : ''} ·
            {formatNumber(clip.clip.views)} {clip.clip.views === 1 ? 'view' : 'views'} · {formatDate(new Date(clip.clip.createdAt))}
          </span>
        {/if}
      </div>
      {#if made}
        <span class="note">Last 30 seconds. Sign in with Twitch to pick the start and end.</span>
        <GateFlag kind="helix" />
      {/if}
      <IconButton icon="twitch" label="Open on Twitch" size={15} onclick={() => open(`https://clips.twitch.tv/${slug}`)} />
      <IconButton icon="x" label="Close" hint="Esc" size={16} onclick={onclose} />
    </header>
    <div class="screen">
      <ClipPlayer {slug} {tile} fresh={made} alone onloaded={(c) => (clip = c)} />
    </div>
  </div>
</div>

<style>
  .overlay {
    position: absolute;
    inset: 0;
    z-index: var(--z-panel);
    display: grid;
    place-items: center;
    padding: 16px;
    background: var(--scrim);
  }
  .box:focus {
    outline: none;
  }
  .box {
    display: flex;
    flex-direction: column;
    width: min(100%, 960px);
    max-height: 100%;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-dialog);
    overflow: hidden;
  }
  /* V101-26 As big as the screen allows, header included. */
  :global(:fullscreen) > .overlay .box {
    width: min(100%, calc((100vh - 90px) * 16 / 9));
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
  .note {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    color: var(--dim);
    font-size: 0.8rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
