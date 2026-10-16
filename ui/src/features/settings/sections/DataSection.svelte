<!-- Export, import, share codes (2.9, X6), Chatterino tabs (V1041-02) and daily backups (2.8). -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import ChatterinoImport from '../chatterino/ChatterinoImport.svelte';
  import type { Partition } from '../../../ipc/bindings/Partition';
  import { systemIpc } from '../../../ipc/system';
  import { settingsIpc } from '../../../ipc/settings';
  import { PARTITION_LABELS, PORTABLE_PARTITIONS, partitionList } from '../../../lib/partitions';
  import { toast } from '../../../lib/toast.svelte';
  import { errorText } from '../../../lib/errors';
  import { copy } from '../../../lib/clipboard';

  let picked = $state<Partition[]>(['general', 'appearance', 'chat', 'highlights', 'mentions', 'commands', 'hotkeys']);
  let code = $state('');
  let backups = $state<string[]>([]);

  const allPicked = $derived(picked.length === PORTABLE_PARTITIONS.length);

  $effect(() => {
    void refreshBackups();
  });

  async function refreshBackups() {
    try {
      backups = await settingsIpc.backups();
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  function toggle(p: Partition) {
    picked = picked.includes(p) ? picked.filter((x) => x !== p) : PORTABLE_PARTITIONS.filter((x) => x === p || picked.includes(x));
  }

  async function attempt<T>(action: () => Promise<T>): Promise<T | undefined> {
    try {
      return await action();
    } catch (e) {
      toast.error(errorText(e));
      return undefined;
    }
  }

  async function exportFile() {
    if (await attempt(() => settingsIpc.exportFile(picked))) toast.success('Settings exported.');
  }

  async function copyCode() {
    const shareCode = await attempt(() => settingsIpc.shareCode(picked));
    if (!shareCode) return;
    await copy(shareCode, 'Share code');
  }

  function imported(partitions: Partition[] | undefined) {
    if (partitions?.length) toast.success(`Imported ${partitionList(partitions)}.`);
  }

  async function importFile() {
    imported(await attempt(() => settingsIpc.importFile()));
  }

  async function importCode(e: SubmitEvent) {
    e.preventDefault();
    const result = await attempt(() => settingsIpc.importCode(code));
    if (result) code = '';
    imported(result);
  }

  async function clearCache() {
    if (await attempt(() => systemIpc.clearPictureCache().then(() => true))) toast.success('Picture cache cleared.');
  }

  async function restore(date: string) {
    const result = await attempt(() => settingsIpc.restoreBackup(date));
    if (result) toast.success(`Restored settings from ${date}.`);
  }
</script>

<h2>Backup and sharing</h2>

<h3 class="label">What to export or share</h3>
<div class="picks">
  <label class="pick">
    <input
      type="checkbox"
      checked={allPicked}
      onchange={() => (picked = allPicked ? [] : [...PORTABLE_PARTITIONS])}
    />
    Everything
  </label>
  {#each PORTABLE_PARTITIONS as p (p)}
    <label class="pick">
      <input type="checkbox" checked={picked.includes(p)} onchange={() => toggle(p)} />
      {PARTITION_LABELS[p]}
    </label>
  {/each}
</div>
<p class="note">Accounts and sign-ins are never included.</p>
<div class="actions" data-setting="data:export">
  <Button icon="download" disabled={!picked.length} onclick={exportFile}>Export file</Button>
  <Button icon="copy" disabled={!picked.length} onclick={copyCode}>Copy share code</Button>
</div>

<h3 class="label">Import</h3>
<div class="actions" data-setting="data:import">
  <Button icon="upload" onclick={importFile}>Import file</Button>
</div>
<form class="code" onsubmit={importCode}>
  <input class="field" placeholder="Paste a share code" bind:value={code} />
  <Button type="submit" disabled={!code.trim()}>Import code</Button>
</form>

<ChatterinoImport />

<h3 class="label">Backups</h3>
<p class="note" data-setting="data:backups">A copy of your settings is kept for each of the last 3 days.</p>
{#each backups as date (date)}
  <Row label={date}>
    <Button small onclick={() => restore(date)}>Restore</Button>
  </Row>
{:else}
  <p class="note">No backups yet. The first one is made tomorrow.</p>
{/each}
<h3 class="label">Cache</h3>
<Row
  label="Picture cache"
  hint="Emotes, badges and paints you’ve seen, saved on this PC. They load again when needed."
  setting="data:cache"
>
  <Button small onclick={clearCache}>Clear</Button>
</Row>
<div class="actions" data-setting="data:folder">
  <Button variant="ghost" small icon="folder" onclick={() => attempt(() => settingsIpc.openFolder())}>Open settings folder</Button>
</div>

<style>
  h3 {
    margin: 24px 0 10px;
  }
  h3:first-of-type {
    margin-top: 4px;
  }
  .picks {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 6px 12px;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
  .pick input {
    accent-color: var(--accent);
  }
  .note {
    margin: 10px 0;
    font-size: 0.82rem;
    color: var(--dim);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .code {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .code .field {
    flex: 1;
    font-family: var(--font-mono);
  }
</style>
