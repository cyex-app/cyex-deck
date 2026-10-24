<script lang="ts">
  import type { Snippet } from 'svelte';

  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  interface Props {
    /** `link`: accent text inside a sentence or a row, no box ("Show all", "Undo"). */
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'link';
    small?: boolean;
    icon?: IconName;
    disabled?: boolean;
    /** A taller invisible hit area (28 px) for a small link; the look and layout stay. */
    hit?: boolean;
    type?: 'button' | 'submit';
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
  }

  let { variant = 'secondary', small = false, icon, disabled = false, hit = false, type = 'button', onclick, children }: Props =
    $props();
</script>

<button class="btn {variant}" class:small class:hit {type} {disabled} {onclick}>
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
  /* Shaped like secondary, so it sits in a row of them; only the text is red. */
  .danger {
    background: var(--surface);
    border-color: var(--border-strong);
    color: var(--red);
  }
  .danger:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .link {
    height: auto;
    padding: 0;
    gap: 5px;
    background: none;
    color: var(--accent-text);
    font-weight: 500;
    border-radius: var(--radius-sm);
  }
  .hit {
    position: relative;
  }
  .hit::before {
    content: '';
    position: absolute;
    inset: -4px 0;
  }
  .link.small {
    height: auto;
    padding: 0;
  }
  .link:hover:not(:disabled) {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  /* R5 Pressed at once. */
  .link:active:not(:disabled) {
    opacity: 0.6;
    transition: none;
  }
  .btn:disabled {
    opacity: 0.5;
  }
</style>
