<!-- V111-06 Commands from Chatterino's commands.json: pick the file, see each command as it
     comes in (notes on what changed, names in use, what can't come and why), then import. The
     toast has Undo. Chatterino's message-menu commands can come along; V111-56 on by default,
     and each command says what it does here. V111-60 The review is CommandsReview, shared with
     Settings › Import. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Dialog from '../../../components/Dialog.svelte';
  import type { CommandPreview } from '../../../ipc/bindings/CommandPreview';
  import { settingsIpc } from '../../../ipc/settings';
  import { errorText } from '../../../lib/errors';
  import { formatNumber } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { outcome, resultText } from './commandImport';
  import CommandsReview from './CommandsReview.svelte';
  import { failedImport } from './report';

  let preview = $state<CommandPreview | null>(null);
  let busy = $state(false);
  let take = $state<Set<number>>(new Set());
  let menu = $state(true);

  const result = $derived(preview ? outcome(preview, take) : null);
  const count = $derived(result ? result.added + result.replaced : 0);
  // V111-56 Menu entries alone (every name kept) are an import too.
  const canImport = $derived(!!result && (count > 0 || (menu && result.menu > 0)));

  async function pick() {
    busy = true;
    try {
      const p = await settingsIpc.commandsPreview();
      if (!p) return;
      preview = p;
      take = new Set();
      menu = true;
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  async function run() {
    if (!preview) return;
    busy = true;
    // Undo puts both lists back as they are now.
    const c = settings.v.commands;
    const before = { lines: [...c.lines], messageMenu: [...c.messageMenu] };
    try {
      const r = await settingsIpc.commandsImport([...take], menu && !!result?.menu);
      const changed = r.added + r.replaced + r.menu > 0;
      const undo = () => void settings.patch('commands', before);
      toast.success(resultText(r), changed ? { action: { label: 'Undo', run: undo } } : undefined);
      preview = null;
    } catch (e) {
      toast.error(failedImport(e));
      close();
    } finally {
      busy = false;
    }
  }

  function close() {
    preview = null;
    void settingsIpc.commandsCancel().catch(() => {});
  }
</script>

<Button small icon="upload" disabled={busy} onclick={pick}>Import from Chatterino</Button>

{#if preview && result}
  <Dialog title="Import commands" width={560} locked={busy} onsubmit={() => canImport && !busy && void run()} onclose={close}>
    <CommandsReview {preview} bind:take bind:menu />
    {#snippet footer()}
      <Button variant="ghost" disabled={busy} onclick={close}>Cancel</Button>
      <Button variant="primary" disabled={busy || !canImport} onclick={run}>{count > 0 ? `Import ${formatNumber(count)}` : 'Import'}</Button>
    {/snippet}
  </Dialog>
{/if}
