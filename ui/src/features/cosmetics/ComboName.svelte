<!-- V1051-04 The name field of a combo, in place: the Save card, or a card being renamed. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import { cleanName, shortError } from './comboName';

  interface Props {
    value?: string;
    /** Returns Rust’s error text, or null when it saved. */
    onsubmit: (name: string) => Promise<string | null>;
    /** `key` is true when Esc did it, so focus can go back where the user came from. */
    oncancel: (key?: boolean) => void;
    /** The message to show for the field: ours while typing, Rust’s after Enter. */
    onerror: (message: string | null) => void;
  }

  let { value = '', onsubmit, oncancel, onerror }: Props = $props();

  // svelte-ignore state_referenced_locally
  let text = $state(value);
  // svelte-ignore state_referenced_locally
  const start = value;
  let touched = false;
  let busy = false;
  let input = $state<HTMLInputElement>();

  onMount(() => {
    input?.focus();
    input?.select();
  });

  function check(): ReturnType<typeof cleanName> {
    const r = cleanName(text);
    onerror(touched && !r.ok ? r.message : null);
    return r;
  }

  async function onkeydown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === 'Escape') {
      e.preventDefault();
      oncancel(true);
    } else if (e.key === 'Enter' && !busy) {
      e.preventDefault();
      touched = true;
      const r = check();
      if (!r.ok) return;
      busy = true;
      const err = await onsubmit(r.name);
      busy = false;
      if (err) onerror(shortError(err));
    }
  }
</script>

<input
  bind:this={input}
  class="field"
  placeholder="Name"
  aria-label="Combo name"
  maxlength={80}
  spellcheck="false"
  autocomplete="off"
  bind:value={text}
  oninput={() => {
    touched = true;
    check();
  }}
  {onkeydown}
  onblur={() => {
    if (!busy && (text.trim() === '' || text === start)) oncancel();
  }}
/>

<style>
  input {
    box-sizing: border-box;
    width: 100%;
    height: 22px;
    padding: 0 6px;
    font-size: 12px;
  }
</style>
