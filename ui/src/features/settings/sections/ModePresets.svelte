<!-- V110-M23 An editable list of lengths for a mode (slow mode, followers only, ads): chips
     with ×, and a field that takes a typed one. Same look as the timeout buttons. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Row from '../../../components/Row.svelte';
  import type { Parsed } from '../../modview/modePresets';
  import { withPreset } from '../../modview/modePresets';

  interface Props {
    setting: string;
    label: string;
    hint: string;
    values: number[];
    /** A stored value as the chip shows it. */
    show: (v: number) => string;
    /** What a typed length means, or why not. */
    parse: (text: string) => Parsed;
    placeholder: string;
    onsave: (list: number[]) => void;
    max?: number;
    /** Values that are always there (not removable). */
    keep?: number[];
  }

  let { setting, label, hint, values, show, parse, placeholder, onsave, max = 10, keep = [] }: Props = $props();

  let text = $state('');
  let addBox = $state<HTMLInputElement>();
  let error = $state('');

  /** V110-F24 The chip with focus is gone after its ×: focus moves to the add field. */
  function remove(v: number) {
    onsave(values.filter((x) => x !== v));
    void tick().then(() => addBox?.focus());
  }

  function add() {
    const p = parse(text);
    if ('error' in p) {
      error = p.error;
      return;
    }
    onsave(withPreset(values, p.value, max));
    text = '';
  }
</script>

<Row {setting} {label} {hint} stacked>
  {#each values as v (v)}
    <span class="chip">
      {show(v)}
      {#if !keep.includes(v) && values.length > 1}
        <button aria-label="Remove {show(v)}" onclick={() => remove(v)}>×</button>
      {/if}
    </span>
  {/each}
  {#if values.length < max}
    <form
      onsubmit={(e) => {
        e.preventDefault();
        add();
      }}
    >
      <input
        bind:this={addBox}
        class="field add"
        class:bad={!!error}
        aria-label="Add a length to {label}"
        aria-invalid={!!error}
        {placeholder}
        bind:value={text}
        oninput={() => (error = '')}
      />
    </form>
  {/if}
  {#if error}<span class="field-error" role="alert">{error}</span>{/if}
</Row>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 4px 0 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    font: 600 0.82rem var(--font-mono);
  }
  .chip button {
    position: relative;
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .chip button::before {
    content: '';
    position: absolute;
    inset: -3px;
  }
  .chip button:hover {
    color: var(--red);
  }
  .add {
    width: 130px;
    height: 28px;
  }
  .field.bad,
  .field.bad:focus-visible {
    border-color: var(--red);
    box-shadow: 0 0 0 1px var(--red);
  }
  .field-error {
    white-space: nowrap;
    color: var(--red);
    font-size: 0.76rem;
    font-weight: 400;
  }
</style>
