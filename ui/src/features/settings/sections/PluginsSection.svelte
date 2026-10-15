<!-- S5 Lua plugins: the main switch, the plugins folder, and a card per plugin. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import EmptyState from '../../../components/EmptyState.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import { pluginsIpc } from '../../../ipc/plugins';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import PluginCard from '../../plugins/PluginCard.svelte';
  import { plugins } from '../../plugins/plugins.svelte';

  const enabled = $derived(!!settings.v.plugins?.enabled);
  const view = $derived(plugins.view);
  const live = $derived(enabled && !view?.safeMode);

  function lookAgain() {
    pluginsIpc.reload().catch((e) => toast.error(String(e)));
  }
</script>

<h2>Plugins</h2>
<p class="intro">
  Lua scripts that add commands, change chat lines and more. Each one only gets the permissions you agree to.
</p>

{#if view?.safeMode}
  <p class="note">Started with --safe-mode, so no plugin runs.</p>
{/if}

<Row setting="plugins.enabled" label="Turn on plugins" hint="Only add plugins from people you trust.">
  <Toggle label="Turn on plugins" checked={enabled} onchange={(v) => settings.patch('plugins', { enabled: v })} />
</Row>

<div class="actions" data-setting="plugins.list">
  <Button small icon="folder" onclick={() => void pluginsIpc.openFolder()}>Open plugins folder</Button>
  <Button small icon="refresh" onclick={lookAgain}>Look again</Button>
</div>

{#if view && !view.list.length}
  <EmptyState icon="puzzle" text="No plugins yet. Put a plugin’s folder into the plugins folder, then Look again." />
{:else if view}
  {#each view.list as p (p.id)}
    <PluginCard plugin={p} {live} />
  {/each}
{/if}

<style>
  .intro {
    margin: -6px 0 6px;
    color: var(--dim);
  }
  .note {
    margin: 0 0 6px;
    color: var(--yellow);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin: 10px 0 14px;
  }
</style>
