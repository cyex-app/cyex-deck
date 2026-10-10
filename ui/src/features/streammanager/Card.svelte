<!-- One stream-manager panel: a title, optional controls on the right, then the body. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    /** Spans two grid columns where there's room. */
    wide?: boolean;
    tools?: Snippet;
    children: Snippet;
  }

  let { title, wide = false, tools, children }: Props = $props();
</script>

<section class="card" class:wide>
  <header>
    <h3>{title}</h3>
    {#if tools}<div class="tools">{@render tools()}</div>{/if}
  </header>
  <div class="body">{@render children()}</div>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    min-width: 0;
    max-height: 460px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .card.wide {
    grid-column: span 2;
  }
  @media (max-width: 720px) {
    .card.wide {
      grid-column: auto;
    }
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 34px;
    padding: 4px 6px 4px 10px;
    border-bottom: 1px solid var(--border);
  }
  h3 {
    flex: 1;
    margin: 0;
    font-size: 0.8rem;
    font-weight: 700;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 4px;
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
