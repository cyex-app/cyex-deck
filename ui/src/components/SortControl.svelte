<!-- V107-Mr Every list's sort: a key dropdown (Time, A–Z, …) and a small direction button
     beside it (↑ ascending, ↓ descending). A new key starts in its own direction (lib/sort). -->
<script lang="ts" generics="K extends string">
  import { flip, pickKey, sortWords, type Sort, type SortKey } from '../lib/sort';
  import IconButton from './IconButton.svelte';
  import Select from './Select.svelte';

  interface Props {
    keys: readonly SortKey<K>[];
    sort: Sort<K>;
    onchange: (sort: Sort<K>) => void;
    /** Names the dropdown for screen readers, e.g. "Sort followers". */
    label?: string;
    width?: string;
  }

  let { keys, sort, onchange, label = 'Sort', width = '88px' }: Props = $props();

  const now = $derived(sortWords(keys, sort));
  /** A `%` width fills what the row gives it; the button keeps its size. */
  const grow = $derived(width.endsWith('%'));
</script>

<span class="sort" class:grow role="group" aria-label={label}>
  {#if keys.length > 1}
    <span class="key"><Select {label} {width} value={sort.key} options={keys} onchange={(k) => onchange(pickKey(keys, sort, k as K))} /></span>
  {/if}
  <IconButton
    icon={sort.desc ? 'arrowDown' : 'arrowUp'}
    label={now}
    hint="Click to flip."
    size={15}
    onclick={() => onchange(flip(sort))}
  />
</span>

<style>
  .sort {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    flex: none;
    min-width: 0;
  }
  .grow {
    display: flex;
    flex: 1 1 120px;
  }
  .key {
    display: flex;
    min-width: 0;
  }
  .grow .key {
    flex: 1;
  }
</style>
