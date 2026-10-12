<script lang="ts">
  interface Props {
    value: number;
    min: number;
    max: number;
    unit?: string;
    label: string;
    onchange: (value: number) => void;
  }

  let { value, min, max, unit = '', label, onchange }: Props = $props();

  const set = (next: number) => onchange(Math.min(Math.max(next, min), max));
</script>

<div class="stepper" role="group" aria-label={label}>
  <button aria-label="Less" disabled={value <= min} onclick={() => set(value - 1)}>−</button>
  <span>{value}{unit}</span>
  <button aria-label="More" disabled={value >= max} onclick={() => set(value + 1)}>+</button>
</div>

<style>
  .stepper {
    display: inline-flex;
    align-items: center;
    height: 30px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--surface);
  }
  button {
    width: 30px;
    height: 100%;
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 1rem;
  }
  button:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface-hover);
  }
  button:disabled {
    opacity: 0.35;
  }
  span {
    min-width: 44px;
    text-align: center;
    font: 600 0.85rem var(--font-mono);
  }
</style>
