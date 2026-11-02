<!-- C30 A link's preview under its message. Asked for once the line scrolls into view;
     Rust keeps the answers (V108-C25). Clicks go through the list (`data-link`). -->
<script lang="ts">
  import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
  import { hostOf } from '../../lib/host';
  import { factText, kindText } from './linkFacts';
  import { links } from './linkPreviews';

  let { url }: { url: string } = $props();

  let el = $state<HTMLElement>();
  let info = $state<LinkPreview | null>(null);
  const host = $derived(hostOf(url));
  /** Title, then the next line or two: the subtitle, facts or the resolver's lines. */
  const lines = $derived.by(() => {
    if (!info) return [];
    const facts = info.facts.map(factText).filter(Boolean).join(' · ');
    const rest = info.lines.filter((l) => l.length).map((l) => l.map((p) => p.text).join(''));
    return [info.title, info.subtitle, facts, ...rest].filter((t): t is string => !!t).slice(0, 3);
  });

  $effect(() => {
    if (!el) return;
    // Asked a screen ahead, so the card grows while it is still off screen.
    const seen = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return;
        seen.disconnect();
        links
          .get(url)
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
      {#if info.thumbnail}<img
          class:blur={info.nsfw}
          src={info.thumbnail}
          alt=""
          loading="lazy"
          referrerpolicy="no-referrer"
        />{/if}
      <span class="text">
        {#each lines as line, i (i)}
          <span class:first={i === 0}>{line}</span>
        {/each}
        <span class="host">{info.kind === 'web' ? host : kindText(info)}</span>
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
  .blur {
    filter: blur(10px);
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
