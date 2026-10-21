<!-- V107-R39 A clip or video from the Clips or Videos panel, in the app's own players, over
     the whole window (drawn on the page so a small or popped-out panel doesn't cut it).
     Escape, the dimmed area or the player's close button shut it. -->
<script lang="ts">
  import { layer } from '../../lib/escape';
  import { focusScope } from '../../lib/focusScope';
  import ClipPlayer from '../player/ClipPlayer.svelte';
  import VodPlayer from '../player/VodPlayer.svelte';
  import type { Playing } from './content';

  interface Props {
    what: Playing;
    account: string | null;
    onclose: () => void;
  }

  let { what, account, onclose }: Props = $props();

  let box = $state<HTMLElement>();

  function toBody(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }

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

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" use:toBody onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="box" bind:this={box} role="dialog" aria-modal="true" aria-label={what.kind === 'clip' ? 'Clip' : 'Video'} tabindex="-1">
    {#if what.kind === 'clip'}
      {#key what.slug}<ClipPlayer slug={what.slug} tile={undefined} {onclose} />{/key}
    {:else}
      {#key what.id}<VodPlayer id={what.id} at={0} tile={undefined} {account} {onclose} />{/key}
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: var(--z-dialog);
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--scrim);
  }
  .box {
    display: flex;
    flex-direction: column;
    /* The video at 16:9 plus the player's header, as big as the window allows. */
    width: min(100%, 1100px, calc((100vh - 48px) * 1.6));
    aspect-ratio: 16 / 10;
    max-height: 100%;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-dialog);
    overflow: hidden;
  }
  .box:focus {
    outline: none;
  }
</style>
