<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import { windowsIpc } from '../../../ipc/windows';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { errorText } from '../../../lib/errors';

  const layouts = $derived(settings.v.windows.layouts);

  let name = $state('');

  async function run(action: () => Promise<void>, done?: string) {
    try {
      await action();
      if (done) toast.success(done);
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  function save(e: SubmitEvent) {
    e.preventDefault();
    const n = name.trim();
    if (!n) return;
    void run(() => windowsIpc.saveLayout(n), `Saved "${n}".`).then(() => (name = ''));
  }
</script>

<h2>Windows</h2>
<p class="intro">A layout remembers where the main window and every pop-out sit, and whether they’re pinned.</p>

<form class="save" data-setting="windows.layouts" onsubmit={save}>
  <input class="field" placeholder="Layout name" maxlength="40" bind:value={name} />
  <Button type="submit" variant="primary" small icon="plus" disabled={!name.trim()}>Save current</Button>
</form>

{#each layouts as layout (layout.name)}
  <Row label={layout.name} hint={`${Object.keys(layout.panels).length} pop-outs`}>
    <Button small onclick={() => run(() => windowsIpc.applyLayout(layout.name))}>Use</Button>
    <Button variant="ghost" small icon="trash" onclick={() => run(() => windowsIpc.deleteLayout(layout.name))}>Delete</Button>
  </Row>
{:else}
  <p class="empty">No saved layouts yet.</p>
{/each}

<style>
  .intro,
  .empty {
    margin: -6px 0 14px;
    color: var(--dim);
  }
  .empty {
    margin-top: 14px;
  }
  .save {
    display: flex;
    gap: 8px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }
  .save .field {
    width: 220px;
  }
</style>
