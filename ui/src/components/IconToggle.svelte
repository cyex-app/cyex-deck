<!-- V105-18 An on/off option as a small icon: tinted while on, dim while off. The tooltip
     names it and says what it does. -->
<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    icon: IconName;
    label: string;
    hint?: string;
    pressed: boolean;
    onchange: (pressed: boolean) => void;
    /** A click does something else than toggling, e.g. opens a menu. */
    onclick?: (e: MouseEvent) => void;
  }

  let { icon, label, hint, pressed, onchange, onclick }: Props = $props();
</script>

<button
  type="button"
  class="opt"
  class:on={pressed}
  aria-label={label}
  aria-pressed={pressed}
  onclick={(e) => (onclick ? onclick(e) : onchange(!pressed))}
  use:tooltip={{ label, hint }}
>
  <Icon name={icon} size={15} />
</button>

<style>
  .opt {
    display: grid;
    place-items: center;
    flex: none;
    width: 26px;
    height: 26px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--dim);
    opacity: 0.75;
    transition:
      background var(--fast),
      color var(--fast),
      opacity var(--fast);
  }
  .opt:hover {
    background: var(--surface-hover);
    color: var(--text);
    opacity: 1;
  }
  .on {
    background: var(--accent-dim);
    border-color: var(--accent-border);
    color: var(--text);
    opacity: 1;
  }
  .on:hover {
    background: var(--accent-wash);
  }
  .opt:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
</style>
