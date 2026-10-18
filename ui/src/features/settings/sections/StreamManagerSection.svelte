<!-- Stream manager settings (10, X4): text files for OBS with your live title, category,
     viewers, uptime, followers and subs, and where they go. The marker log lives in the
     same folder. -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import Button from '../../../components/Button.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { ObsFile } from '../../../ipc/bindings/ObsFile';
  import { smIpc } from '../../../ipc/streammanager';
  import { copy } from '../../../lib/clipboard';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { errorText } from '../../../lib/errors';

  const obs = $derived(settings.v.streammanager.obs);
  let folder = $state<string | null>(null);
  let files = $state.raw<ObsFile[]>([]);

  function reload() {
    smIpc
      .obsFiles()
      .then((f) => (files = f))
      .catch(() => (files = []));
  }

  $effect(() => {
    void obs.folder;
    void obs.enabled;
    smIpc
      .obsFolder()
      .then((f) => (folder = f))
      .catch(() => (folder = null));
    reload();
  });

  // V106-15
  async function pick() {
    try {
      const p = await smIpc.obsPickFolder();
      if (p) await settings.patch('streammanager', { obs: { folder: p } });
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  // V106-16
  function openFile(name: string) {
    smIpc
      .obsOpenFile(name)
      .catch((e) => toast.error(errorText(e)))
      .finally(reload);
  }
</script>

<SectionHead title="Stream manager" level={2}>Tools for when you stream, like OBS files.</SectionHead>

<SectionHead title="Files for OBS">Text files OBS can show on your stream.</SectionHead>
<Row setting="streammanager.obs.enabled" label="Write status files" hint="Updated about every 30 seconds while the stream manager is open.">
  <Toggle label="Write status files" checked={obs.enabled} onchange={(on) => void settings.patch('streammanager', { obs: { enabled: on } })} />
</Row>
<Row setting="streammanager.obs.folder" label="Folder" hint="Where the files go. Default: a stream-status folder next to your settings." stacked>
  <div class="folder">
    <Button small icon="folder" onclick={pick}>Select folder</Button>
    <Button small icon="popout" onclick={() => void smIpc.obsOpen().catch((e) => toast.error(errorText(e)))}>Open folder</Button>
    {#if obs.folder}<IconButton icon="undo" label="Use default folder" hint="Back to the stream-status folder next to the settings." size={14} onclick={() => void settings.patch('streammanager', { obs: { folder: '' } })} />{/if}
  </div>
  {#if folder}<p class="path">{folder}</p>{/if}
</Row>

<p class="note">In OBS, add a “Text (GDI+)” source, tick “Read from file” and pick one of these files.</p>
<table class="files" data-setting="streammanager:files">
  <thead><tr><th>File</th><th>What’s in it</th><th><span class="sr-only">Actions</span></th></tr></thead>
  <tbody>
    {#each files as f (f.name)}
      <tr>
        <td><code>{f.name}</code></td>
        <td>{f.what}{#if !f.exists}<span class="later"> · not written yet</span>{/if}</td>
        <td class="acts">
          <IconButton icon="popout" label="Open {f.name}" size={14} onclick={() => openFile(f.name)} />
          <IconButton icon="copy" label="Copy the path of {f.name}" size={14} onclick={() => void copy(f.path, 'Path')} />
        </td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .folder {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .path {
    margin: 4px 0 0;
    font-size: 0.76rem;
    color: var(--dim);
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
    user-select: text;
  }
  .note {
    margin: 14px 0 6px;
    font-size: 0.82rem;
    color: var(--muted);
  }
  .files {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
    color: var(--muted);
  }
  th {
    text-align: left;
    padding: 0 8px 4px 0;
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--dim);
  }
  td {
    padding: 5px 8px 5px 0;
    border-top: 1px solid var(--border);
    vertical-align: middle;
  }
  td:first-child {
    white-space: nowrap;
  }
  .acts {
    width: 1%;
    white-space: nowrap;
    text-align: right;
    padding-right: 0;
  }
  .later {
    color: var(--dim);
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text);
  }
</style>
