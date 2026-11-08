<!-- C30 A trusted link's preview under its message. Asked for once the line is near the screen;
     Rust keeps the answers (V108-C25). Clicks go through the list (`data-link`).
     V110-C66 The card has its final box from the first frame: a skeleton the same size until the
     answer comes, the picture's box drawn before it loads. Nothing on the line moves when it
     fills in; a link with nothing to show keeps a one-line box when on screen, and closes
     when off screen (the list keeps what is read in place). -->
<script lang="ts">
  import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
  import { hostOf } from '../../lib/host';
  import { CARD_HEIGHT, CARD_PIC, cardAfter, cardLines, type CardState } from './cardBox';
  import { kindText } from './linkFacts';
  import { embeds } from './linkPreviews';

  let { url }: { url: string } = $props();

  let el = $state<HTMLElement>();
  let info = $state<LinkPreview | null>(null);
  let phase = $state<CardState>('wait');
  let broken = $state(false);
  const host = $derived(hostOf(url));
  const lines = $derived(cardLines(info));

  /** The nearest scrolling ancestor's box, or the window's. */
  function viewport(node: HTMLElement): { top: number; bottom: number } {
    for (let p = node.parentElement; p; p = p.parentElement) {
      const y = getComputedStyle(p).overflowY;
      if (y === 'auto' || y === 'scroll') {
        const r = p.getBoundingClientRect();
        return { top: r.top, bottom: r.bottom };
      }
    }
    return { top: 0, bottom: innerHeight };
  }

  function arrived(node: HTMLElement, i: LinkPreview | null) {
    const r = node.getBoundingClientRect();
    const v = viewport(node);
    info = i;
    phase = cardAfter(i, r.bottom > v.top && r.top < v.bottom);
  }

  $effect(() => {
    if (!el) return;
    const node = el;
    let gone = false;
    // Asked a screen ahead, so the card is filled while it is still off screen.
    const seen = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return;
        seen.disconnect();
        embeds
          .get(url)
          .then((i) => !gone && arrived(node, i))
          .catch(() => !gone && arrived(node, null));
      },
      { rootMargin: '600px 0px' },
    );
    seen.observe(node);
    return () => {
      gone = true;
      seen.disconnect();
    };
  });
</script>

<div class="slot" bind:this={el} class:gone={phase === 'gone'} style:--card-h={CARD_HEIGHT}>
  {#if phase === 'ready'}
    <a class="inner" href={url} data-link={url} data-host={host}>
      {#if info?.thumbnail}
        <span class="pic" style:width="{CARD_PIC.w}px" style:height="{CARD_PIC.h}px">
          {#if !broken}<img
              class:blur={info.nsfw}
              src={info.thumbnail}
              width={CARD_PIC.w}
              height={CARD_PIC.h}
              alt=""
              loading="lazy"
              referrerpolicy="no-referrer"
              onerror={() => (broken = true)}
            />{/if}
        </span>
      {/if}
      <span class="text">
        {#each lines as line, i (i)}
          <span class:first={i === 0}>{line}</span>
        {/each}
        <span class="host">{info?.kind === 'web' ? host : info ? kindText(info) : host}</span>
      </span>
    </a>
  {:else if phase === 'plain'}
    <a class="inner plain" href={url} data-link={url} data-host={host}>
      <span class="text"><span class="host">{host}</span><span>No preview</span></span>
    </a>
  {:else if phase === 'wait'}
    <div class="inner skel" role="status" aria-label="Loading preview">
      <span class="pic" style:width="{CARD_PIC.w}px" style:height="{CARD_PIC.h}px"></span>
      <span class="text">
        <span class="bar" style:width="70%"></span>
        <span class="bar" style:width="45%"></span>
        <span class="bar" style:width="25%"></span>
      </span>
    </div>
  {/if}
</div>

<style>
  .slot.gone {
    display: none;
  }
  .inner {
    box-sizing: content-box;
    display: flex;
    gap: 8px;
    height: var(--card-h);
    max-width: 420px;
    margin: 4px 0 2px;
    padding: 6px;
    overflow: hidden;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.85em;
    text-decoration: none;
  }
  a.inner:hover {
    border-color: var(--border-strong);
  }
  .pic {
    flex: none;
    overflow: hidden;
    background: var(--surface-hover);
    border-radius: var(--radius);
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .blur {
    filter: blur(10px);
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    justify-content: center;
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
  .bar {
    display: block;
    height: 0.8em;
    margin: 0.3em 0;
    background: var(--surface-hover);
    border-radius: var(--radius-sm);
  }
</style>
