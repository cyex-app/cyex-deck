<!-- A number box in the app's look (`.field`), no OS spinners. Typing is free; Enter or
     leaving the box keeps the number, clamped to min..max and the step (a comma works as the
     decimal mark). Up/Down step, Shift or Page keys by ten, Home/End jump; each step saves.
     Something that isn't a number goes back to the last value. -->
<script lang="ts">
  import { formatNumber } from '../lib/format';
  import { parseNumber, stepFor } from './number.ts';

  interface Props {
    value: number;
    min: number;
    max: number;
    step?: number;
    label: string;
    /** After the number, e.g. "s" or "%". */
    unit?: string;
    width?: string;
    disabled?: boolean;
    onchange: (value: number) => void;
  }

  let { value, min, max, step = 1, label, unit = '', width = '84px', disabled = false, onchange }: Props = $props();

  let text = $state('');
  let editing = $state(false);

  const range = $derived({ min, max, step });
  const shown = $derived(Number.isInteger(value) ? formatNumber(value) : String(value).replace('.', ','));

  function commit() {
    if (!editing) return;
    editing = false;
    const n = parseNumber(text, range);
    if (n !== null && n !== value) onchange(n);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') return commit();
    if (e.key === 'Escape' && editing) {
      // Back to the saved number; a second Escape is the layer's.
      e.preventDefault();
      e.stopPropagation();
      editing = false;
      return;
    }
    const base = editing ? (parseNumber(text, range) ?? value) : value;
    const next = stepFor(e.key, e.shiftKey, base, range);
    if (next === null) return;
    e.preventDefault();
    editing = false;
    if (next !== value) onchange(next);
  }
</script>

<span class="number field" class:disabled style:width>
  <input
    type="text"
    inputmode="decimal"
    role="spinbutton"
    aria-label={label}
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuenow={value}
    spellcheck="false"
    autocomplete="off"
    {disabled}
    value={editing ? text : shown}
    data-own-escape={editing ? '' : undefined}
    oninput={(e) => {
      editing = true;
      text = e.currentTarget.value;
    }}
    onfocus={(e) => e.currentTarget.select()}
    onblur={commit}
    {onkeydown}
  />
  {#if unit}<span class="unit">{unit}</span>{/if}
</span>

<style>
  .number {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    cursor: text;
  }
  .number:focus-within {
    border-color: var(--accent-border);
  }
  .disabled {
    opacity: 0.5;
  }
  input {
    flex: 1;
    min-width: 0;
    padding: 0;
    background: none;
    border: 0;
    /* The box's border shows focus. */
    outline: none;
    font-family: var(--font-ui);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .unit {
    flex: none;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
