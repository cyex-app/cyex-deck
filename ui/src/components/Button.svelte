<script lang="ts">
  import type { Snippet } from 'svelte';

  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  interface Props {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    small?: boolean;
    icon?: IconName;
    disabled?: boolean;
    type?: 'button' | 'submit';
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
  }

  let { variant = 'secondary', small = false, icon, disabled = false, type = 'button', onclick, children }: Props =
    $props();
</script>

<button class="btn {variant}" class:small {type} {disabled} {onclick}>
  {#if icon}<Icon name={icon} size={small ? 14 : 15} />{/if}
  {@render children()}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 32px;
    padding: 0 14px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    font-size: 0.87rem;
    font-weight: 600;
    white-space: nowrap;
    transition:
      background var(--fast),
      border-color var(--fast),
      color var(--fast);
  }
  .small {
    height: 27px;
    padding: 0 10px;
    font-size: 0.8rem;
  }
  .primary {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .secondary {
    background: var(--surface);
    border-color: var(--border-strong);
  }
  .secondary:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .ghost {
    background: none;
    color: var(--muted);
  }
  .ghost:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }
  .danger {
    background: none;
    border-color: var(--border-strong);
    color: var(--red);
  }
  .danger:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .btn:disabled {
    opacity: 0.5;
  }
</style>
