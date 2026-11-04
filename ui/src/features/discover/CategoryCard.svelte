<!-- A category in discover (1.7): box art, name, viewers. -->
<script lang="ts">
  import type { Category } from '../../ipc/bindings/Category';
  import { openMenu } from '../../components/menu.svelte';
  import { formatShort } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { withHidden, withoutHidden } from './hidden';

  let { category, onopen }: { category: Category; onopen: (c: Category) => void } = $props();

  // V108-N05 Right-click: leave the category out of Home and Discover (Undo on the toast).
  function hide() {
    const before = settings.v.browse.hiddenCategories;
    void settings.patch('browse', { hiddenCategories: withHidden(before, { id: category.id, name: category.name }) });
    toast.info(`Hid ${category.name}.`, {
      key: 'hide-category',
      action: { label: 'Undo', run: () => void settings.patch('browse', { hiddenCategories: withoutHidden(settings.v.browse.hiddenCategories, category.id) }) },
    });
  }

  /** Shown when Twitch has no box art. */
  const initials = $derived(
    category.name
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w[0].toUpperCase())
      .join(''),
  );
</script>

<button
  class="cat"
  onclick={() => onopen(category)}
  oncontextmenu={(e) => (e.preventDefault(), openMenu(e, [{ label: 'Hide category', icon: 'eyeOff', run: hide }]))}
>
  <div class="art">
    {#if category.boxArt}<img src={category.boxArt} alt="" loading="lazy" decoding="async" />{:else}<span class="initials" aria-hidden="true">{initials}</span>{/if}
  </div>
  <span class="name" title={category.name}>{category.name}</span>
  <span class="viewers">{formatShort(category.viewers)} {category.viewers === 1 ? 'viewer' : 'viewers'}</span>
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
  .initials {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--dim);
    font-size: 1.4rem;
    font-weight: 700;
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
