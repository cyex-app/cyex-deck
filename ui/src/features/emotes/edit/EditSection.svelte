<!-- V106-31/32 The card's 7TV edit row: add to a channel, rename or remove in this chat's set. -->
<script lang="ts">
  import { tick, untrack } from 'svelte';

  import Button from '../../../components/Button.svelte';
  import { openMenu } from '../../../components/menu.svelte';
  import type { EditOptions } from '../../../ipc/bindings/EditOptions';
  import type { HereEdit } from '../../../ipc/bindings/HereEdit';
  import type { Layer } from '../../../ipc/bindings/Layer';
  import type { CardTarget } from '../emotes';
  import AliasForm from './AliasForm.svelte';
  import { addNote, targetItems } from './menuItems';
  import { editOptionsFor } from './options';
  import { refocus } from './refocus';
  import { afterEdit, currentName } from './renamed';
  import { type EditForm, checkName, removeHere, submitForm } from './run';

  interface Props {
    account: string;
    login: string;
    /** The emote as the card shows it; a rename here comes back through `onchange`. */
    emote: Layer;
    target: CardTarget;
    /** An edit went through: the emote after it (renamed, or the same). */
    onchange: (emote: Layer) => void;
    /** c-3 Where focus goes when the form closes and its opener is gone (the card's heading). */
    home?: () => HTMLElement | undefined;
  }

  let { account, login, emote, target, onchange, home }: Props = $props();

  let options = $state<EditOptions | null>(null);
  let asked = $state(false);
  let busy = $state(false);
  // The card opens straight into the form chosen in the right-click menu.
  // svelte-ignore state_referenced_locally
  let form = $state<EditForm | null>(target.edit ?? null);
  let root = $state<HTMLElement>();
  /** c-3 The button that opened the form: `add`, `pick` or `rename`. */
  let opener: string | null = null;
  let seq = 0;
  let shownId = '';

  async function load(): Promise<void> {
    const mine = ++seq;
    const o = await editOptionsFor(account, login, emote.id, emote.name);
    if (mine !== seq) return;
    options = o;
    asked = true;
    // rc-2 A line from before a rename: the card follows the emote's current name in the set.
    const now = currentName(emote, o);
    if (now !== emote) onchange(now);
  }

  // c-1 Asks again when the emote or its name changes (a rename from this card). Only a new
  // emote clears the row; the load itself isn't tracked, so its own state can't re-run this.
  $effect(() => {
    const id = emote.id;
    void emote.name;
    if (id !== shownId) {
      shownId = id;
      asked = false;
      options = null;
    }
    untrack(() => void load());
  });

  const add = $derived(options?.state === 'ready' ? options.add : null);
  const here = $derived(options?.state === 'ready' ? options.here : null);
  const single = $derived(add && add.targets.length === 1 ? add.targets[0] : null);
  const note = $derived(add ? (addNote(add) ?? null) : null);
  const hasAny = $derived(!!(add && add.targets.length > 0) || !!here);

  function openForm(f: EditForm, by: string): void {
    opener = by;
    form = f;
  }

  /** c-3 Closes the form; focus goes back to its opener, else the card's heading. */
  async function close(): Promise<void> {
    form = null;
    await tick();
    refocus(root, opener, home?.());
  }

  function pickTarget(e: MouseEvent): void {
    if (!add || e.currentTarget === null) return;
    openMenu(e.currentTarget as Element, targetItems(add, (t) => openForm({ kind: 'add', target: t }, 'pick')), 'start');
  }

  /** The form's add or rename; on success closes it and refreshes. */
  async function submit(f: EditForm, alias: string): Promise<string | null> {
    busy = true;
    try {
      return await submitForm(account, login, emote.id, f, alias, () => {
        const next = afterEdit(emote, f, alias);
        void close();
        // A rename re-asks through the effect once the card shows the new name.
        if (next === emote) void load();
        onchange(next);
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
      onchange(emote);
    }
  }
</script>

{#if asked && options?.state === 'signInAgain'}
  <div class="edit"><p class="note">Sign in to 7TV again to edit emotes.</p></div>
{:else if asked && hasAny}
  <div class="edit" bind:this={root}>
    {#if form?.kind === 'add'}
      {@const t = form.target}
      <AliasForm
        label={`Name in ${t.channel}`}
        value={options?.ownName ?? emote.name}
        submit={busy ? 'Adding…' : 'Add'}
        {busy}
        check={(n) => checkName(account, form!, n)}
        onsubmit={(n) => submit(form!, n)}
        oncancel={() => void close()}
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
        oncancel={() => void close()}
      />
    {:else}
      <div class="row">
        {#if add && add.targets.length > 0}
          {#if single}
            <span data-opener="add">
              <Button small disabled={!!(add.blocked ?? single.blocked)} onclick={() => openForm({ kind: 'add', target: single }, 'add')}>
                Add to {single.channel}
              </Button>
            </span>
          {:else}
            <span data-opener="pick"><Button small disabled={!!add.blocked} onclick={pickTarget}>Add to 7TV channel</Button></span>
          {/if}
        {/if}
        {#if here}
          <span data-opener="rename"><Button small icon="pencil" onclick={() => openForm({ kind: 'rename', here }, 'rename')}>Rename</Button></span>
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
  [data-opener] {
    display: contents;
  }
  .note {
    margin: 6px 0 0;
    color: var(--dim);
    font-size: var(--text-hint);
  }
</style>
