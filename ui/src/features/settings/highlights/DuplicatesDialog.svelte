<!-- V106-33 (Q13) Asked once after the Mentions merge found repeated rules. Nothing is removed unless
     "Remove duplicates" is picked; Enter, Esc, × and a click outside all keep everything. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Button from '../../../components/Button.svelte';
  import Dialog from '../../../components/Dialog.svelte';
  import type { PhraseRule } from '../../../ipc/bindings/PhraseRule';
  import { mentionsIpc } from '../../../ipc/mentions';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { groups, removeCopies } from './duplicates';

  type Found = { pattern: string; where: string };

  let found = $state.raw<Found[] | null>(null);
  let count = 0;
  let phrases: PhraseRule[] = [];
  let globalList: PhraseRule[] = [];
  let phraseChecks: Awaited<ReturnType<typeof mentionsIpc.check>> = [];
  let globalChecks: typeof phraseChecks = [];

  const save = (patch: Parameters<typeof settings.patch<'highlights'>>[1]) => void settings.patch('highlights', patch);

  function describe(list: PhraseRule[], checks: typeof phraseChecks, where: string): Found[] {
    return groups(list, checks).map((g) => ({ pattern: list[g.first].pattern, where: `${where} · ${g.copies.length + 1} copies` }));
  }

  $effect(() => {
    let live = true;
    const h = untrack(() => settings.v.highlights);
    phrases = h.phrases;
    globalList = h.globalList;
    Promise.all([mentionsIpc.check(phrases), mentionsIpc.check(globalList)])
      .then(([p, g]) => {
        if (!live) return;
        phraseChecks = p;
        globalChecks = g;
        const all = [...describe(phrases, p, 'Words'), ...describe(globalList, g, 'Global mentions')];
        count = groups(phrases, p).reduce((n, x) => n + x.copies.length, 0) + groups(globalList, g).reduce((n, x) => n + x.copies.length, 0);
        // Copies already removed by hand: nothing to ask.
        if (all.length === 0) save({ duplicatesAsked: true });
        else found = all;
      })
      .catch(() => {});
    return () => (live = false);
  });

  function keepAll() {
    save({ duplicatesAsked: true });
  }

  function remove() {
    const before = phrases;
    const beforeGlobal = globalList;
    const n = count;
    save({ phrases: removeCopies(phrases, phraseChecks), globalList: removeCopies(globalList, globalChecks), duplicatesAsked: true });
    toast.success(`Removed ${n} ${n === 1 ? 'duplicate' : 'duplicates'}.`, {
      action: { label: 'Undo', run: () => save({ phrases: before, globalList: beforeGlobal }) },
    });
  }
</script>

{#if found}
  <Dialog title="Some highlights are there twice" width={440} onclose={keepAll} onsubmit={keepAll}>
    <p class="lead">Mentions are part of Highlights now, and these were in both lists. Nothing was removed.</p>
    <ul class="groups">
      {#each found as f, i (i)}
        <li>
          <code>{f.pattern || '(empty)'}</code>
          <span class="where">{f.where}</span>
        </li>
      {/each}
    </ul>
    <p class="hint">Remove duplicates keeps the first of each and deletes the later copies.</p>
    {#snippet footer()}
      <Button variant="ghost" onclick={remove}>Remove duplicates</Button>
      <Button variant="primary" onclick={keepAll}>Keep all</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .lead {
    margin: 0 0 10px;
  }
  .hint,
  .where {
    color: var(--dim);
  }
  .hint {
    margin: 10px 0 0;
    font-size: 0.75rem;
  }
  .groups {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 7px 10px;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  code {
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }
  .where {
    font-size: 0.75rem;
  }
</style>
