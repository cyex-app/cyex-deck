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
  const title = $derived(
    mode.kind === 'add' ? `Add ${emote.name} to ${channel}` : `Rename ${emote.name} in ${channel}`,
  );

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
  <p class="title">{title}</p>
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
    margin: 0 0 6px;
    overflow: hidden;
    font-size: 0.8rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
