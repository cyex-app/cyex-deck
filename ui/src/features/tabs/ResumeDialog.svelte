<!-- V108-N21 After a crash: which tabs to reopen. The rest wait in "Reopen closed tab". -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import { focusTabChat } from './focusChat';
  import { picked, reopenLabel } from './resume';
  import { resumeDialog, resumeWith } from './resume.svelte';
  import { label, tabs } from './tabs.svelte';

  // svelte-ignore state_referenced_locally
  const list = tabs.list.map((t) => ({ id: t.id, name: label(t) || 'Tab' }));
  const all = list.map((t) => t.id);
  let off = $state(new Set<string>());
  const keep = $derived(picked(all, off));


  function toggle(id: string, on: boolean) {
    const next = new Set(off);
    if (on) next.delete(id);
    else next.add(id);
    off = next;
  }

  async function go(ids: string[]) {
    resumeDialog.open = false;
    await resumeWith(all, ids);
    // The dialog opened over nothing in particular: land in the chat of the tab on screen, not on the page.
    if (list.length > 1 && tabs.active) focusTabChat(tabs.active, false);
  }

  // Nothing to pick from with one tab or none: it stays, no question.
  $effect(() => {
    if (list.length < 2) void go(all);
  });
</script>

<!-- Escape keeps everything, as it was. -->
{#if list.length > 1}
<Dialog title="Reopen your tabs?" width={380} onclose={() => go(all)} onsubmit={() => keep.length && go(keep)}>
  <p class="lead">cyex deck closed unexpectedly. Pick the tabs to bring back.</p>
  <div class="list">
    {#each list as t (t.id)}
      <Checkbox checked={!off.has(t.id)} label={t.name} onchange={(c) => toggle(t.id, c)} />
    {/each}
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={() => go([])}>Start empty</Button>
    <Button variant="primary" disabled={!keep.length} onclick={() => go(keep)}>{reopenLabel(keep.length)}</Button>
  {/snippet}
</Dialog>
{/if}

<style>
  .lead {
    margin: 0 0 12px;
    color: var(--muted);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 280px;
    overflow-y: auto;
  }
</style>
