<!-- T2 A color as a swatch and its hex code side by side. The swatch opens the system picker;
     the hex field takes `#rrggbb` or `rrggbb` (Enter or leaving the field keeps it). `value` is
     what is painted: when a theme nudges the pick, the swatch and the field show the nudge (T1). -->
<script lang="ts">
  import { parseHex } from './themes';

  interface Props {
    value: string;
    label: string;
    /** Called with `#rrggbb`. */
    onchange: (hex: string) => void;
    /** The typed text wasn't a color; the field goes back to `value`. */
    oninvalid?: () => void;
    width?: string;
  }

  let { value, label, onchange, oninvalid, width = '78px' }: Props = $props();

  /** What the user just picked or typed, held while the saved value catches up. */
  let draft = $state<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const shown = $derived(draft ?? value);

  function hold(hex: string) {
    draft = hex;
    clearTimeout(timer);
    timer = setTimeout(() => (draft = null), 500);
  }

  function commit(input: HTMLInputElement) {
    const hex = parseHex(input.value);
    if (!hex) {
      input.value = shown;
      oninvalid?.();
      return;
    }
    input.value = hex;
    if (hex !== value) {
      hold(hex);
      onchange(hex);
    }
  }

  $effect(() => () => clearTimeout(timer));
</script>

<span class="color">
  <input
    type="color"
    class="swatch"
    aria-label={label}
    value={shown}
    oninput={(e) => {
      hold(e.currentTarget.value);
      onchange(e.currentTarget.value);
    }}
  />
  <input
    class="field hex"
    aria-label="{label} hex code"
    spellcheck="false"
    autocomplete="off"
    maxlength="7"
    style:width
    value={shown}
    onchange={(e) => commit(e.currentTarget)}
    onkeydown={(e) => e.key === 'Enter' && commit(e.currentTarget)}
  />
</span>

<style>
  .color {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .swatch {
    flex: none;
    width: 30px;
    height: 28px;
    padding: 2px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    cursor: pointer;
  }
  .swatch::-webkit-color-swatch-wrapper {
    padding: 0;
  }
  .swatch::-webkit-color-swatch {
    border: 0;
    border-radius: var(--radius-sm);
  }
  .hex {
    height: 28px;
    padding: 0 6px;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    text-transform: lowercase;
  }
</style>
