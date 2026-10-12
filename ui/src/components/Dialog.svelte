<!-- Modal dialog over the window. Escape, the x or a click outside closes it. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import IconButton from './IconButton.svelte';

  interface Props {
    title: string;
    width?: number;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  }

  let { title, width = 420, onclose, children, footer }: Props = $props();

  let box = $state<HTMLElement>();

  $effect(() => {
    box?.querySelector<HTMLElement>('input, textarea, select')?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onclose();
      }
    };
    addEventListener('keydown', onKey);
    return () => removeEventListener('keydown', onKey);
  });
</script>

<!-- The backdrop only catches outside clicks; Escape is handled above. -->
<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="backdrop" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label={title} bind:this={box} style:width="{width}px">
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
    z-index: var(--z-popover);
    display: grid;
    place-items: start center;
    padding-top: 12vh;
    background: var(--overlay);
    animation: fade var(--fast) ease-out;
  }
  .dialog {
    max-width: calc(100vw - 24px);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 16px 48px var(--shadow);
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
</style>
