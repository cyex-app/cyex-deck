<!-- C30 A link's preview under its message. Asked for once the line scrolls into view;
     Rust keeps the answers for the run. Clicks go through the list (`data-link`). -->
<script lang="ts">
  import type { LinkInfo } from '../../ipc/bindings/LinkInfo';
  import { chatIpc } from '../../ipc/chat';

  let { url }: { url: string } = $props();

  let el = $state<HTMLElement>();
  let info = $state<LinkInfo | null>(null);
  const host = $derived.by(() => {
    try {
      return new URL(url).hostname.replace(/^www\./, '');
    } catch {
      return '';
    }
  });
  /** Title, then the next line or two. */
  const lines = $derived((info?.lines ?? []).filter((l) => l.length).slice(0, 3));

  $effect(() => {
    if (!el) return;
    // Asked a screen ahead, so the card grows while it is still off screen.
    const seen = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return;
        seen.disconnect();
        chatIpc
          .resolveLink(url)
          .then((i) => !gone && (info = i))
          .catch(() => {});
      },
      { rootMargin: '600px 0px' },
    );
    let gone = false;
    seen.observe(el);
    return () => {
      gone = true;
      seen.disconnect();
    };
  });
</script>

<div class="card" bind:this={el} class:empty={!info || (!lines.length && !info.thumbnail)}>
  {#if info && (lines.length || info.thumbnail)}
    <a class="inner" href={url} data-link={url} data-host={host}>
      {#if info.thumbnail}<img src={info.thumbnail} alt="" loading="lazy" referrerpolicy="no-referrer" />{/if}
      <span class="text">
        {#each lines as line, i (i)}
          <span class:first={i === 0}>{line.map((p) => p.text).join('')}</span>
        {/each}
        <span class="host">{host}</span>
      </span>
    </a>
  {/if}
</div>

<style>
  .card.empty {
    height: 0;
  }
  .inner {
    display: flex;
    gap: 8px;
    max-width: 420px;
    margin: 4px 0 2px;
    padding: 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.85em;
    text-decoration: none;
  }
  .inner:hover {
    border-color: var(--border-strong);
  }
  img {
    width: 96px;
    height: 54px;
    object-fit: cover;
    border-radius: var(--radius);
    flex: none;
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .text > span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .first {
    color: var(--text);
    font-weight: 600;
  }
  .host {
    color: var(--dim);
  }
</style>
