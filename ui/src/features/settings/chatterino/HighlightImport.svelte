<!-- V105-16 Highlights from Chatterino's settings.json: pick the file, see what comes in and
     what can't, then import (the result is a toast). Words, people and badges only; the rest of the file is never read.
     V111-60 The review is HighlightsReview, shared with Settings › Import. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Dialog from '../../../components/Dialog.svelte';
  import type { HighlightPreview } from '../../../ipc/bindings/HighlightPreview';
  import { settingsIpc } from '../../../ipc/settings';
  import { toast } from '../../../lib/toast.svelte';
  import { failedImport } from './report';
  import { errorText } from '../../../lib/errors';
  import HighlightsReview from './HighlightsReview.svelte';
  import { highlightCanImport, highlightResult } from './importAll';

  let preview = $state<HighlightPreview | null>(null);
  let busy = $state(false);
  let choice = $state<'mine' | 'theirs'>('mine');

  const canImport = $derived(!!preview && highlightCanImport(preview, choice));

  async function pick() {
    busy = true;
    try {
      const p = await settingsIpc.highlightsPreview();
      if (!p) return;
      preview = p;
      choice = 'mine';
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  async function run() {
    busy = true;
    try {
      const r = await settingsIpc.highlightsImport(choice === 'mine');
      toast.success(highlightResult(r));
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
</script>

<Button small icon="upload" disabled={busy} onclick={pick}>Import from Chatterino</Button>

{#if preview}
  <Dialog title="Import highlights" width={460} locked={busy} onsubmit={() => canImport && !busy && void run()} onclose={close}>
    <HighlightsReview {preview} bind:choice />
    {#snippet footer()}
      <Button variant="ghost" disabled={busy} onclick={close}>Cancel</Button>
      <Button variant="primary" disabled={busy || !canImport} onclick={run}>Import</Button>
    {/snippet}
  </Dialog>
{/if}
