<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    icon: IconName;
    label: string;
    hint?: string;
    active?: boolean;
    danger?: boolean;
    disabled?: boolean;
    size?: number;
    /** Window-control look: wide, flat, full height. */
    chrome?: boolean;
    /** A small dot in the corner, e.g. the miner at work. */
    badge?: boolean;
    onclick?: (e: MouseEvent) => void;
  }

  let {
    icon,
    label,
    hint,
    active = false,
    danger = false,
    disabled = false,
    size = 16,
    chrome = false,
    badge = false,
    onclick,
  }: Props = $props();
</script>

<button
  class="icon-btn"
  class:active
  class:danger
  class:chrome
  aria-label={label}
  aria-pressed={active}
  {disabled}
  {onclick}
  use:tooltip={{ label, hint }}
>
  <Icon name={icon} {size} />
  {#if badge}<span class="badge"></span>{/if}
</button>

<style>
  .icon-btn {
    position: relative;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    transition:
      background var(--fast),
      color var(--fast);
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }
  .icon-btn.active {
    color: var(--accent-text);
    background: var(--accent-dim);
  }
  .icon-btn:disabled {
    opacity: 0.4;
  }
  .chrome {
    width: 44px;
    height: var(--chrome-h);
    border-radius: 0;
  }
  .chrome.active {
    background: none;
  }
  .chrome.danger:hover {
    background: #e81123;
    color: #fff;
  }
  .badge {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
  }
</style>
