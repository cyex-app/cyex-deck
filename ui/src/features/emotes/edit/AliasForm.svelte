<!-- V106-31/32 The name field shared by the card, the picker bar and rename: checks 7TV's rule, then Rust's. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Button from '../../../components/Button.svelte';
  import { layer } from '../../../lib/escape';
  import { checkAlias } from './alias';

  interface Props {
    label: string;
    value: string;
    submit: string;
    busy?: boolean;
    /** Rust's name check: an error line, '' for "not ok, say nothing", null when fine. */
    check?: (name: string) => Promise<string | null>;
    /** Returns an error line to keep the form open, or null when done. */
    onsubmit: (name: string) => Promise<string | null>;
    oncancel: () => void;
  }

  let { label, value, submit, busy = false, check, onsubmit, oncancel }: Props = $props();

  // svelte-ignore state_referenced_locally
  let text = $state(value);
  let touched = $state(false);
  let ok = $state(true);
  let message = $state<string | null>(null);
  let input = $state<HTMLInputElement>();
  let seq = 0;

  onMount(() => {
    input?.focus();
    input?.select();
    const off = layer('popover', oncancel);
    // Rust's check for the untouched default too (e.g. same name as before).
    void validate();
    return off;
  });

  async function validate(): Promise<void> {
    const mine = ++seq;
    const r = checkAlias(text);
    if (!r.ok) {
      ok = false;
      message = r.message;
      return;
    }
    const rust = check ? await check(r.name) : null;
    if (mine !== seq) return;
    ok = rust === null;
    message = rust ? rust : null;
  }

  function oninput(): void {
    touched = true;
    void validate();
  }

  async function send(): Promise<void> {
    if (!ok || busy) return;
    const err = await onsubmit(text);
    if (err) {
      ok = false;
      touched = true;
      message = err;
    }
  }

  function onkeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter') {
      e.preventDefault();
      void send();
    }
  }
</script>

<div class="form">
  <label for="alias-field">{label}</label>
  <input
    id="alias-field"
    bind:this={input}
    class="field"
    spellcheck="false"
    autocomplete="off"
    maxlength={200}
    bind:value={text}
    {oninput}
    {onkeydown}
  />
  <p class="err" aria-live="polite">{touched ? (message ?? '') : ''}</p>
  <div class="row">
    <Button variant="ghost" small onclick={oncancel}>Cancel</Button>
    <Button variant="primary" small disabled={busy || !ok} onclick={() => void send()}>{submit}</Button>
  </div>
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  label {
    font-size: 0.78rem;
    color: var(--dim);
  }
  input {
    box-sizing: border-box;
    width: 100%;
    height: 26px;
  }
  .err {
    margin: 0;
    min-height: 1.1em;
    font-size: 0.75rem;
    color: var(--red);
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
</style>
