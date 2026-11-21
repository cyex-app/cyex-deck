<!-- V112-30 Slow mode and followers-only ask for their length here, over the chat they belong to.
     The chat is locked until OK or Cancel; a bad length keeps it open with the range note. -->
<script lang="ts">
  import { onMount, tick } from 'svelte';

  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import { LENGTH_ASK, type LengthKind, type ModeChange, submitLength } from './roomModes';

  let { kind, start, onok, oncancel }: { kind: LengthKind; start: string; onok: (c: ModeChange) => void; oncancel: () => void } = $props();

  const ask = $derived(LENGTH_ASK[kind]);
  // svelte-ignore state_referenced_locally
  let text = $state(start);
  let error = $state('');
  let input = $state<HTMLInputElement>();

  function ok() {
    const r = submitLength(kind, text);
    if ('error' in r) {
      error = r.error;
      input?.focus();
      return;
    }
    onok(r.change);
  }

  // The field takes the keyboard with its text selected, so typing replaces it.
  onMount(() => void tick().then(() => requestAnimationFrame(() => (input?.focus({ preventScroll: true }), input?.select()))));
</script>

<Dialog title={ask.title} width={280} within onsubmit={ok} onclose={oncancel}>
  <label class="row">
    <span class="label">{ask.label}</span>
    <input
      class="field"
      bind:this={input}
      bind:value={text}
      data-autofocus
      inputmode={kind === 'slow' ? 'numeric' : 'text'}
      placeholder={ask.placeholder}
      aria-invalid={!!error}
      spellcheck="false"
      autocomplete="off"
      oninput={() => (error = '')}
    />
  </label>
  {#if error}<div class="error" role="alert">{error}</div>{/if}
  {#snippet footer()}
    <Button variant="ghost" onclick={oncancel}>Cancel</Button>
    <Button variant="primary" onclick={ok}>OK</Button>
  {/snippet}
</Dialog>

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .label {
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .error {
    margin-top: 6px;
    color: var(--red);
    font-size: var(--text-sub);
  }
</style>
