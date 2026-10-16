<!-- S5 One plugin in Settings: who made it, what it may do, on or off, its settings,
     reload and console. Turning it on asks first when it wants permissions. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { popOut } from '../../app/panels';
  import type { PluginInfo } from '../../ipc/bindings/PluginInfo';
  import { pluginsIpc } from '../../ipc/plugins';
  import { systemIpc } from '../../ipc/system';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import PluginField from './PluginField.svelte';
  import { STATUS, menuSummary, permissionText } from './permissions';
  import { errorText } from '../../lib/errors';

  interface Props {
    plugin: PluginInfo;
    /** Plugins are turned on in general and not in safe mode. */
    live: boolean;
  }

  let { plugin, live }: Props = $props();

  const m = $derived(plugin.info);
  const on = $derived(!!settings.v.plugins?.list[plugin.id]?.on);
  const status = $derived(STATUS[plugin.status]);
  let asking = $state(false);

  function setOn(next: boolean) {
    if (next && m?.permissions.length && !asking) {
      asking = true;
      return;
    }
    asking = false;
    pluginsIpc.setOn(plugin.id, next).catch((e) => toast.error(errorText(e)));
  }

  function reload() {
    pluginsIpc.reload(plugin.id).catch((e) => toast.error(errorText(e)));
  }
</script>

<section class="card">
  <header>
    <div class="who">
      <Icon name="puzzle" size={16} />
      <strong>{m?.name ?? plugin.id}</strong>
      {#if m}<span class="muted">{m.version}</span>{/if}
    </div>
    <span class="status {status.tone}">{live || plugin.status !== 'off' ? status.text : 'Plugins are off'}</span>
    {#if m}
      <Toggle label="Turn {m.name} on" checked={on} onchange={setOn} />
    {/if}
  </header>

  {#if m?.description}<p class="desc">{m.description}</p>{/if}
  {#if m && (m.authors.length || m.license || m.homepage)}
    <p class="meta muted">
      {[m.authors.join(', '), m.license].filter(Boolean).join(' · ')}
      {#if m.homepage}·
        <button class="link" onclick={() => void systemIpc.openChatLink(m.homepage!)}>{m.homepage.replace(/^https?:\/\//, '')}</button
        >{/if}
    </p>
  {/if}

  {#if plugin.error}<p class="error">{plugin.error}</p>{/if}
  {#if plugin.status === 'needsOk'}
    <p class="warn">
      An update asks for more than you agreed to.
      <Button small onclick={() => (asking = true)}>See what</Button>
    </p>
  {/if}

  {#if m?.permissions.length}
    <ul class="perms">
      {#each m.permissions as p, i (i)}
        {@const t = permissionText(p)}
        <li><Icon name={t.icon} size={13} />{t.text}</li>
      {/each}
    </ul>
  {/if}

  {#if plugin.exports.commands.length || plugin.exports.menu.length}
    <p class="meta muted">
      {#if plugin.exports.commands.length}Commands: {plugin.exports.commands.map((c) => `/${c}`).join(', ')}{/if}
      {#if plugin.exports.commands.length && plugin.exports.menu.length}·{/if}
      {#if plugin.exports.menu.length}Menu entries: {menuSummary(plugin.exports.menu)}{/if}
    </p>
  {/if}
  {#if plugin.exports.paused}
    <p class="warn">Its line callback was too slow and is paused. Reload to try again.</p>
  {/if}

  {#if m?.settings.length}
    <div class="fields">
      {#each m.settings as f (f.key)}
        <PluginField plugin={plugin.id} field={f} value={plugin.values[f.key]} />
      {/each}
    </div>
  {/if}

  <footer>
    <IconButton icon="folder" label="Open its folder" size={15} onclick={() => void pluginsIpc.openFolder(plugin.id)} />
    <IconButton
      icon="refresh"
      label="Reload"
      hint="Reads its files again and starts it fresh"
      size={15}
      onclick={reload}
    />
    {#if m}
      <IconButton
        icon="terminal"
        label="Console"
        hint="Its log, and a line to run Lua in it"
        size={15}
        onclick={() => void popOut(`plugin/${plugin.id}`, `${m.name} console`)}
      />
    {/if}
  </footer>
</section>

{#if asking && m}
  <Dialog title="Turn on {m.name}?" onclose={() => (asking = false)}>
    <p>It will be able to:</p>
    <ul class="perms big">
      {#each m.permissions as p, i (i)}
        {@const t = permissionText(p)}
        <li><Icon name={t.icon} size={15} />{t.text}</li>
      {/each}
    </ul>
    <p class="muted small">Only turn on plugins from people you trust.</p>
    {#snippet footer()}
      <Button variant="ghost" onclick={() => (asking = false)}>Cancel</Button>
      <Button variant="primary" onclick={() => setOn(true)}>Turn on</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .card {
    margin-bottom: 12px;
    padding: 12px 16px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .who {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .who strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    font-size: 0.8rem;
  }
  .ok {
    color: var(--green);
  }
  .muted {
    color: var(--dim);
  }
  .warn {
    color: var(--yellow);
  }
  .bad {
    color: var(--red);
  }
  p {
    margin: 6px 0 0;
    font-size: 0.85rem;
  }
  .meta {
    font-size: 0.78rem;
  }
  .link {
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text);
    font: inherit;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .error {
    color: var(--red);
    font-family: var(--font-mono);
    font-size: 0.78rem;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  p.warn {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .perms {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .perms li {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .perms.big {
    gap: 6px;
    margin: 6px 0 10px;
    font-size: 0.87rem;
    color: var(--text);
  }
  .small {
    font-size: 0.8rem;
  }
  .fields {
    margin-top: 6px;
    border-top: 1px solid var(--border);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 2px;
    margin-top: 8px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }
</style>
