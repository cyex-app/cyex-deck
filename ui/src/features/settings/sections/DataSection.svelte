<!-- Export, import, share codes (2.9, X6), Chatterino tabs (V1041-02) and daily backups (2.8). -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import ChatterinoImport from '../chatterino/ChatterinoImport.svelte';
  import type { Partition } from '../../../ipc/bindings/Partition';
  import { systemIpc } from '../../../ipc/system';
  import { settingsIpc } from '../../../ipc/settings';
  import { PARTITION_LABELS, PORTABLE_PARTITIONS, importedText, partitionList } from '../../../lib/partitions';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { errorText } from '../../../lib/errors';
  import { copy } from '../../../lib/clipboard';
  import { formatBytes, formatDate, formatTime } from '../../../lib/format';
  import SectionHead from '../SectionHead.svelte';
  import ConfirmButton from '../../../components/ConfirmButton.svelte';
  import Checkbox from '../../../components/Checkbox.svelte';
  import { backupText } from '../backupLine';
  import { putBackText } from '../putBack';
  import type { BackupLine, BrokenFile, Imported, ImportReview } from '../../../ipc/settings';
  import { brokenRestoredText, brokenRow } from '../brokenFiles';

  let picked = $state<Partition[]>(['general', 'appearance', 'chat', 'highlights', 'commands', 'hotkeys']);
  let code = $state('');
  /** D4 Why the last pasted code didn't import, right under the field. */
  let codeError = $state('');
  let backups = $state<string[]>([]);
  /** V1052-02 Bytes the picture cache takes; null until known. */
  let cacheSize = $state<number | null>(null);

  const allPicked = $derived(picked.length === PORTABLE_PARTITIONS.length);

  /** V110-U55 When the last copy was made, or why they fail. */
  let line = $state<BackupLine | null>(null);
  const status = $derived(backupText(line, (d) => `${formatDate(d)} ${formatTime(d)}`));
  let busy = $state(false);

  async function refreshStatus() {
    line = await settingsIpc.backupStatus().catch(() => line);
  }

  async function backUpNow() {
    busy = true;
    try {
      line = await settingsIpc.backupNow();
    } catch (e) {
      toast.error(errorText(e));
      await refreshStatus();
    } finally {
      busy = false;
    }
    void refreshBackups();
  }

  $effect(() => {
    void refreshStatus();
    void refreshBackups();
    void refreshBroken();
    void refreshCache();
  });

  async function refreshCache() {
    cacheSize = await systemIpc.pictureCacheSize().catch(() => null);
  }

  /** V110-A44 Files a start set aside because they couldn't be read. */
  let broken = $state<BrokenFile[]>([]);

  async function refreshBroken() {
    broken = await settingsIpc.brokenFiles().catch(() => []);
  }

  /** Same safety copy and Undo as a backup restore; the file itself stays where it is. */
  async function restoreBroken(file: BrokenFile) {
    if (busy) return;
    busy = true;
    try {
      const kept = await keepCurrent(true);
      if (kept === null) return;
      const result = await attempt(() => settingsIpc.restoreBroken(file.name));
      void refreshBroken();
      if (!result) return;
      toast.success(brokenRestoredText(file), { action: undoTo(kept) });
    } finally {
      busy = false;
    }
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

  /** V110-F84 What is there now, kept before anything overwrites it: a share code (Undo imports it again)
   *  and, for a restore (V1052-03), a snapshot on disk (no sign-ins). An import keeps the code only, so a
   *  cancelled file pick or a mistyped code leaves no snapshot behind. Null, with a toast, when it failed. */
  async function keepCurrent(snapshot = false): Promise<Kept | null> {
    try {
      const code = await settingsIpc.shareCode([...PORTABLE_PARTITIONS]);
      // A2-1 The flag travels with the code, so a second call can't overwrite it.
      const kept = { code, automationsOn: settings.v.commands.automationsOn };
      if (snapshot) await settingsIpc.snapshotNow();
      return kept;
    } catch {
      toast.error('Couldn’t keep your current settings first, so nothing was changed.');
      return null;
    }
  }

  /** V110-F97 The safety copy and whether automations were on when it was taken: an import switches them
   *  off, and Undo is an import too, so it switches them back on itself. */
  interface Kept {
    code: string;
    automationsOn: boolean;
  }

  /** The Undo that puts the kept settings back. */
  function undoTo(kept: Kept) {
    const on = kept.automationsOn;
    return {
      label: 'Undo',
      run: () =>
        void attempt(() => settingsIpc.importCode(kept.code)).then(async (done) => {
          if (!done) return;
          const p = done.partitions;
          // A2-3 A refused patch is a toast, not an unhandled rejection.
          if (on && p.includes('commands') && (await attempt(() => settings.patch('commands', { automationsOn: true }).then(() => true))) === undefined) return;
          // V110-N5 R2-3 The note only when they were on and the undo left them off.
          toast.success(putBackText(on, settings.v.commands.automationsOn));
        }),
    };
  }

  function imported(done: Imported | undefined, before?: Kept) {
    if (done?.partitions.length)
      toast.success(importedText(done.partitions, done.notes), { action: before ? undoTo(before) : undefined });
  }

  /** V110-A-idea What the picked file or pasted code brings, until Apply or Cancel. */
  let review = $state<ImportReview | null>(null);

  async function importFile() {
    const found = await attempt(() => settingsIpc.peekFile());
    if (found) review = found;
  }

  async function importCode(e: SubmitEvent) {
    e.preventDefault();
    codeError = '';
    try {
      review = await settingsIpc.peekCode(code);
    } catch (err) {
      // t5 Said once, under the field (it has role=alert); a toast too stacked up per try.
      codeError = errorText(err);
    }
  }

  async function applyImport() {
    if (busy) return;
    busy = true;
    try {
      const before = await keepCurrent();
      if (before === null) return;
      const done = await attempt(() => settingsIpc.applyImport());
      if (!done) return;
      review = null;
      code = '';
      imported(done, before);
    } finally {
      busy = false;
    }
  }

  // A2-2 A review still open when the section goes away is dropped on the Rust side too.
  $effect(() => () => {
    if (review) void settingsIpc.cancelImport().catch(() => {});
  });

  function cancelImport() {
    review = null;
    void settingsIpc.cancelImport().catch(() => {});
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
    // V110-F84 No safety copy, no restore.
    if (busy) return;
    busy = true;
    try {
      const kept = await keepCurrent(true);
      if (kept === null) return;
      const result = await attempt(() => settingsIpc.restoreBackup(date));
      void refreshBackups();
      if (!result) return;
      toast.success(restoredText(date), { action: undoTo(kept) });
    } finally {
      busy = false;
    }
  }

  /** t5 A snapshot by its time, not its folder name ("before-restore-2026-10-20-021030"). */
  function restoredText(id: string): string {
    const snap = /^before-restore-\d{4}-\d{2}-\d{2}-(\d{2})(\d{2})/.exec(id);
    if (!snap) return `Restored settings from ${id}.`;
    const when = dayName(id).split(' · ')[1] ?? '';
    return `Restored your settings from ${/^\d/.test(when) ? when : when.toLowerCase()}, ${snap[1]}:${snap[2]}.`;
  }

  /** t5 Two snapshots in the same minute get their seconds, so the rows differ. */
  const labels = $derived.by(() => {
    const names = backups.map((id) => dayName(id));
    return backups.map((id, i) => {
      const sec = /^before-restore-.*-\d{4}(\d{2})$/.exec(id)?.[1];
      return sec && names.indexOf(names[i]) !== names.lastIndexOf(names[i]) ? `${names[i]}:${sec}` : names[i];
    });
  });

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

<SectionHead title="Backup and sharing" level={2} docs="data">Move your settings to another PC, share them, or go back to an earlier copy.</SectionHead>

<SectionHead title="What to export or share">Pick the parts to put in the file.</SectionHead>
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

<SectionHead title="Import">Load a file or share code, from someone else or yourself.</SectionHead>
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
{#if review}
  <section class="review" aria-label="Review before importing" data-setting="data:review">
    {#if review.partitions.length}
      <p class="lead">This brings in {partitionList(review.partitions)}.</p>
      {#if review.lines.length}
        <ul>
          {#each review.lines as line (line)}<li>{line}</li>{/each}
        </ul>
      {/if}
      {#if review.partitions.includes('commands')}<p class="note">Automations come in switched off.</p>{/if}
    {:else}
      <p class="lead">There is nothing in it to import.</p>
    {/if}
    <div class="actions">
      <Button variant="primary" disabled={!review.partitions.length || busy} onclick={applyImport}>Apply</Button>
      <Button disabled={busy} onclick={cancelImport}>Cancel</Button>
    </div>
  </section>
{/if}

<ChatterinoImport />

<!-- rb-8 The subtitle, so the gap under the heading matches the other blocks. -->
<SectionHead title="Backups" setting="data:backups">
  A copy of your settings is kept for each of the last 3 days. Restoring keeps your current settings first, so you can undo it. Accounts, plugins and uploader stay as they are.
</SectionHead>
{#if status}
  <div class="status" data-setting="data:backup-status" class:failing={status.failing} role={status.failing ? 'alert' : undefined}>
    <p>{status.text}</p>
    {#if status.failing}<Button small disabled={busy} onclick={backUpNow}>Back up now</Button>{/if}
  </div>
{/if}
{#each backups as date, i (date)}
  <Row label={labels[i]}>
    <ConfirmButton label="Restore" confirm="Click to restore" disabled={busy} onconfirm={() => restore(date)} />
  </Row>
{:else}
  <p class="note">No backups yet. One is made each day you use cyex deck.</p>
{/each}
{#if broken.length}
  <SectionHead title="Restore from a broken file" setting="data:broken">
    Files set aside when they couldn’t be read. They’re removed after 30 days.
  </SectionHead>
  {#each broken as file (file.name)}
    {@const row = brokenRow(file)}
    <Row label={row.label} hint={row.hint}>
      <ConfirmButton label="Restore" confirm="Click to restore" disabled={busy} onconfirm={() => restoreBroken(file)} />
    </Row>
  {/each}
  <div class="actions">
    <Button variant="ghost" small icon="folder" onclick={() => attempt(() => settingsIpc.openFolder())}>Open folder</Button>
  </div>
{/if}
<SectionHead title="Cache">Saved pictures that load faster next time.</SectionHead>
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
  .picks {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 6px 12px;
  }
  /* rb-9 A note: the hint size. */
  .note {
    margin: 10px 0;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 8px 0;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  .status p {
    margin: 0;
  }
  .status.failing {
    color: var(--red);
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
  .review {
    margin-top: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  .review p {
    margin: 0;
  }
  .review .lead {
    font-size: var(--text-sub);
  }
  .review ul {
    margin: 6px 0 0;
    padding-left: 18px;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  .review .note {
    margin-top: 6px;
  }
  .review .actions {
    margin-top: 10px;
  }
  .problem {
    margin: 6px 0 0;
    color: var(--red);
    font-size: var(--text-sub);
  }
</style>
