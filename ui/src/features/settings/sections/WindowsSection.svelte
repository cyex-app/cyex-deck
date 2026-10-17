<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import EmptyState from '../../../components/EmptyState.svelte';
  import Row from '../../../components/Row.svelte';
  import { uniqueKeys } from '../../../lib/keys';
  import { windowsIpc } from '../../../ipc/windows';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { errorText } from '../../../lib/errors';
  import type { Layout } from '../../../ipc/bindings/Layout';
  import { drafts } from '../drafts.svelte';

  const layouts = $derived(settings.v.windows.layouts);
  const layoutKeys = $derived(uniqueKeys(layouts, (l) => l.name));

  async function run(action: () => Promise<void>, done?: string): Promise<boolean> {
    try {
      await action();
      if (done) toast.success(done);
      return true;
    } catch (e) {
      toast.error(errorText(e));
      return false;
    }
  }

  /** Deletes at once; Undo saves the layout back into the list where it was. */
  function remove(layout: Layout, at: number) {
    void run(() => windowsIpc.deleteLayout(layout.name)).then((ok) => {
      if (!ok) return;
      toast.success(`Deleted “${layout.name}”.`, {
        action: {
          label: 'Undo',
          run: () => {
            const now = settings.v.windows.layouts.filter((l) => l.name !== layout.name);
            void settings.patch('windows', { layouts: [...now.slice(0, at), layout, ...now.slice(at)] });
          },
        },
      });
    });
  }

  function save(e: SubmitEvent) {
    e.preventDefault();
    const n = drafts.layoutName.trim();
    if (!n) return;
    void run(() => windowsIpc.saveLayout(n), `Saved “${n}”.`).then((ok) => ok && (drafts.layoutName = ''));
  }
</script>

<h2>Windows</h2>
<p class="intro">A layout remembers where the main window and every pop-out sit, and whether they’re pinned.</p>

<form class="save" data-setting="windows.layouts" onsubmit={save}>
  <input class="field" placeholder="Layout name" maxlength="40" bind:value={drafts.layoutName} />
  <Button type="submit" variant="primary" small icon="plus" disabled={!drafts.layoutName.trim()}>Save current</Button>
</form>

{#each layouts as layout, i (layoutKeys[i])}
  {@const pops = Object.keys(layout.panels).length}
  <Row label={layout.name} hint={pops ? `Main window and ${pops} pop-out${pops === 1 ? '' : 's'}` : 'Main window only'}>
    <Button small onclick={() => run(() => windowsIpc.applyLayout(layout.name))}>Use</Button>
    <Button variant="ghost" small icon="trash" onclick={() => remove(layout, i)}>Delete</Button>
  </Row>
{:else}
  <EmptyState align="start" icon="layout" text="No saved layouts yet." />
{/each}

<style>
  .intro {
    margin: -6px 0 14px;
    color: var(--dim);
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
