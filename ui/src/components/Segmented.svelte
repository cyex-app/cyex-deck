<script lang="ts" generics="T extends string | number">
  import { radioKey } from './select';

  interface Props {
    options: readonly { value: T; label: string }[];
    value: T;
    label: string;
    onchange: (value: T) => void;
    /** A neutral filled state instead of the accent tint. */
    neutral?: boolean;
    /** Wrap onto a second line instead of scrolling sideways. */
    wrap?: boolean;
  }

  let { options, value, label, onchange, neutral = false, wrap = false }: Props = $props();

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
<div class="seg" class:neutral class:wrap role="radiogroup" aria-label={label} bind:this={group} {onkeydown}>
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
  .seg.wrap {
    flex-wrap: wrap;
    overflow: visible;
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
      box-shadow var(--fast),
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
  /* V107-H18 The pale wash vanishes on light: a grey track, a solid raised pick with a ring (3:1). */
  :global(:root[data-theme='light']) .seg {
    background: var(--neutral-chip);
  }
  :global(:root[data-theme='light']) button.on {
    background: var(--elevated);
    box-shadow:
      0 0 0 1px var(--accent-text),
      0 1px 2px var(--shadow);
    color: var(--accent-text);
  }
  :global(:root[data-theme='light']) .neutral button.on {
    box-shadow:
      0 0 0 1px var(--muted),
      0 1px 2px var(--shadow);
    color: var(--text);
  }
  /* F9 More contrast asked for: the track and the pick get solid borders. */
  @media (prefers-contrast: more) {
    .seg {
      border-color: var(--text);
    }
    button.on,
    .neutral button.on,
    :global(:root[data-theme='light']) button.on,
    :global(:root[data-theme='light']) .neutral button.on {
      box-shadow: 0 0 0 1px var(--text);
    }
  }
  /* Forced colours: the pick is a Highlight block with HighlightText. */
  @media (forced-colors: active) {
    .seg {
      background: Canvas;
      border-color: ButtonText;
    }
    button.on,
    .neutral button.on,
    :global(:root[data-theme='light']) button.on,
    :global(:root[data-theme='light']) .neutral button.on {
      background: Highlight;
      box-shadow: none;
      color: HighlightText;
    }
  }
</style>
