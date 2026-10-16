<!-- A category in discover (1.7): box art, name, viewers. -->
<script lang="ts">
  import type { Category } from '../../ipc/bindings/Category';
  import { formatShort } from '../../lib/format';

  let { category, onopen }: { category: Category; onopen: (c: Category) => void } = $props();
</script>

<button class="cat" onclick={() => onopen(category)}>
  <div class="art">
    {#if category.boxArt}<img src={category.boxArt} alt="" loading="lazy" decoding="async" />{/if}
  </div>
  <span class="name" title={category.name}>{category.name}</span>
  <span class="viewers">{formatShort(category.viewers)} viewers</span>
</button>

<style>
  .cat {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    padding: 0;
    background: none;
    border: 0;
    color: var(--text);
    text-align: left;
  }
  .art {
    aspect-ratio: 3 / 4;
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
  .cat:hover img {
    transform: scale(1.03);
  }
  .name {
    margin-top: 3px;
    font-weight: 600;
    font-size: 0.82rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .viewers {
    color: var(--dim);
    font-size: 0.75rem;
  }
</style>
