<!-- Export, import, share codes (2.9, X6) and daily backups (2.8). -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import type { Partition } from '../../../ipc/bindings/Partition';
  import { settingsIpc } from '../../../ipc/settings';
  import { ALL_PARTITIONS, PARTITION_LABELS, partitionList } from '../../../lib/partitions';
  import { toast } from '../../../lib/toast.svelte';

  let picked = $state<Partition[]>(['general', 'appearance', 'chat', 'highlights', 'commands', 'hotkeys']);
  let code = $state('');
  let backups = $state<string[]>([]);

  const allPicked = $derived(picked.length === ALL_PARTITIONS.length);

  $effect(() => {
    void refreshBackups();
  });

  async function refreshBackups() {
    try {
      backups = await settingsIpc.backups();
    } catch (e) {
      toast.error(String(e));
    }
  }

  function toggle(p: Partition) {
    picked = picked.includes(p) ? picked.filter((x) => x !== p) : ALL_PARTITIONS.filter((x) => x === p || picked.includes(x));
  }

  async function attempt<T>(action: () => Promise<T>): Promise<T | undefined> {
    try {
      return await action();
    } catch (e) {
      toast.error(String(e));
      return undefined;
    }
  }

  async function exportFile() {
    if (await attempt(() => settingsIpc.exportFile(picked))) toast.info('Settings exported.');
  }

  async function copyCode() {
    const shareCode = await attempt(() => settingsIpc.shareCode(picked));
    if (!shareCode) return;
    await navigator.clipboard.writeText(shareCode);
    toast.info('Share code copied.');
  }

  function imported(partitions: Partition[] | undefined) {
    if (partitions?.length) toast.info(`Imported ${partitionList(partitions)}.`);
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

  async function restore(date: string) {
    const result = await attempt(() => settingsIpc.restoreBackup(date));
    if (result) toast.info(`Restored settings from ${date}.`);
  }
</script>

<h2>Backup and sharing</h2>

<h3 class="label">What to export or share</h3>
<div class="picks">
  <label class="pick">
    <input
      type="checkbox"
      checked={allPicked}
      onchange={() => (picked = allPicked ? [] : [...ALL_PARTITIONS])}
    />
    Everything
  </label>
  {#each ALL_PARTITIONS as p (p)}
    <label class="pick">
      <input type="checkbox" checked={picked.includes(p)} onchange={() => toggle(p)} />
      {PARTITION_LABELS[p]}
    </label>
  {/each}
</div>
<p class="note">Sign-ins and tokens are never included.</p>
<div class="actions">
  <Button icon="download" disabled={!picked.length} onclick={exportFile}>Export file</Button>
  <Button icon="copy" disabled={!picked.length} onclick={copyCode}>Copy share code</Button>
</div>

<h3 class="label">Import</h3>
<div class="actions">
  <Button icon="upload" onclick={importFile}>Import file</Button>
</div>
<form class="code" onsubmit={importCode}>
  <input class="field" placeholder="Paste a share code" bind:value={code} />
  <Button type="submit" disabled={!code.trim()}>Import code</Button>
</form>

<h3 class="label">Backups</h3>
<p class="note">A copy of your settings is kept for each of the last 3 days.</p>
{#each backups as date (date)}
  <Row label={date}>
    <Button small onclick={() => restore(date)}>Restore</Button>
  </Row>
{:else}
  <p class="note">No backups yet. The first one is made tomorrow.</p>
{/each}
<div class="actions">
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
