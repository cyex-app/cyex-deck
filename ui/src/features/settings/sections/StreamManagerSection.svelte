<!-- Stream manager settings (10, X4): text files for OBS with your live title, category,
     viewers, uptime, followers and subs, and where they go. The marker log lives in the
     same folder. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import { smIpc } from '../../../ipc/streammanager';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';

  const FILES: [string, string][] = [
    ['live.txt', 'live or offline'],
    ['title.txt', 'stream title'],
    ['category.txt', 'category'],
    ['viewers.txt', 'viewers right now'],
    ['uptime.txt', 'time live, like 1:23'],
    ['followers.txt', 'follower count'],
    ['subscribers.txt', 'subscriber count'],
  ];

  const obs = $derived(settings.v.streammanager.obs);
  let folder = $state<string | null>(null);
  let typed = $state(settings.v.streammanager.obs.folder);

  $effect(() => {
    void obs.folder;
    smIpc
      .obsFolder()
      .then((f) => (folder = f))
      .catch(() => (folder = null));
  });

  function saveFolder() {
    void settings.patch('streammanager', { obs: { folder: typed.trim() } });
  }
</script>

<h2>Stream manager</h2>

<h3 class="label">Files for OBS</h3>
<Row setting="streammanager.obs.enabled" label="Write status files" hint="Updated about every 30 seconds while the stream manager is open.">
  <Toggle label="Write status files" checked={obs.enabled} onchange={(on) => void settings.patch('streammanager', { obs: { enabled: on } })} />
</Row>
<Row setting="streammanager.obs.folder" label="Folder" hint="Leave empty for a stream-status folder next to the settings." stacked>
  <form
    class="folder"
    onsubmit={(e) => {
      e.preventDefault();
      saveFolder();
    }}
  >
    <input class="field" placeholder="C:\Streaming\status" bind:value={typed} onblur={saveFolder} />
    <Button small onclick={() => void smIpc.obsOpen().catch((e) => toast.error(String(e)))}>Open folder</Button>
  </form>
  {#if folder}<p class="path">{folder}</p>{/if}
</Row>

<p class="note">In OBS, add a "Text (GDI+)" source, tick "Read from file" and pick one of these:</p>
<ul class="files">
  {#each FILES as [name, what] (name)}
    <li><code>{name}</code> {what}</li>
  {/each}
</ul>
<p class="note">Markers you add in the stream manager are listed in <code>markers.log</code> in the same folder, with the time and how long you’d been live.</p>

<style>
  .folder {
    display: flex;
    gap: 6px;
  }
  .folder .field {
    flex: 1;
    min-width: 0;
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
    margin: 0;
    padding-left: 18px;
    font-size: 0.82rem;
    color: var(--muted);
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text);
  }
</style>
