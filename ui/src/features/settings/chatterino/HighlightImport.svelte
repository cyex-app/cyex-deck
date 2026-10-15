<!-- V105-16 Highlights from Chatterino's settings.json: pick the file, see what comes in and
     what can't, then import (the result is a toast). Words, people and badges only; the rest of the file is never read. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Dialog from '../../../components/Dialog.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import type { HighlightPreview } from '../../../ipc/bindings/HighlightPreview';
  import type { Skipped } from '../../../ipc/bindings/Skipped';
  import { settingsIpc } from '../../../ipc/settings';
  import { formatNumber } from '../../../lib/format';
  import { toast } from '../../../lib/toast.svelte';
  import { failedImport } from './report';

  const MAX_LINES = 20;
  const CHOICES = [
    { value: 'mine', label: 'Keep mine' },
    { value: 'theirs', label: 'Use Chatterino’s' },
  ] as const;

  let preview = $state<HighlightPreview | null>(null);
  let busy = $state(false);
  let choice = $state<'mine' | 'theirs'>('mine');

  const fresh = $derived(preview ? preview.phrases + preview.users + preview.badges : 0);
  const dupes = $derived(preview ? preview.duplicates.length : 0);
  const canImport = $derived(fresh > 0 || (dupes > 0 && choice === 'theirs'));
  const headline = $derived.by(() => {
    if (!preview || !fresh) return 'Nothing new to import.';
    const parts = [
      preview.phrases && `${formatNumber(preview.phrases)} ${preview.phrases === 1 ? 'word' : 'words'}`,
      preview.users && `${formatNumber(preview.users)} ${preview.users === 1 ? 'person' : 'people'}`,
      preview.badges && `${formatNumber(preview.badges)} ${preview.badges === 1 ? 'badge' : 'badges'}`,
    ].filter(Boolean);
    return `${formatNumber(fresh)} new: ${parts.join(', ')}.`;
  });

  async function pick() {
    busy = true;
    try {
      const p = await settingsIpc.highlightsPreview();
      if (!p) return;
      preview = p;
      choice = 'mine';
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  async function run() {
    busy = true;
    try {
      const r = await settingsIpc.highlightsImport(choice === 'mine');
      const n = r.added + r.replaced;
      const skipped = r.skipped.length ? `, skipped ${formatNumber(r.skipped.length)}` : '';
      toast.success(n ? `Imported ${formatNumber(n)}${skipped}.` : `Nothing new${skipped}.`);
      close();
    } catch (e) {
      toast.error(failedImport(e));
      close();
    } finally {
      busy = false;
    }
  }

  function close() {
    preview = null;
  }

  function rows(list: Skipped[]) {
    return { shown: list.slice(0, MAX_LINES), more: Math.max(0, list.length - MAX_LINES) };
  }
</script>

{#snippet skippedList(list: Skipped[], title: string)}
  {@const r = rows(list)}
  <p class="head">{title}</p>
  <ul>
    {#each r.shown as s, i (i)}
      <li><code>{s.pattern}</code> <span class="why">{s.reason}</span></li>
    {/each}
  </ul>
  {#if r.more}<p class="why">and {formatNumber(r.more)} more</p>{/if}
{/snippet}

<Button small icon="upload" disabled={busy} onclick={pick}>Import from Chatterino</Button>

{#if preview}
  <Dialog title="Import highlights" width={460} onclose={close}>
    <p class="head">{headline}</p>
    {#if dupes}
      <p class="why">{formatNumber(dupes)} already there.</p>
      <Segmented options={CHOICES} value={choice} label="Duplicates" onchange={(v) => (choice = v)} />
    {/if}
    {#if preview.invalid.length}
      {@render skippedList(preview.invalid, `${formatNumber(preview.invalid.length)} can’t be imported:`)}
    {/if}
    {#snippet footer()}
      <Button variant="ghost" disabled={busy} onclick={close}>Cancel</Button>
      <Button variant="primary" disabled={busy || !canImport} onclick={run}>Import</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .head {
    margin: 0 0 8px;
    font-weight: 600;
  }
  .why {
    margin: 6px 0;
    font-size: 0.82rem;
    color: var(--dim);
  }
  ul {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 4px;
    font-size: 0.82rem;
  }
  li {
    overflow-wrap: anywhere;
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }
</style>
