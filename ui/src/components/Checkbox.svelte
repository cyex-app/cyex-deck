<script lang="ts">
  import Icon from './Icon.svelte';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    checked: boolean;
    label: string;
    /** Tooltip, e.g. the long name of a short label. */
    hint?: string;
    disabled?: boolean;
    /** A dash instead of a tick: some, not all, of a group. Shown only while unchecked. */
    indeterminate?: boolean;
    onchange: (checked: boolean) => void;
  }

  let { checked, label, hint, disabled = false, indeterminate = false, onchange }: Props = $props();
</script>

<label class="check" class:disabled use:tooltip={hint ?? ''}>
  <input type="checkbox" {checked} {disabled} indeterminate={indeterminate && !checked} onchange={(e) => onchange(e.currentTarget.checked)} />
  <span class="box">{#if checked}<Icon name="check" size={12} />{:else if indeterminate}<span class="dash"></span>{/if}</span>
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
  .dash {
    width: 8px;
    height: 2px;
    border-radius: 1px;
    background: currentColor;
  }
  input:checked + .box,
  input:indeterminate + .box {
    background: var(--accent);
    border-color: var(--accent-line);
  }
  input:focus-visible + .box {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
</style>
