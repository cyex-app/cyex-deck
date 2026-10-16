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
    /** Skeleton stream cards in the grid's shape (picture, two lines). */
    cards?: number;
    size?: 'small' | 'large';
    video?: boolean;
    minHeight?: string;
  }

  let { label = '', rows = 0, cards = 0, size = 'small', video = false, minHeight }: Props = $props();
</script>

{#if cards > 0}
  <div class="cards" role="status" aria-label={label || 'Loading'} style:min-height={minHeight}>
    {#each { length: cards } as _, i (i)}
      <div class="card" aria-hidden="true">
        <span class="blob thumb"></span>
        <span class="blob line" style:width="{70 - ((i * 13) % 30)}%"></span>
        <span class="blob line short"></span>
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
    {#if label}<span class="label">{label}</span>{:else}<span class="sr">Loading</span>{/if}
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
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 16px 12px;
    padding: 4px 0;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .thumb {
    aspect-ratio: 16 / 9;
    width: 100%;
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
