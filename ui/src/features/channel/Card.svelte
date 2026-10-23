<!-- A clip or video in a channel's lists (1.21): picture with its length and views on it, and
     how far a VOD was watched (V107-Mv). -->
<script lang="ts">
  interface Props {
    image: string | null;
    title: string;
    sub: string;
    /** Bottom right of the picture, e.g. the length. */
    corner: string;
    /** Bottom left, e.g. views. */
    views: string;
    /** Top left, e.g. LIVE or Subs. */
    badge?: string | null;
    /** V107-Mv 0 to 1, a thin bar along the bottom. */
    progress?: number | null;
    /** V107-Mv Seen to 90 % or more. */
    watched?: boolean;
    onclick: () => void;
  }

  let { image, title, sub, corner, views, badge = null, progress = null, watched = false, onclick }: Props = $props();
</script>

<button class="card" {onclick}>
  <div class="pic">
    {#if image}<img src={image} alt="" loading="lazy" decoding="async" />{/if}
    {#if badge}<span class="tag badge">{badge}</span>{/if}
    <span class="tag views">{views}</span>
    <span class="tag corner">{corner}</span>
    {#if watched}<span class="tag seen">Watched</span>{/if}
    {#if progress}<span class="bar" style:--part={Math.min(1, progress)}></span>{/if}
  </div>
  <span class="title" title={title}>{title || 'Untitled'}</span>
  <span class="sub">{sub}</span>
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    padding: 0;
    background: none;
    border: 0;
    color: var(--text);
    text-align: left;
  }
  .pic {
    position: relative;
    aspect-ratio: 16 / 9;
    background: var(--surface);
    border-radius: var(--radius);
    overflow: hidden;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform var(--base) var(--ease);
  }
  .card:hover img {
    transform: scale(1.03);
  }
  .tag {
    position: absolute;
    padding: 0 5px;
    background: var(--scrim-strong);
    border-radius: var(--radius);
    color: var(--on-video);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .badge,
  .seen {
    max-width: calc(50% - 9px);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    top: 6px;
    left: 6px;
    background: var(--live);
    font-weight: 700;
  }
  .views {
    bottom: 6px;
    left: 6px;
  }
  .corner {
    bottom: 6px;
    right: 6px;
  }
  .seen {
    top: 6px;
    right: 6px;
  }
  .bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    background: color-mix(in srgb, var(--on-video) 30%, transparent);
  }
  .bar::after {
    content: "";
    position: absolute;
    inset: 0;
    transform: scaleX(var(--part));
    transform-origin: left;
    background: var(--accent);
  }
  .title {
    font-weight: 600;
    font-size: 0.85rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    color: var(--dim);
    font-size: 0.75rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
