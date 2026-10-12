<!-- Panel text (1.21), already turned into blocks in Rust: no HTML is ever rendered.
     Links open in the browser. -->
<script lang="ts">
  import type { Block } from '../../ipc/bindings/Block';
  import type { Span } from '../../ipc/bindings/Span';
  import { systemIpc } from '../../ipc/system';
  import { toast } from '../../lib/toast.svelte';

  let { blocks }: { blocks: Block[] } = $props();

  function open(e: MouseEvent, url: string) {
    e.preventDefault();
    systemIpc.openChatLink(url).catch((err) => toast.error(String(err)));
  }
</script>

{#snippet spans(list: Span[])}
  {#each list as s, i (i)}
    {#if s.t === 'text'}<span class:b={s.bold} class:i={s.italic}>{s.text}</span>
    {:else if s.t === 'code'}<code>{s.text}</code>
    {:else if s.t === 'link'}<a href={s.url} onclick={(e) => open(e, s.url)}>{s.text}</a>
    {:else if s.t === 'image'}<img src={s.url} alt={s.alt} loading="lazy" referrerpolicy="no-referrer" />
    {:else}<br />{/if}
  {/each}
{/snippet}

<div class="md">
  {#each blocks as b, i (i)}
    {#if b.t === 'heading'}<p class="h h{Math.min(b.level, 3)}">{@render spans(b.spans)}</p>
    {:else if b.t === 'paragraph'}<p>{@render spans(b.spans)}</p>
    {:else if b.t === 'list'}
      {#if b.ordered}
        <ol>{#each b.items as item, j (j)}<li>{@render spans(item)}</li>{/each}</ol>
      {:else}
        <ul>{#each b.items as item, j (j)}<li>{@render spans(item)}</li>{/each}</ul>
      {/if}
    {:else if b.t === 'quote'}<blockquote>{@render spans(b.spans)}</blockquote>
    {:else}<hr />{/if}
  {/each}
</div>

<style>
  .md {
    font-size: 0.85rem;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  p,
  ul,
  ol,
  blockquote {
    margin: 0 0 8px;
  }
  ul,
  ol {
    padding-left: 18px;
  }
  .h {
    font-weight: 700;
  }
  .h1 {
    font-size: 1.1rem;
  }
  .h2 {
    font-size: 1rem;
  }
  .b {
    font-weight: 700;
  }
  .i {
    font-style: italic;
  }
  code {
    padding: 0 3px;
    background: var(--surface);
    border-radius: var(--radius);
    font-family: var(--font-mono);
    font-size: 0.8rem;
  }
  a {
    color: var(--accent-text);
    text-decoration: none;
  }
  a:hover {
    text-decoration: underline;
  }
  img {
    max-width: 100%;
    vertical-align: middle;
  }
  blockquote {
    padding-left: 10px;
    border-left: 3px solid var(--border-strong);
    color: var(--muted);
  }
  hr {
    border: 0;
    border-top: 1px solid var(--border);
  }
</style>
