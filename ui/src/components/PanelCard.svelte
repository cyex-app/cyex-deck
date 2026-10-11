<!-- V103-06 One panel's frame in the mod view and the stream manager: a title bar (dragged
     while the layout is unlocked, with a close button then), optional tools on the right,
     then the body, which scrolls. `flush` drops the body's padding, for lists. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { grabFrom, usePanel } from './panel';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    title: string;
    tools?: Snippet;
    flush?: boolean;
    children: Snippet;
  }

  let { title, tools, flush = false, children }: Props = $props();

  const panel = usePanel();
  const ctl = $derived(panel?.());
  const grab = $derived(grabFrom(ctl?.grab));
</script>

<section class="card">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class:grab={!!grab} onpointerdown={grab}>
    <h3>{title}</h3>
    {#if tools}<div class="tools">{@render tools()}</div>{/if}
    {#if ctl?.close}
      <button class="x" use:tooltip={'Close'} aria-label="Close {title}" onclick={ctl.close}>×</button>
    {/if}
  </header>
  <div class="body" class:flush>{@render children()}</div>
</section>

<style>
  .card {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 34px;
    padding: 4px 6px 4px 10px;
    border-bottom: 1px solid var(--border);
  }
  header.grab {
    cursor: grab;
  }
  /* The title keeps its words; tools wrap beside it. */
  h3 {
    flex: 1 0 auto;
    margin: 0;
    font-size: 0.8rem;
    font-weight: 700;
    white-space: nowrap;
  }
  .tools {
    display: flex;
    flex: 0 1 auto;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .x {
    flex: none;
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 1rem;
    line-height: 1;
    cursor: pointer;
  }
  .x:hover {
    color: var(--text);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .body.flush {
    padding: 0;
    gap: 0;
  }
  .body :global(.field) {
    background: var(--bg);
    min-width: 0;
  }
  .body :global(.note) {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
  }
  .body :global(.line) {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
</style>
