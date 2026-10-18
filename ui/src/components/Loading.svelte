<!-- Something is on its way. A small spinner with an optional word, centered in the room it
     will take (`minHeight`, so the list doesn't jump when it arrives), or `rows` skeleton
     rows shaped like a list of avatars and names. `video` draws it white over a player.
     Reduce motion: the spinner and the shimmer hold still. -->
<script lang="ts">
  interface Props {
    /** Under the spinner, e.g. "Loading…"; screen readers hear it either way. */
    label?: string;
    /** Skeleton rows instead of the spinner. */
    rows?: number;
    /** Skeleton cards in the real grid's shape. */
    cards?: number;
    /** 'stream': Home and Discover cards (picture, avatar, three lines). 'media': videos and clips (picture, two lines). */
    kind?: 'stream' | 'media';
    size?: 'small' | 'large';
    video?: boolean;
    minHeight?: string;
  }

  let { label = '', rows = 0, cards = 0, kind = 'stream', size = 'small', video = false, minHeight }: Props = $props();
</script>

{#if cards > 0}
  <div class="cards {kind}" role="status" aria-label={label || 'Loading'} style:min-height={minHeight}>
    {#each { length: cards } as _, i (i)}
      <div class="card" aria-hidden="true">
        <span class="blob thumb"></span>
        {#if kind === 'stream'}
          <div class="info">
            <span class="blob av"></span>
            <div class="text">
              <span class="ln name"><span class="blob" style:width="{50 - ((i * 7) % 20)}%"></span></span>
              <span class="ln title"><span class="blob" style:width="{85 - ((i * 13) % 30)}%"></span></span>
              <span class="ln game"><span class="blob" style:width="35%"></span></span>
            </div>
          </div>
        {:else}
          <span class="ln title"><span class="blob" style:width="{90 - ((i * 13) % 30)}%"></span></span>
          <span class="ln sub"><span class="blob" style:width="55%"></span></span>
        {/if}
      </div>
    {/each}
  </div>
{:else if rows > 0}
  <div class="skeleton" role="status" aria-label={label || 'Loading'} style:min-height={minHeight}>
    {#each { length: rows } as _, i (i)}
      <div class="row" aria-hidden="true">
        <span class="blob round"></span>
        <span class="lines">
          <span class="blob line" style:width="{60 - ((i * 17) % 30)}%"></span>
          <span class="blob line short"></span>
        </span>
      </div>
    {/each}
  </div>
{:else}
  <div class="loading {size}" class:video role="status" style:min-height={minHeight}>
    <span class="spinner" aria-hidden="true"></span>
    {#if label}<span class="label">{label}</span>{:else}<span class="sr-only">Loading</span>{/if}
  </div>
{/if}

<style>
  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 16px;
    color: var(--dim);
    font-size: 0.85rem;
  }
  .spinner {
    --ring: var(--border-strong);
    --head: var(--muted);
    width: 18px;
    height: 18px;
    border: 2px solid var(--ring);
    border-top-color: var(--head);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }
  .large .spinner {
    width: 36px;
    height: 36px;
    border-width: 3px;
  }
  .video {
    color: var(--on-video-dim);
  }
  .video .spinner {
    --ring: var(--on-video-faint);
    --head: var(--on-video);
  }
  /* Same grid, gaps, picture ratio and text rows as StreamCard / channel Card, so nothing
     jumps when the real cards arrive. */
  .cards {
    display: grid;
  }
  .cards.stream {
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 20px 14px;
  }
  .cards.media {
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 14px 12px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .media .card {
    gap: 4px;
  }
  .thumb {
    aspect-ratio: 16 / 9;
    width: 100%;
    border-radius: var(--radius);
  }
  .info {
    display: flex;
    gap: 10px;
    min-width: 0;
  }
  .av {
    flex: none;
    width: 32px;
    height: 32px;
    border-radius: 50%;
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  /* Each row takes the height of the text it stands for. */
  .ln {
    display: flex;
    align-items: center;
    line-height: 1.35;
  }
  .ln .blob {
    height: 0.7em;
  }
  .ln.name,
  .ln.title {
    font-size: 0.85rem;
  }
  .stream .ln.title {
    font-size: 0.8rem;
  }
  .ln.game {
    font-size: 0.78rem;
  }
  .ln.sub {
    font-size: 0.75rem;
  }
  .skeleton {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 10px;
  }
  .lines {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 6px;
  }
  .blob {
    display: block;
    background: linear-gradient(90deg, var(--surface-hover) 25%, var(--elevated-2) 50%, var(--surface-hover) 75%);
    background-size: 300% 100%;
    border-radius: var(--radius-sm);
    animation: shimmer 1.4s ease-in-out infinite;
  }
  .round {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 50%;
  }
  .line {
    height: 9px;
  }
  .short {
    width: 30%;
    height: 7px;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: 0 0;
    }
  }
  /* The global rule zeroes the duration, which would leave the spinner at a random angle. */
  @media (prefers-reduced-motion: reduce) {
    .spinner,
    .blob {
      animation: none;
    }
  }
</style>
