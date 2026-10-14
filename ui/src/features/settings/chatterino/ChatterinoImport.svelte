<!-- V1041-02 Tabs from a Chatterino layout: pick the file, see what goes in and what stays
     out, then open them as chat tabs or chat + viewer tabs, joined as the default account. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Card from '../../../components/PanelCard.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import type { ImportReport } from '../../../ipc/bindings/ImportReport';
  import { settingsIpc } from '../../../ipc/settings';
  import { accounts } from '../../../lib/accounts.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { counts, leftOut } from './report';

  type OpenAs = 'chat' | 'both';
  const OPEN_AS = [
    { value: 'chat', label: 'Chat tabs' },
    { value: 'both', label: 'Chat + viewer tabs' },
  ] as const;

  let report = $state<ImportReport | null>(null);
  /** The import ran; `report` says what it did. */
  let done = $state(false);
  let busy = $state(false);
  let openAs = $state<OpenAs>('chat');

  const account = $derived(accounts.current?.displayName || accounts.current?.login);
  const lines = $derived(report ? leftOut(report) : []);
  const ready = $derived(!!report && report.tabs > 0);
  const asked = $derived(!!report && !done && report.tabs > 0);

  async function attempt(action: () => Promise<void>) {
    busy = true;
    try {
      await action();
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  function pick() {
    return attempt(async () => {
      const preview = await settingsIpc.chatterinoPreview();
      if (!preview) return;
      report = preview;
      done = false;
    });
  }

  function run() {
    return attempt(async () => {
      report = await settingsIpc.chatterinoImport(openAs === 'both');
      done = true;
    }).finally(() => {
      // A failed import needs a fresh pick: the lookup is used up.
      if (!done) report = null;
    });
  }

  function close() {
    report = null;
    done = false;
  }
</script>

<h3 class="label">Import from Chatterino</h3>
<p class="note" data-setting="data:chatterino">
  Your Chatterino tabs, in order and with their names. Pick <code>window-layout.json</code> or one of its backups.
</p>
{#if report}
  <div class="card">
    <Card title={done ? 'Imported' : 'Ready to import'}>
      <p class="headline">
        {#if report.tabs}
          {done ? `Added ${counts(report)}.` : counts(report)}
        {:else}
          {done ? 'Nothing was added.' : 'Nothing to import.'}
        {/if}
      </p>
      {#each lines as line (line.label)}
        <p class="note"><span class="why">{line.label}:</span> {line.text}</p>
      {/each}
      {#if asked}
        <div class="line">
          <span class="ask">Open them as</span>
          <Segmented options={OPEN_AS} value={openAs} label="Open them as" onchange={(v) => (openAs = v)} />
        </div>
        {#if account}<p class="note">Joined as {account}.</p>{/if}
        <div class="actions">
          <Button variant="primary" disabled={busy || !ready} onclick={run}>Import</Button>
          <Button variant="ghost" disabled={busy} onclick={close}>Cancel</Button>
        </div>
      {:else}
        <div class="actions">
          <Button onclick={close}>{done ? 'Done' : 'Close'}</Button>
        </div>
      {/if}
    </Card>
  </div>
{:else}
  <div class="actions">
    <Button icon="upload" disabled={busy} onclick={pick}>Pick layout file</Button>
  </div>
{/if}

<style>
  h3 {
    margin: 24px 0 10px;
  }
  .note {
    margin: 10px 0;
    font-size: 0.82rem;
    color: var(--dim);
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }
  .card {
    display: flex;
    max-width: 560px;
    margin-top: 10px;
  }
  .headline {
    margin: 0;
    font-weight: 600;
  }
  .why {
    color: var(--muted);
    font-weight: 600;
  }
  .ask {
    color: var(--muted);
    font-size: 0.82rem;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
</style>
