<!-- A number box in the app's look (`.field`), no OS spinners. Typing is free; Enter or
     leaving the box keeps the number, clamped to min..max and the step (a comma works as the
     decimal mark). Up/Down step, Shift or Page keys by ten, Home/End jump; each step saves.
     Something that isn't a number goes back to the last value. -->
<script lang="ts">
  import { onDestroy } from 'svelte';

  import { formatNumber } from '../lib/format';
  import { parseNumber, rangeNote, stepFor } from './number.ts';

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
    /** Up/down buttons and mouse-wheel steps while focused, and a short "Max 5.000" note when a typed number was pulled into range. */
    steppers?: boolean;
    onchange: (value: number) => void;
    /** Given: text that isn't a number stays in the box (flagged, not reverted) so the form
     *  can block its save; Escape or a number clears it. */
    oninvalid?: (bad: boolean) => void;
  }

  let { value, min, max, step = 1, label, unit = '', width = '84px', disabled = false, steppers = false, onchange, oninvalid }: Props = $props();

  let text = $state('');
  let editing = $state(false);

  const range = $derived({ min, max, step });
  const shown = $derived(Number.isInteger(value) ? formatNumber(value) : String(value).replace('.', ','));

  let note = $state('');
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  function setNote(t: string) {
    clearTimeout(noteTimer);
    note = t;
    if (t) noteTimer = setTimeout(() => (note = ''), 4000);
  }

  onDestroy(() => clearTimeout(noteTimer)); // V110-F24

  function nudge(dir: 1 | -1) {
    const base = editing ? (parseNumber(text, range) ?? value) : value;
    const next = stepFor(dir > 0 ? 'ArrowUp' : 'ArrowDown', false, base, range);
    editing = false;
    setNote('');
    if (next !== null && next !== value) onchange(next);
  }

  function commit() {
    if (!editing) return;
    if (steppers) setNote(rangeNote(text, range, formatNumber) ?? '');
    const n = parseNumber(text, range);
    if (n === null && oninvalid) {
      oninvalid(true);
      return;
    }
    editing = false;
    oninvalid?.(false);
    if (n !== null && n !== value) onchange(n);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') return commit();
    if (e.key === 'Escape' && editing) {
      // Back to the saved number; a second Escape is the layer's.
      e.preventDefault();
      e.stopPropagation();
      editing = false;
      oninvalid?.(false);
      return;
    }
    const base = editing ? (parseNumber(text, range) ?? value) : value;
    const next = stepFor(e.key, e.shiftKey, base, range);
    if (next === null) return;
    e.preventDefault();
    editing = false;
    oninvalid?.(false);
    if (next !== value) onchange(next);
  }
</script>

<span class="wrap">
<span class="number field" class:disabled class:steppers style:width>
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
      setNote('');
      oninvalid?.(parseNumber(text, range) === null);
    }}
    onfocus={(e) => e.currentTarget.select()}
    onblur={commit}
    {onkeydown}
    onwheel={steppers
      ? (e) => {
          if (document.activeElement !== e.currentTarget) return;
          e.preventDefault();
          nudge(e.deltaY < 0 ? 1 : -1);
        }
      : undefined}
  />
  {#if unit}<span class="unit">{unit}</span>{/if}
  {#if steppers}
    <span class="steps">
      <button type="button" tabindex="-1" aria-label="More" disabled={disabled || value >= max} onmousedown={(e) => e.preventDefault()} onclick={() => nudge(1)}>
        <svg viewBox="0 0 8 5" aria-hidden="true"><path d="M1 4l3-3 3 3" /></svg>
      </button>
      <button type="button" tabindex="-1" aria-label="Less" disabled={disabled || value <= min} onmousedown={(e) => e.preventDefault()} onclick={() => nudge(-1)}>
        <svg viewBox="0 0 8 5" aria-hidden="true"><path d="M1 1l3 3 3-3" /></svg>
      </button>
    </span>
  {/if}
</span>
{#if note}<span class="note" role="status">{note}</span>{/if}
</span>

<style>
  .wrap {
    position: relative;
    display: inline-flex;
    max-width: 100%;
  }
  .note {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 2px;
    font-size: 0.78rem;
    color: var(--muted);
    white-space: nowrap;
    animation: note-in var(--fast, 120ms) ease-out;
    pointer-events: none;
  }
  @keyframes note-in {
    from {
      opacity: 0;
    }
  }
  .steppers {
    padding-right: 0;
  }
  .steps {
    flex: none;
    display: flex;
    flex-direction: column;
    height: 100%;
    border-left: 1px solid var(--border-strong);
  }
  .steps button {
    flex: 1;
    width: 20px;
    padding: 0;
    display: grid;
    place-items: center;
    background: none;
    border: 0;
    color: var(--muted);
  }
  .steps button + button {
    border-top: 1px solid var(--border-strong);
  }
  .steps button:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface-hover);
  }
  .steps button:disabled {
    opacity: 0.35;
  }
  .steps svg {
    width: 8px;
    height: 5px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
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
