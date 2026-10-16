<!-- Modal dialog over the window. Escape, the x or a click outside closes it. Focus starts on the
     first field (or `data-autofocus`, or the primary button), Tab stays inside, and focus goes
     back to what opened it on close. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { layer } from '../lib/escape';
  import { firstField, focusScope } from '../lib/focusScope';
  import IconButton from './IconButton.svelte';

  interface Props {
    title: string;
    width?: number;
    /** V1051-05 Fixed near the top, so a body that grows doesn't move the title. */
    top?: boolean;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  }

  let { title, width = 420, top = false, onclose, children, footer }: Props = $props();

  let box = $state<HTMLElement>();

  $effect(() => {
    if (!box) return;
    // A dialog without fields still takes focus, so Enter and typing don't reach the chat box behind it.
    const release = focusScope(box, { trap: true, initial: firstField });
    const off = layer('dialog', () => onclose());
    return () => {
      off();
      release();
    };
  });
</script>

<!-- The backdrop only catches outside clicks; Escape is handled above. -->
<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="backdrop" class:top onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label={title} tabindex="-1" bind:this={box} style:width="{width}px">
    <header>
      <h2>{title}</h2>
      <IconButton icon="x" label="Close" size={15} onclick={onclose} />
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: var(--z-dialog);
    display: grid;
    /* V105-16: centered in the window. Fixed tracks, so max-width/height below hold the
       dialog inside the window and a tall body scrolls instead of running off it. */
    grid-template: minmax(0, 1fr) / minmax(0, 1fr);
    place-items: center;
    padding: 12px;
    background: var(--overlay);
    animation: fade var(--fast) ease-out;
  }
  .backdrop.top {
    align-items: start;
    padding-top: min(14vh, 96px);
  }
  .dialog {
    max-width: 100%;
    max-height: 100%;
    animation: pop var(--fast) ease-out;
    display: flex;
    flex-direction: column;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-dialog);
  }
  /* Only the dialog box itself; what's inside keeps its ring. */
  .dialog:focus {
    outline: none;
  }
  header,
  footer {
    flex: none;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 8px 4px 16px;
  }
  h2 {
    margin: 0;
  }
  .body {
    min-height: 0;
    padding: 8px 16px 14px;
    overflow-y: auto;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .backdrop,
    .dialog {
      animation: none;
    }
  }
</style>
