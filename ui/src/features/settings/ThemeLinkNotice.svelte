<!-- V109-T01 A theme added from a link: which version you have, Update when a newer one is
     out (asks first when you changed it), Ignore this version, Go back to one you had, and a
     quiet line when the author took it down. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import ConfirmChoices from '../../components/ConfirmChoices.svelte';
  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
  import { themeIpc } from '../../ipc/theme';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { copyLabel, updateAvailable, updateLabel } from './themeLink';

  interface Props {
    theme: CustomTheme;
    /** Saves edits still waiting for the next frame, before the theme is replaced. */
    flush: () => void;
  }

  let { theme, flush }: Props = $props();

  const link = $derived(theme.link);
  const next = $derived(updateAvailable(link));
  let busy = $state(false);
  let asking = $state(false);
  let open = $state(false);

  async function run(task: () => Promise<void>) {
    if (busy) return;
    busy = true;
    flush();
    try {
      await task();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  const update = (replace: boolean) =>
    run(async () => {
      asking = false;
      const r = await themeIpc.linkUpdate(theme.id, replace);
      if (r === 'askReplace') asking = true;
      else toast.success('Theme updated.', { action: { label: 'Go back', run: () => void restore(0) } });
    });

  const restore = (index: number) =>
    run(async () => {
      open = false;
      await themeIpc.linkRestore(theme.id, index);
    });

  const ignore = () => run(() => themeIpc.linkIgnore(theme.id));
</script>

{#if link?.sourceId}
  <div class="from">
    {#if link.removed}
      <span class="quiet">The author removed this theme. Your copy stays.</span>
    {:else if next !== null && asking}
      <ConfirmChoices
        ask="Replace your changes?"
        choices={[{ label: 'Replace', variant: 'primary', run: () => void update(true) }]}
        disabled={busy}
        oncancel={() => (asking = false)}
      />
    {:else if next !== null}
      <span class="notice">{updateLabel(next)}</span>
      <Button small variant="primary" disabled={busy} onclick={() => void update(false)}>Update</Button>
      <Button small variant="ghost" disabled={busy} onclick={() => void ignore()}>Ignore this version</Button>
    {:else}
      <span class="quiet">From a link · v{link.sourceVersion}{link.edited ? ' · changed by you' : ''}</span>
    {/if}
    {#if link.history.length}
      <span class="wrap">
        <Button small variant="ghost" icon="history" disabled={busy} onclick={() => (open = !open)}>Go back</Button>
        <Popover bind:open align="end" role="menu" label="Earlier versions">
          <div class="list">
            {#each link.history as c, i (i)}
              <button class="row" role="menuitem" onclick={() => void restore(i)}>
                <Icon name="history" size={14} />
                <span class="name">{copyLabel(c)}</span>
              </button>
            {/each}
          </div>
        </Popover>
      </span>
    {/if}
  </div>
{/if}

<style>
  .from {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 10px 0 8px;
    border-bottom: 1px solid var(--border);
  }
  .notice {
    color: var(--text);
    font-weight: 600;
  }
  .quiet {
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .wrap {
    position: relative;
    margin-left: auto;
  }
  .list {
    display: flex;
    flex-direction: column;
    min-width: 200px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--surface-hover);
  }
  .row:focus-visible {
    outline-offset: -2px;
  }
  .row :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .name {
    white-space: nowrap;
  }
</style>
