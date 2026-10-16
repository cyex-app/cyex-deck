<!-- N1 A switch whose "on" is neutral, not the accent: for long lists of them, where a wall
     of accent blue is noise. Same size and behaviour as components/Toggle.svelte. Until
     Toggle has a `neutral` variant this stands in for it. -->
<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  }

  let { checked, label, disabled = false, onchange }: Props = $props();
</script>

<button
  class="toggle"
  class:on={checked}
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
  .toggle.on {
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
    background: var(--bg);
  }
  .toggle:disabled {
    opacity: 0.5;
  }
</style>
