<!-- V111-60 Settings › Import: everything from Chatterino in one place. The folder is found on
     its own (a portable one is picked); Import everything reads it and shows one review, or each
     kind comes on its own. Chatterino's files are only read. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import type { ChatterinoFolder } from '../../../ipc/bindings/ChatterinoFolder';
  import type { ChatterinoScan } from '../../../ipc/bindings/ChatterinoScan';
  import type { ImportKind } from '../../../ipc/bindings/ImportKind';
  import { settingsIpc } from '../../../ipc/settings';
  import { errorText } from '../../../lib/errors';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import ChatterinoReview from '../chatterino/ChatterinoReview.svelte';
  import { KINDS, LEFT_OUT } from '../chatterino/importAll';
  import SectionHead from '../SectionHead.svelte';

  const FILE: Record<ImportKind, keyof Omit<ChatterinoFolder, 'path'>> = {
    tabs: 'layout',
    highlights: 'settings',
    commands: 'commands',
    extras: 'settings',
  };
  const FILE_NAME: Record<ImportKind, string> = {
    tabs: 'window-layout.json',
    highlights: 'settings.json',
    commands: 'commands.json',
    extras: 'settings.json',
  };

  let folder = $state<ChatterinoFolder | null>(null);
  let looked = $state(false);
  let busy = $state<ImportKind | 'all' | 'folder' | null>(null);
  let review = $state<{ scan: ChatterinoScan; kinds: ImportKind[] } | null>(null);

  onMount(() => {
    settingsIpc
      .chatterinoFolder()
      .then((f) => (folder = f))
      .catch(() => {})
      .finally(() => (looked = true));
    // Seen the page: the one-time hint has done its job.
    if (!settings.v.general.chatterinoOffered) void settings.patch('general', { chatterinoOffered: true });
  });

  const has = (kind: ImportKind) => !!folder?.[FILE[kind]];
  const anything = $derived(KINDS.some((k) => has(k.kind)));

  async function pickFolder() {
    busy = 'folder';
    try {
      const f = await settingsIpc.chatterinoPickFolder();
      if (f) folder = f;
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = null;
    }
  }

  async function read(kinds: ImportKind[], what: ImportKind | 'all') {
    if (!folder) return;
    busy = what;
    try {
      const wanted = kinds.filter(has);
      review = { scan: await settingsIpc.chatterinoScan(folder.path, wanted), kinds: wanted };
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = null;
    }
  }
</script>

<SectionHead title="Import" level={2} page="import" />

<SectionHead title="From Chatterino" setting="import:folder">
  {#if folder}
    <span class="path">{folder.path}</span>
  {:else if looked}
    Chatterino isn’t in its usual place. For a portable one, pick its folder.
  {/if}
</SectionHead>
<div class="actions" data-setting="import:everything">
  <Button
    variant="primary"
    icon="download"
    disabled={!anything || !!busy}
    onclick={() => read(KINDS.map((k) => k.kind), 'all')}
  >
    {busy === 'all' ? 'Reading…' : 'Import everything'}
  </Button>
  <Button icon="folder" disabled={!!busy} onclick={pickFolder}>{folder ? 'Other folder' : 'Pick folder'}</Button>
</div>

<SectionHead title="One at a time" />
{#each KINDS as k (k.kind)}
  <Row label={k.label} hint={folder && !has(k.kind) ? `No ${FILE_NAME[k.kind]} in this folder.` : k.hint} setting={`import:${k.kind}`}>
    <Button small disabled={!has(k.kind) || !!busy} onclick={() => read([k.kind], k.kind)}>
      {busy === k.kind ? 'Reading…' : 'Import'}
    </Button>
  </Row>
{/each}

<SectionHead title="What can’t come" setting="import:left-out" />
{#each LEFT_OUT as l (l.label)}
  <p class="note"><span class="why">{l.label}:</span> {l.text}</p>
{/each}

{#if review}
  <ChatterinoReview scan={review.scan} kinds={review.kinds} onclose={() => (review = null)} />
{/if}

<style>
  .path {
    font-family: var(--font-mono);
    font-size: var(--text-hint);
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 10px;
  }
  .note {
    margin: 6px 0;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  .why {
    color: var(--muted);
    font-weight: 600;
  }
</style>
