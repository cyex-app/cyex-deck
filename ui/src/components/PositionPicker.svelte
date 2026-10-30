<!-- A radio group of small window drawings: pick the side the chat or the tab bar sits on. -->
<script lang="ts" generics="T extends Side">
  import { blocks, type Side } from './position';
  import { radioKey } from './select';

  interface Props {
    options: readonly { value: T; label: string }[];
    value: T;
    label: string;
    onchange: (value: T) => void;
    /** What the highlighted block stands for: a wide chat column or a thin tab strip. */
    kind: 'chat' | 'tabs';
  }

  let { options, value, label, onchange, kind }: Props = $props();

  // Roving tabindex like Segmented: one tab stop (the picked tile); arrows move and pick.
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
<div class="pos" role="radiogroup" aria-label={label} bind:this={group} {onkeydown}>
  {#each options as option, i (option.value)}
    {@const g = blocks(option.value, kind)}
    <button
      role="radio"
      aria-checked={value === option.value}
      tabindex={i === current ? 0 : -1}
      class:on={value === option.value}
      onclick={() => onchange(option.value)}
    >
      <svg viewBox="0 0 56 38" width="56" height="38" aria-hidden="true">
        <rect class="frame" x="0.5" y="0.5" width="55" height="37" rx="2.5" />
        <rect class="bar" x="1" y="1" width="54" height="5" />
        <rect class="rest" x={g.rest.x} y={g.rest.y} width={g.rest.w} height={g.rest.h} />
        <rect class="block" x={g.block.x + 1} y={g.block.y + 1} width={g.block.w - 2} height={g.block.h - 2} rx="1" />
      </svg>
      <span>{option.label}</span>
    </button>
  {/each}
</div>

<style>
  .pos {
    display: inline-flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
    max-width: 100%;
  }
  button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px 6px 4px;
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 600;
    transition:
      background var(--fast),
      border-color var(--fast),
      color var(--fast);
  }
  button:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }
  button.on {
    background: var(--accent-dim);
    border-color: var(--accent-text);
    color: var(--accent-text);
  }
  svg {
    display: block;
  }
  .frame {
    fill: var(--surface);
    stroke: var(--border-strong);
  }
  .bar {
    fill: var(--border-strong);
  }
  .rest {
    fill: var(--elevated-2);
  }
  .block {
    fill: var(--dim);
  }
  button.on .block {
    fill: var(--accent-text);
  }
  /* The pale wash vanishes on light: the player area gets a tint of its own. */
  :global(:root[data-theme='light']) .rest {
    fill: var(--neutral-chip);
  }
  @media (prefers-contrast: more) {
    button.on {
      border-color: var(--text);
    }
  }
  @media (forced-colors: active) {
    .frame,
    .bar {
      fill: Canvas;
      stroke: ButtonText;
    }
    .rest {
      fill: Canvas;
    }
    .block,
    button.on .block {
      fill: ButtonText;
    }
    button.on {
      background: Highlight;
      border-color: Highlight;
      color: HighlightText;
      forced-color-adjust: none;
    }
    button.on .frame,
    button.on .bar {
      fill: Highlight;
      stroke: HighlightText;
    }
    button.on .rest {
      fill: Highlight;
    }
    button.on .block {
      fill: HighlightText;
    }
  }
</style>
