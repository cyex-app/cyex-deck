<script lang="ts">
  import Icon from './Icon.svelte';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    checked: boolean;
    label: string;
    /** Tooltip, e.g. the long name of a short label. */
    hint?: string;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  }

  let { checked, label, hint, disabled = false, onchange }: Props = $props();
</script>

<label class="check" class:disabled use:tooltip={hint ?? ''}>
  <input type="checkbox" {checked} {disabled} onchange={(e) => onchange(e.currentTarget.checked)} />
  <span class="box">{#if checked}<Icon name="check" size={12} />{/if}</span>
  {label}
</label>

<style>
  /* Holds its hidden input: without it the input sat outside any scrolling list, made the
     page taller, and focusing it (Tab, a click) scrolled the whole window up. */
  .check {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    cursor: pointer;
  }
  .disabled {
    opacity: 0.5;
    cursor: default;
  }
  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .box {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--accent-ink);
  }
  input:checked + .box {
    background: var(--accent);
    border-color: var(--accent);
  }
  input:focus-visible + .box {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
</style>
