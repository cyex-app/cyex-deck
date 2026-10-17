<!-- V106-31/32 The card's 7TV edit row: add to a channel, rename or remove in this chat's set. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import { type MenuItem, openMenu } from '../../../components/menu.svelte';
  import type { AddTarget } from '../../../ipc/bindings/AddTarget';
  import type { EditOptions } from '../../../ipc/bindings/EditOptions';
  import type { HereEdit } from '../../../ipc/bindings/HereEdit';
  import type { Layer } from '../../../ipc/bindings/Layer';
  import { emotesIpc } from '../../../ipc/emotes';
  import type { CardTarget } from '../emotes';
  import AliasForm from './AliasForm.svelte';
  import { editOptionsFor } from './options';
  import { runEdit } from './run';
  import { blockedText, inlineText } from './words';

  interface Props {
    account: string;
    login: string;
    emote: Layer;
    target: CardTarget;
    /** An edit went through; the card may reload. */
    onchange: () => void;
  }

  let { account, login, emote, target, onchange }: Props = $props();

  type Form = { kind: 'add'; target: AddTarget } | { kind: 'rename'; here: HereEdit };

  let options = $state<EditOptions | null>(null);
  let asked = $state(false);
  let busy = $state(false);
  // The card opens straight into the form chosen in the right-click menu.
  // svelte-ignore state_referenced_locally
  let form = $state<Form | null>(target.edit ?? null);

  async function load(): Promise<void> {
    const id = emote.id;
    const o = await editOptionsFor(account, login, id, emote.name);
    if (id !== emote.id) return;
    options = o;
    asked = true;
  }

  $effect(() => {
    void emote.id;
    asked = false;
    options = null;
    void load();
  });

  const add = $derived(options?.state === 'ready' ? options.add : null);
  const here = $derived(options?.state === 'ready' ? options.here : null);
  const single = $derived(add && add.targets.length === 1 ? add.targets[0] : null);
  const note = $derived(
    add?.blocked
      ? blockedText(add.blocked)
      : single?.blocked
        ? blockedText(single.blocked, single.channel)
        : null,
  );
  const hasAny = $derived(!!(add && add.targets.length > 0) || !!here);

  function pickTarget(e: MouseEvent): void {
    if (!add || e.currentTarget === null) return;
    const items: MenuItem[] = add.targets.map((t) => ({
      label: t.channel,
      hint: t.viewed ? 'This chat' : t.own ? 'Yours' : undefined,
      disabled: !!t.blocked,
      note: t.blocked ? blockedText(t.blocked, t.channel) : undefined,
      run: () => (form = { kind: 'add', target: t }),
    }));
    openMenu(e.currentTarget as Element, items, 'start');
  }

  async function checkAdd(t: AddTarget, name: string): Promise<string | null> {
    const r = await emotesIpc.editName(account, t.setId, emote.id, name);
    return r === 'taken' ? `Taken in ${t.channel}.` : null;
  }

  async function checkRename(h: HereEdit, name: string): Promise<string | null> {
    const r = await emotesIpc.editName(account, h.setId, emote.id, name, h.alias);
    if (r === 'same') return '';
    return r === 'taken' ? `Taken in ${h.channel}.` : null;
  }

  async function submitAdd(t: AddTarget, name: string): Promise<string | null> {
    return finish(await runEdit(account, login, { type: 'add', setId: t.setId, emoteId: emote.id, alias: name }, t.channel, quiet(t.channel)), t.channel);
  }

  async function submitRename(h: HereEdit, name: string): Promise<string | null> {
    const op = { type: 'rename', setId: h.setId, emoteId: emote.id, from: h.alias, to: name } as const;
    return finish(await runEdit(account, login, op, h.channel, quiet(h.channel)), h.channel);
  }

  function quiet(channel: string) {
    return { quietFail: (e: Parameters<typeof inlineText>[0]) => inlineText(e, channel) !== null };
  }

  /** The inline line when the form stays, else close it and refresh. */
  function finish(fail: Parameters<typeof inlineText>[0] | null, channel: string): string | null {
    if (fail) return inlineText(fail, channel);
    form = null;
    void load();
    onchange();
    return null;
  }

  async function remove(h: HereEdit): Promise<void> {
    busy = true;
    const fail = await runEdit(account, login, { type: 'remove', setId: h.setId, emoteId: emote.id, alias: h.alias }, h.channel);
    busy = false;
    if (!fail) {
      void load();
      onchange();
    }
  }
</script>

{#if asked && options?.state === 'signInAgain'}
  <div class="edit"><p class="note">Sign in to 7TV again to edit emotes.</p></div>
{:else if asked && hasAny}
  <div class="edit">
    {#if form?.kind === 'add'}
      {@const t = form.target}
      <AliasForm
        label={`Name in ${t.channel}`}
        value={options?.ownName ?? emote.name}
        submit={busy ? 'Adding…' : 'Add'}
        {busy}
        check={(n) => checkAdd(t, n)}
        onsubmit={async (n) => {
          busy = true;
          const r = await submitAdd(t, n);
          busy = false;
          return r;
        }}
        oncancel={() => (form = null)}
      />
    {:else if form?.kind === 'rename'}
      {@const h = form.here}
      <AliasForm
        label={`New name in ${h.channel}`}
        value={h.alias}
        submit={busy ? 'Saving…' : 'Save'}
        {busy}
        check={(n) => checkRename(h, n)}
        onsubmit={async (n) => {
          busy = true;
          const r = await submitRename(h, n);
          busy = false;
          return r;
        }}
        oncancel={() => (form = null)}
      />
    {:else}
      <div class="row">
        {#if add && add.targets.length > 0}
          {#if single}
            <Button small disabled={!!(add.blocked ?? single.blocked)} onclick={() => (form = { kind: 'add', target: single })}>
              Add to {single.channel}
            </Button>
          {:else}
            <Button small disabled={!!add.blocked} onclick={pickTarget}>Add to 7TV channel</Button>
          {/if}
        {/if}
        {#if here}
          <Button small icon="pencil" onclick={() => (form = { kind: 'rename', here })}>Rename</Button>
          <Button small icon="trash" disabled={busy} onclick={() => void remove(here)}>Remove</Button>
        {/if}
      </div>
      {#if note}<p class="note">{note}</p>{/if}
    {/if}
  </div>
{/if}

<style>
  .edit {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .note {
    margin: 6px 0 0;
    color: var(--dim);
    font-size: 0.75rem;
  }
</style>
