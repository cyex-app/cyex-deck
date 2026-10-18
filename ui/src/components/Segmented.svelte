<script lang="ts" generics="T extends string | number">
  import { radioKey } from './select';

  interface Props {
    options: readonly { value: T; label: string }[];
    value: T;
    label: string;
    onchange: (value: T) => void;
    /** A neutral filled state instead of the accent tint. */
    neutral?: boolean;
  }

  let { options, value, label, onchange, neutral = false }: Props = $props();

  // Roving tabindex: one tab stop (the picked option); arrows move and pick.
  const current = $derived(Math.max(options.findIndex((o) => o.value === value), 0));
  let group = $state<HTMLElement>();

  function onkeydown(e: KeyboardEvent) {
    const to = radioKey(e, current, options.length);
    if (to === null) return;
    e.preventDefault();
    onchange(options[to].value);
    (group?.children[to] as HTMLElement | undefined)?.focus();
  }
</script>

<!-- svelte-ignore a11y_interactive_supports_focus (the radios inside hold the tab stop) -->
<div class="seg" class:neutral role="radiogroup" aria-label={label} bind:this={group} {onkeydown}>
  {#each options as option, i (option.value)}
    <button
      role="radio"
      aria-checked={value === option.value}
      tabindex={i === current ? 0 : -1}
      class:on={value === option.value}
      onclick={() => onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  /* C9 Always one line; the label beside it gives way. */
  .seg {
    display: inline-flex;
    flex-wrap: nowrap;
    max-width: 100%;
    /* Narrower than its options: scroll sideways rather than clip. */
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
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
    border-radius: var(--radius-sm);
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
    white-space: nowrap;
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
  .neutral button.on {
    box-shadow: 0 0 0 1px var(--border-strong);
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
