<!-- V106-31/32 The picker's add/rename strip, pinned to its bottom edge. The title names the
     channel; the field label doesn't repeat it (c-8). -->
<script lang="ts">
  import AliasForm from './AliasForm.svelte';
  import { type EditForm, checkName, formChannel, submitForm } from './run';

  interface Props {
    account: string;
    login: string;
    emote: { id: string; name: string };
    mode: EditForm;
    /** After a successful edit. */
    ondone: () => void;
    oncancel: () => void;
  }

  let { account, login, emote, mode, ondone, oncancel }: Props = $props();

  let busy = $state(false);

  const channel = $derived(formChannel(mode));
  /** V107-N11b A long emote name shortens; the channel stays whole. */
  const head = $derived(mode.kind === 'add' ? `Add ${emote.name}` : `Rename ${emote.name}`);
  const tail = $derived(` ${mode.kind === 'add' ? 'to' : 'in'} ${channel}`);

  const check = (name: string) => checkName(account, mode, name);

  async function submit(name: string): Promise<string | null> {
    busy = true;
    try {
      return await submitForm(account, login, emote.id, mode, name, ondone);
    } finally {
      busy = false;
    }
  }
</script>

<div class="bar">
  <p class="title"><span class="head">{head}</span><span class="tail">{tail}</span></p>
  {#if mode.kind === 'add'}
    <AliasForm
      label="Name"
      value={mode.ownName ?? emote.name}
      submit={busy ? 'Adding…' : 'Add'}
      {busy}
      {check}
      onsubmit={submit}
      {oncancel}
    />
  {:else}
    <AliasForm
      label="New name"
      value={mode.here.alias}
      submit={busy ? 'Saving…' : 'Save'}
      {busy}
      {check}
      onsubmit={submit}
      {oncancel}
    />
  {/if}
</div>

<style>
  .bar {
    position: absolute;
    inset: auto 0 0 0;
    padding: 8px 10px;
    background: var(--elevated);
    border-top: 1px solid var(--border-strong);
    border-radius: 0 0 var(--radius) var(--radius);
  }
  .title {
    display: flex;
    margin: 0 0 6px;
    font-size: 0.8rem;
    white-space: pre;
  }
  .head {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tail {
    flex: none;
    max-width: 50%;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
