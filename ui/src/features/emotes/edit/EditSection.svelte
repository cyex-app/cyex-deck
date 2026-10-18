<!-- V106-31/32 The card's 7TV edit row: add to a channel, rename or remove in this chat's set. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import { openMenu } from '../../../components/menu.svelte';
  import type { EditOptions } from '../../../ipc/bindings/EditOptions';
  import type { HereEdit } from '../../../ipc/bindings/HereEdit';
  import type { Layer } from '../../../ipc/bindings/Layer';
  import type { CardTarget } from '../emotes';
  import AliasForm from './AliasForm.svelte';
  import { addNote, targetItems } from './menuItems';
  import { editOptionsFor } from './options';
  import { type EditForm, checkName, removeHere, submitForm } from './run';

  interface Props {
    account: string;
    login: string;
    emote: Layer;
    target: CardTarget;
    /** An edit went through; the card may reload. */
    onchange: () => void;
  }

  let { account, login, emote, target, onchange }: Props = $props();

  let options = $state<EditOptions | null>(null);
  let asked = $state(false);
  let busy = $state(false);
  // The card opens straight into the form chosen in the right-click menu.
  // svelte-ignore state_referenced_locally
  let form = $state<EditForm | null>(target.edit ?? null);

  // Current alias; follows a rename made in this card.
  // svelte-ignore state_referenced_locally
  let name = $state(emote.name);

  async function load(): Promise<void> {
    const id = emote.id;
    const o = await editOptionsFor(account, login, id, name);
    if (id !== emote.id) return;
    options = o;
    asked = true;
  }

  $effect(() => {
    void emote.id;
    name = emote.name;
    asked = false;
    options = null;
    void load();
  });

  const add = $derived(options?.state === 'ready' ? options.add : null);
  const here = $derived(options?.state === 'ready' ? options.here : null);
  const single = $derived(add && add.targets.length === 1 ? add.targets[0] : null);
  const note = $derived(add ? (addNote(add) ?? null) : null);
  const hasAny = $derived(!!(add && add.targets.length > 0) || !!here);

  function pickTarget(e: MouseEvent): void {
    if (!add || e.currentTarget === null) return;
    openMenu(e.currentTarget as Element, targetItems(add, (t) => (form = { kind: 'add', target: t })), 'start');
  }

  /** The form's add or rename; on success closes it and refreshes. */
  async function submit(f: EditForm, alias: string): Promise<string | null> {
    busy = true;
    try {
      return await submitForm(account, login, emote.id, f, alias, () => {
        if (f.kind === 'rename') name = alias;
        form = null;
        void load();
        onchange();
      });
    } finally {
      busy = false;
    }
  }

  async function remove(h: HereEdit): Promise<void> {
    busy = true;
    const fail = await removeHere(account, login, emote.id, h);
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
        check={(n) => checkName(account, form!, n)}
        onsubmit={(n) => submit(form!, n)}
        oncancel={() => (form = null)}
      />
    {:else if form?.kind === 'rename'}
      {@const h = form.here}
      <AliasForm
        label={`New name in ${h.channel}`}
        value={h.alias}
        submit={busy ? 'Saving…' : 'Save'}
        {busy}
        check={(n) => checkName(account, form!, n)}
        onsubmit={(n) => submit(form!, n)}
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
