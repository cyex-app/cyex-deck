<!-- V106-31/32 The picker's add/rename strip, pinned to its bottom edge. -->
<script lang="ts">
  import type { AddTarget } from '../../../ipc/bindings/AddTarget';
  import type { HereEdit } from '../../../ipc/bindings/HereEdit';
  import { emotesIpc } from '../../../ipc/emotes';
  import AliasForm from './AliasForm.svelte';
  import { runEdit } from './run';
  import { inlineText } from './words';

  type Mode = { kind: 'add'; target: AddTarget; ownName: string } | { kind: 'rename'; here: HereEdit };

  interface Props {
    account: string;
    login: string;
    emote: { id: string; name: string };
    mode: Mode;
    /** After a successful edit. */
    ondone: () => void;
    oncancel: () => void;
  }

  let { account, login, emote, mode, ondone, oncancel }: Props = $props();

  let busy = $state(false);

  const channel = $derived(mode.kind === 'add' ? mode.target.channel : mode.here.channel);
  const setId = $derived(mode.kind === 'add' ? mode.target.setId : mode.here.setId);
  const title = $derived(
    mode.kind === 'add' ? `Add ${emote.name} to ${channel}` : `Rename ${emote.name} in ${channel}`,
  );

  async function check(name: string): Promise<string | null> {
    const r = await emotesIpc.editName(
      account,
      setId,
      emote.id,
      name,
      mode.kind === 'rename' ? mode.here.alias : null,
    );
    if (r === 'taken') return `Taken in ${channel}.`;
    return r === 'same' ? '' : null;
  }

  async function submit(name: string): Promise<string | null> {
    busy = true;
    try {
      const op =
        mode.kind === 'add'
          ? ({ type: 'add', setId, emoteId: emote.id, alias: name } as const)
          : ({ type: 'rename', setId, emoteId: emote.id, from: mode.here.alias, to: name } as const);
      const fail = await runEdit(account, login, op, channel, {
        quietFail: (e) => inlineText(e, channel) !== null,
      });
      if (!fail) {
        ondone();
        return null;
      }
      return inlineText(fail, channel) ?? 'Didn’t work. Try again.';
    } finally {
      busy = false;
    }
  }
</script>

<div class="bar">
  <p class="title">{title}</p>
  {#if mode.kind === 'add'}
    <AliasForm
      label="Name in {channel}"
      value={mode.ownName}
      submit={busy ? 'Adding…' : 'Add'}
      {busy}
      {check}
      onsubmit={submit}
      {oncancel}
    />
  {:else}
    <AliasForm
      label="New name in {channel}"
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
