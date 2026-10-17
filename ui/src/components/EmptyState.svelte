<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  interface Props {
    icon: IconName;
    text: string;
    /** `start`: icon and one line, left-aligned, for inside a page of settings. */
    align?: 'center' | 'start';
    children?: Snippet;
  }

  let { icon, text, align = 'center', children }: Props = $props();
</script>

<div class="empty" class:start={align === 'start'}>
  <Icon name={icon} size={align === 'start' ? 20 : 28} />
  <p>{text}</p>
  {@render children?.()}
</div>

<style>
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--dim);
  }
  .start {
    flex: none;
    flex-direction: row;
    justify-content: flex-start;
    gap: 10px;
    padding: 10px 0;
    font-size: 0.85rem;
  }
  .start :global(svg) {
    flex: none;
  }
  p {
    margin: 0;
  }
</style>
