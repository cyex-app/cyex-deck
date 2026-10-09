<!-- Dropdown under its trigger. Put it next to the trigger inside a `position: relative`
     wrapper; clicks outside that wrapper and Escape close it. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    open: boolean;
    align?: 'start' | 'end';
    children: Snippet;
  }

  let { open = $bindable(), align = 'end', children }: Props = $props();

  let box = $state<HTMLElement>();

  $effect(() => {
    if (!open) return;
    const wrapper = box?.parentElement;
    const onDown = (e: MouseEvent) => {
      if (!wrapper?.contains(e.target as Node)) open = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') open = false;
    };
    const onBlur = () => (open = false);
    addEventListener('mousedown', onDown, true);
    addEventListener('keydown', onKey);
    addEventListener('blur', onBlur);
    return () => {
      removeEventListener('mousedown', onDown, true);
      removeEventListener('keydown', onKey);
      removeEventListener('blur', onBlur);
    };
  });
</script>

{#if open}
  <div class="popover {align}" bind:this={box} role="menu">
    {@render children()}
  </div>
{/if}

<style>
  .popover {
    position: absolute;
    top: calc(100% + 6px);
    z-index: var(--z-popover);
    min-width: 220px;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px var(--shadow);
  }
  .start {
    left: 0;
  }
  .end {
    right: 0;
  }
</style>
