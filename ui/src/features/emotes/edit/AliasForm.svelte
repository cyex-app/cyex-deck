<!-- V106-31/32 The name field shared by the card, the picker bar and rename: checks 7TV's rule, then Rust's. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Button from '../../../components/Button.svelte';
  import { layer } from '../../../lib/escape';
  import { checkAlias, judgeAlias } from './alias';

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

  /** Checks `text`; returns its verdict (see judgeAlias), shown unless a newer check started. */
  async function validate(): Promise<string | null> {
    const mine = ++seq;
    const t = text;
    // The own rule answers at once, so the button greys out without a wait.
    const r = checkAlias(t);
    if (!r.ok) {
      ok = false;
      message = r.message;
      return r.message;
    }
    const v = await judgeAlias(t, check);
    if (mine === seq) {
      ok = v === null;
      message = v ? v : null;
    }
    return v;
  }

  function oninput(): void {
    touched = true;
    void validate();
  }

  let sending = false;

  async function send(): Promise<void> {
    if (busy || sending) return;
    sending = true;
    try {
      // c-12 Judge the text as it is now: `ok` may still be the last keystroke's answer.
      const t = text;
      if ((await validate()) !== null) {
        // s3-5 A bad name says why, even untouched.
        touched = true;
        return;
      }
      const err = await onsubmit(t);
      if (err) {
        ok = false;
        touched = true;
        message = err;
      }
    } finally {
      sending = false;
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
    data-draft={text ? '' : undefined}
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
    font-size: var(--text-hint);
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
