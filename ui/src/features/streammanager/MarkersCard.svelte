<!-- 1.35, X4 Stream markers: add one at the current spot (live only) and see the marker
     log, which the app keeps as a text file next to the OBS status files. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { smIpc } from '../../ipc/streammanager';
  import { toast } from '../../lib/toast.svelte';
  import Card from '../../components/PanelCard.svelte';
  import { sm } from './streammanager.svelte';
  import { errorText } from '../../lib/errors';

  const MAX = 140;

  let text = $state('');
  let lines = $state<string[]>([]);
  let busy = $state(false);

  const live = $derived(sm.stats?.live ?? false);

  function load() {
    smIpc
      .markers()
      .then((l) => (lines = l))
      .catch(() => {});
  }
  load();

  function add() {
    busy = true;
    smIpc
      .marker(text.trim())
      .then(() => {
        text = '';
        toast.info('Marker added.');
        load();
      })
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (busy = false));
  }
</script>

<Card title="Markers">
  {#snippet tools()}
    <IconButton icon="folder" label="Open folder" hint="The marker log and OBS files live here." size={14} onclick={() => void smIpc.obsOpen().catch((e) => toast.error(errorText(e)))} />
  {/snippet}
  <form
    class="line"
    onsubmit={(e) => {
      e.preventDefault();
      add();
    }}
  >
    <input class="field grow" placeholder="What happened (optional)" maxlength={MAX} bind:value={text} disabled={!live} />
    <Button small type="submit" disabled={!live || busy}>Add marker</Button>
  </form>
  {#if !live}<p class="note">Markers go on a live stream.</p>{/if}
  {#if lines.length}
    <ul>
      {#each lines as l, i (i)}<li>{l}</li>{/each}
    </ul>
  {/if}
</Card>

<style>
  .grow {
    flex: 1;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 0.78rem;
    font-family: var(--font-mono);
    color: var(--muted);
  }
  li {
    padding: 2px 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
</style>
