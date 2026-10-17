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
  import { formatBytes, formatDate } from '../../../lib/format';
  import SectionHead from '../SectionHead.svelte';
  import ConfirmButton from '../../../components/ConfirmButton.svelte';
  import Checkbox from '../../../components/Checkbox.svelte';

  let picked = $state<Partition[]>(['general', 'appearance', 'chat', 'highlights', 'commands', 'hotkeys']);
  let code = $state('');
  /** D4 Why the last pasted code didn't import, right under the field. */
  let codeError = $state('');
  let backups = $state<string[]>([]);
  /** V1052-02 Bytes the picture cache takes; null until known. */
  let cacheSize = $state<number | null>(null);

  const allPicked = $derived(picked.length === PORTABLE_PARTITIONS.length);

  $effect(() => {
    void refreshBackups();
    void refreshCache();
  });

  async function refreshCache() {
    cacheSize = await systemIpc.pictureCacheSize().catch(() => null);
  }

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
    codeError = '';
    try {
      const result = await settingsIpc.importCode(code);
      code = '';
      imported(result);
    } catch (err) {
      codeError = errorText(err);
      toast.error(codeError);
    }
  }

  async function clearCache() {
    const had = cacheSize;
    if (await attempt(() => systemIpc.clearPictureCache().then(() => true)))
      toast.success(had ? `Freed ${formatBytes(had)}.` : 'Picture cache cleared.');
    await refreshCache();
  }

  /** D1 Restoring first keeps what is there now as a share code (Undo imports it again) and,
   *  V1052-03, as a snapshot on disk (no sign-ins). */
  async function restore(date: string) {
    const before = await attempt(() => settingsIpc.shareCode([...PORTABLE_PARTITIONS]));
    await attempt(() => settingsIpc.snapshotNow());
    const result = await attempt(() => settingsIpc.restoreBackup(date));
    void refreshBackups();
    if (!result) return;
    toast.success(`Restored settings from ${date}.`, {
      action: before
        ? { label: 'Undo', run: () => void attempt(() => settingsIpc.importCode(before)).then((p) => p && imported(p)) }
        : undefined,
    });
  }

  /** D2 "2026/10/16 · Today"; V1052-03 snapshots read "Before restore · Today". */
  function dayName(id: string): string {
    const snap = /^before-restore-(\d{4})-(\d{2})-(\d{2})-(\d{2})?(\d{2})?/.exec(id);
    const date = snap ? `${snap[1]}/${snap[2]}/${snap[3]}` : id;
    const day = (back: number) => {
      const d = new Date();
      d.setDate(d.getDate() - back);
      return formatDate(d);
    };
    const when = date === day(0) ? 'Today' : date === day(1) ? 'Yesterday' : date;
    const time = snap?.[4] && snap[5] ? ` · ${snap[4]}:${snap[5]}` : '';
    return snap ? `Before restore · ${when}${time}` : when === date ? date : `${date} · ${when}`;
  }
</script>

<SectionHead title="Backup and sharing" level={2} docs="data" />

<h3 class="label">What to export or share</h3>
<div class="picks">
  <Checkbox label="Everything" checked={allPicked} indeterminate={picked.length > 0} onchange={() => (picked = allPicked ? [] : [...PORTABLE_PARTITIONS])} />
  {#each PORTABLE_PARTITIONS as p (p)}
    <Checkbox label={PARTITION_LABELS[p]} checked={picked.includes(p)} onchange={() => toggle(p)} />
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
  <input
    class="field"
    placeholder="Paste a share code"
    aria-label="Share code"
    aria-invalid={!!codeError}
    bind:value={code}
    oninput={() => (codeError = '')}
  />
  <Button type="submit" disabled={!code.trim()}>Import code</Button>
</form>
{#if codeError}<p class="problem" role="alert">{codeError}</p>{/if}

<ChatterinoImport />

<h3 class="label">Backups</h3>
<p class="note" data-setting="data:backups">
  A copy of your settings is kept for each of the last 3 days. Restoring keeps your current settings first, so you can undo it.
</p>
{#each backups as date (date)}
  <Row label={dayName(date)}>
    <ConfirmButton label="Restore" confirm="Click again (current is kept)" onconfirm={() => restore(date)} />
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
  <Button small onclick={clearCache}>{cacheSize ? `Clear (${formatBytes(cacheSize)})` : 'Clear'}</Button>
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
  .code .field[aria-invalid='true'] {
    border-color: var(--red);
  }
  .problem {
    margin: 6px 0 0;
    color: var(--red);
    font-size: 0.82rem;
  }
</style>
