<script lang="ts" generics="T extends string">
  interface Props {
    options: readonly { value: T; label: string }[];
    value: T;
    label: string;
    onchange: (value: T) => void;
  }

  let { options, value, label, onchange }: Props = $props();
</script>

<div class="seg" role="radiogroup" aria-label={label}>
  {#each options as option (option.value)}
    <button role="radio" aria-checked={value === option.value} class:on={value === option.value} onclick={() => onchange(option.value)}>
      {option.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  button {
    height: 24px;
    padding: 0 10px;
    background: none;
    border: 0;
    border-radius: 2px;
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
    transition:
      background var(--fast),
      color var(--fast);
  }
  button:hover {
    color: var(--text);
  }
  button.on {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
</style>
