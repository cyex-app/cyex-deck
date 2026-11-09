<!-- V110-U67 Adds (or edits) an entry of your own: what it does, its name, and the text it uses
     with placeholders like $(nick). The reason it can't be saved shows as you type. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Button from '../../../components/Button.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import {
    actionValue,
    type CustomKind,
    KIND_LABELS,
    kindsFor,
    makeAction,
    PLACEHOLDERS,
    placeholdersFor,
    validateCustom,
  } from '../../../components/menus/custom.ts';
  import type { CustomAction } from '../../../components/menus/types.ts';
  import { MAX_LABEL } from '../../../components/menus/ops.ts';

  interface Props {
    menuId: string;
    /** Set when editing an entry that exists. */
    initial?: { label: string; action: CustomAction };
    onsubmit: (value: { label: string; action: CustomAction }) => void;
    oncancel: () => void;
  }

  let { menuId, initial, onsubmit, oncancel }: Props = $props();

  // The form starts from what it is given; later changes of the props don't rewrite what you type.
  const start = untrack(() => ({ menuId, initial }));
  const kinds = kindsFor(start.menuId);
  let kind = $state<CustomKind>(start.initial?.action.kind ?? kinds[0]);
  let label = $state(start.initial?.label ?? '');
  let value = $state(start.initial ? actionValue(start.initial.action) : '');
  /** The error shows once something was typed in the box it is about, or on a try to save. */
  let tried = $state(false);
  let box = $state<HTMLInputElement>();
  let nameBox = $state<HTMLInputElement>();

  const chips = $derived(placeholdersFor(menuId));
  const error = $derived(validateCustom(menuId, label, makeAction(kind, value)));
  const shown = $derived(tried || (value.trim() !== '' && label.trim() !== '') ? error : null);

  const HINTS: Record<CustomKind, { label: string; placeholder: string }> = {
    link: { label: 'Link', placeholder: 'https://example.com/$(nick)' },
    chat: { label: 'Command', placeholder: '/timeout $(nick) 600' },
    prefill: { label: 'Text', placeholder: '@$(nick) ' },
    copy: { label: 'Text', placeholder: '$(nick)' },
  };

  function put(token: string) {
    const el = box;
    const at = el?.selectionStart ?? value.length;
    const end = el?.selectionEnd ?? at;
    value = value.slice(0, at) + token + value.slice(end);
    void Promise.resolve().then(() => {
      el?.focus();
      el?.setSelectionRange(at + token.length, at + token.length);
    });
  }

  function submit(e?: Event) {
    e?.preventDefault();
    tried = true;
    if (error) {
      (!label.trim() ? nameBox : box)?.focus();
      return;
    }
    onsubmit({ label: label.trim(), action: makeAction(kind, value) });
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      oncancel();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions (Escape closes the form) -->
<form class="form" role="group" onsubmit={submit} onkeydown={onkey} aria-label={initial ? 'Edit your entry' : 'Add your own entry'}>
  <div class="line">
    <span class="lab">Does</span>
    {#if kinds.length > 1}
      <Segmented
        label="What it does"
        options={kinds.map((k) => ({ value: k, label: KIND_LABELS[k] }))}
        value={kind}
        onchange={(k) => (kind = k)}
        neutral
        wrap
      />
    {/if}
  </div>
  <label class="line">
    <span class="lab">Name</span>
    <input class="field" bind:this={nameBox} bind:value={label} maxlength={MAX_LABEL} placeholder="Name in the menu" spellcheck="false" autocomplete="off" />
  </label>
  <label class="line">
    <span class="lab">{HINTS[kind].label}</span>
    <input
      class="field mono"
      bind:this={box}
      bind:value
      placeholder={HINTS[kind].placeholder}
      spellcheck="false"
      autocomplete="off"
      aria-invalid={!!shown}
      aria-describedby="custom-error"
    />
  </label>
  {#if chips.length}
    <div class="line">
      <span class="lab">Fill in</span>
      <span class="chips">
        {#each PLACEHOLDERS.filter((p) => chips.includes(p.name)) as p (p.name)}
          <button type="button" class="chip" title={p.label} onclick={() => put(p.token)}>{p.token}</button>
        {/each}
      </span>
    </div>
  {/if}
  {#if menuId === 'chat.name'}<p class="note">From the viewer list there is no message, so $(msg) and $(msg-id) stay empty.</p>{/if}
  <p id="custom-error" class="error" role="alert">{shown ?? ''}</p>
  <div class="acts">
    <Button variant="ghost" small onclick={oncancel}>Cancel</Button>
    <Button variant="primary" small type="submit" disabled={tried && !!error}>{initial ? 'Save' : 'Add'}</Button>
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 560px;
    margin: 10px 0 4px;
    padding: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .lab {
    flex: 0 0 52px;
    color: var(--muted);
    font-size: var(--text-hint);
  }
  .line > .field {
    flex: 1;
    min-width: 0;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 0.82rem;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    height: 24px;
    padding: 0 7px;
    background: var(--elevated-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: 0.74rem var(--font-mono);
  }
  .chip:hover {
    border-color: var(--dim);
  }
  .chip:active {
    background: var(--accent-dim);
  }
  .note {
    margin: 0;
    padding-left: 62px;
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .error {
    min-height: 1.1em;
    margin: 0;
    padding-left: 62px;
    color: var(--red);
    font-size: var(--text-hint);
  }
  .acts {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
</style>
