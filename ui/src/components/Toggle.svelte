<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    /** `neutral`: "on" is grey, not the accent, for long lists of switches. */
    variant?: 'accent' | 'neutral';
    onchange: (checked: boolean) => void;
  }

  let { checked, label, disabled = false, variant = 'accent', onchange }: Props = $props();
</script>

<button
  class="toggle"
  class:on={checked}
  class:neutral={variant === 'neutral'}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 36px;
    height: 20px;
    flex: none;
    padding: 0;
    background: var(--border-strong);
    border: 0;
    border-radius: var(--radius-pill);
    transition: background var(--fast);
  }
  /* Light mode: the off track must read against white. */
  :global(.theme-light) .toggle:not(.on) {
    background: var(--dim);
  }
  .toggle.on {
    background: var(--accent);
  }
  .toggle.neutral.on {
    background: var(--muted);
  }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--on-solid);
    transition: transform var(--fast) var(--ease);
  }
  .on .knob {
    transform: translateX(16px);
    background: var(--accent-ink);
  }
  .neutral.on .knob {
    background: var(--bg);
  }
  .toggle:disabled {
    opacity: 0.5;
  }
</style>
