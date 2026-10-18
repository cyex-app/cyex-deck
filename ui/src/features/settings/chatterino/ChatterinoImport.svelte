<!-- V1041-02 Tabs from a Chatterino layout: pick the file, see what goes in and what stays
     out in a dialog, then open them as chat tabs or chat + viewer tabs (the result is a toast),
     joined as the default account. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Checkbox from '../../../components/Checkbox.svelte';
  import Dialog from '../../../components/Dialog.svelte';
  import SectionHead from '../SectionHead.svelte';
  import type { ImportReport } from '../../../ipc/bindings/ImportReport';
  import { settingsIpc } from '../../../ipc/settings';
  import { accounts } from '../../../lib/accounts.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { counts, failedImport, leftOut, tabsText } from './report';

  let report = $state<ImportReport | null>(null);
  let busy = $state(false);
  let viewerTabs = $state(false);

  const account = $derived(accounts.current?.displayName || accounts.current?.login);
  const lines = $derived(report ? leftOut(report) : []);
  const ready = $derived(!!report && report.tabs > 0);

  async function pick() {
    busy = true;
    try {
      const preview = await settingsIpc.chatterinoPreview();
      if (!preview) return;
      report = preview;
    } catch (e) {
      toast.error(failedImport(e));
    } finally {
      busy = false;
    }
  }

  async function run() {
    busy = true;
    try {
      const done = await settingsIpc.chatterinoImport(viewerTabs);
      toast.success(done.tabs ? `Imported ${tabsText(done.tabs)}.` : 'Nothing new.');
    } catch (e) {
      // A failed import needs a fresh pick: the lookup is used up.
      toast.error(failedImport(e));
    } finally {
      busy = false;
      close();
    }
  }

  function close() {
    report = null;
  }
</script>

<!-- b-14 The line is the heading's subtitle, so the gap under it matches the other blocks. -->
<SectionHead title="Import from Chatterino" setting="data:chatterino"
  >Your Chatterino tabs, in order and with their names. Pick <code>window-layout.json</code> or one of its backups.</SectionHead
>
<div class="actions">
  <Button icon="upload" disabled={busy} onclick={pick}>Pick layout file</Button>
</div>

{#if report}
  <Dialog title="Import tabs" width={460} locked={busy} onsubmit={() => ready && !busy && void run()} onclose={close}>
    <p class="headline">{report.tabs ? counts(report) : 'Nothing to import.'}</p>
    {#each lines as line (line.label)}
      <p class="note"><span class="why">{line.label}:</span> {line.text}</p>
    {/each}
    {#if ready}
      <div class="line">
        <Checkbox checked={viewerTabs} label="Also open viewer tabs" onchange={(v) => (viewerTabs = v)} />
      </div>
      {#if account}<p class="note">Joined as {account}.</p>{/if}
    {/if}
    {#snippet footer()}
      <Button variant="ghost" disabled={busy} onclick={close}>Cancel</Button>
      <Button variant="primary" disabled={busy || !ready} onclick={run}>Import</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .note {
    margin: 10px 0;
    font-size: var(--text-sub);
    color: var(--dim);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--text-hint);
  }
  .headline {
    margin: 0;
    font-weight: 600;
  }
  .why {
    color: var(--muted);
    font-weight: 600;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 10px;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
</style>
