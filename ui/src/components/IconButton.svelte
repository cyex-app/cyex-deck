<script lang="ts">
  import type { TokenKind } from '../ipc/bindings/TokenKind';
  import { FLAGS } from '../lib/gate';
  import { gate } from '../lib/gate.svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    icon: IconName;
    label: string;
    hint?: string;
    /** Shown pressed or current (accent tint). */
    active?: boolean;
    /** An on/off button: screen readers hear `active` as pressed. Leave it off for plain
     *  buttons and for nav buttons that only mark where you are. */
    toggle?: boolean;
    danger?: boolean;
    disabled?: boolean;
    size?: number;
    /** Window-control look: wide, flat, full height. */
    chrome?: boolean;
    /** A small dot in the corner, e.g. the miner at work. */
    badge?: boolean;
    /** V102-20 Greyed out with a lock: a click goes to the sign-in that unlocks it. */
    lock?: TokenKind | null;
    onclick?: (e: MouseEvent) => void;
  }

  let {
    icon,
    label,
    hint,
    active = false,
    toggle = false,
    danger = false,
    disabled = false,
    size = 16,
    chrome = false,
    badge = false,
    lock = null,
    onclick,
  }: Props = $props();

  function click(e: MouseEvent) {
    if (lock) gate.ask(lock);
    else onclick?.(e);
  }
</script>

<button
  class="icon-btn"
  class:active
  class:danger
  class:chrome
  class:locked={!!lock}
  aria-label={label}
  aria-pressed={toggle ? active : undefined}
  {disabled}
  onclick={click}
  use:tooltip={lock ? { label: `${label} · ${FLAGS[lock].label}`, hint: FLAGS[lock].hint } : { label, hint }}
>
  <Icon name={icon} {size} />
  {#if lock}<span class="lock"><Icon name="lock" size={9} /></span>{:else if badge}<span class="badge"></span>{/if}
</button>

<style>
  .icon-btn {
    position: relative;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    transition:
      background var(--fast),
      color var(--fast);
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }
  /* ka-4 Press feedback, at once. */
  .icon-btn:active:not(:disabled) {
    opacity: 0.6;
    transition: none;
  }
  .icon-btn.active {
    color: var(--accent-text);
    background: var(--accent-dim);
  }
  .icon-btn:disabled {
    opacity: 0.4;
  }
  .locked > :global(svg) {
    opacity: 0.4;
  }
  .lock {
    position: absolute;
    right: 3px;
    bottom: 3px;
    display: flex;
    color: var(--muted);
  }
  .chrome {
    width: 44px;
    height: var(--chrome-h);
    border-radius: 0;
  }
  .chrome.active {
    background: none;
  }
  .chrome.danger:hover {
    background: var(--danger-solid);
    color: var(--on-solid);
  }
  .badge {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
  }
</style>
