<!-- The points miner (08): on or off, which account mines, and its channels (1.31,
     1.33, 1.44, W3), stats (1.32) and log (W4). Settings → Miner has the rest. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import GateFlag from '../../components/GateFlag.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { type MenuEntry, openMenuAt } from '../../components/menu.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { miner } from './miner.svelte';
  import MinerChannels from './MinerChannels.svelte';
  import MinerLog from './MinerLog.svelte';
  import MinerStats from './MinerStats.svelte';

  interface Props {
    /** Opens Settings → Miner. */
    onsettings: () => void;
  }

  let { onsettings }: Props = $props();

  type Page = 'channels' | 'stats' | 'log';
  const PAGES: { value: Page; label: string }[] = [
    { value: 'channels', label: 'Channels' },
    { value: 'stats', label: 'Stats' },
    { value: 'log', label: 'Log' },
  ];
  let page = $state<Page>('channels');

  const s = $derived(settings.v.miner);
  const able = $derived(accounts.list.filter((a) => a.features.includes('miner')));
  const owner = $derived(
    accounts.list.find((a) => a.userId === (miner.status?.account ?? s.account ?? accounts.current?.userId)),
  );
  const line = $derived.by(() => {
    const st = miner.status;
    if (!s.enabled) return 'Off';
    if (!st) return '';
    if (st.problem) return st.problem;
    const mining = miner.mining;
    const watching = st.channels.filter((c) => c.watching).map((c) => c.name);
    const parts = [];
    if (mining.length) parts.push(`Mining ${mining.join(' and ')}`);
    if (watching.length) parts.push(`watching ${watching.join(' and ')} here`);
    if (parts.length) return parts.join(', ');
    return st.channels.some((c) => c.live) ? 'Starting…' : 'Nothing live to mine';
  });

  function pickAccount(e: MouseEvent) {
    const items: MenuEntry[] = able.map((a) => ({
      label: a.displayName,
      checked: a.userId === miner.status?.account,
      run: () => void settings.patch('miner', { account: a.userId }),
    }));
    openMenuAt(e, items);
  }
</script>

<div class="miner">
  <header>
    <h1>Miner</h1>
    {#if owner}
      <button class="account" disabled={able.length < 2} onclick={pickAccount}>
        <Avatar src={owner.avatar} name={owner.displayName} size={20} />
        <span>{owner.displayName}</span>
        {#if able.length > 1}<Icon name="chevronDown" size={14} />{/if}
      </button>
    {/if}
    <Segmented label="Page" options={PAGES} value={page} onchange={(p) => (page = p)} />
    <span class="status" class:on={miner.mining.length > 0}>{line}</span>
    <span class="grow"></span>
    <span class="switch" use:tooltip={{ label: 'Mining', hint: 'Watches channels in the background to collect points and drops.' }}>
      <span class="sw-label">Mining</span>
      <Toggle label="Mining" checked={s.enabled} onchange={(enabled) => void settings.patch('miner', { enabled })} />
    </span>
    <IconButton icon="settings" label="Miner settings" hint="Which channels win, raids, bets and drops." onclick={onsettings} />
  </header>

  <div class="scroll">
    {#if !able.length}
      <div class="note">
        <span>The miner needs an account with a GQL sign-in.</span>
        <GateFlag kind="gql" />
      </div>
    {:else if page === 'channels'}
      <MinerChannels />
    {:else if page === 'stats'}
      <MinerStats />
    {:else}
      <MinerLog />
    {/if}
  </div>
</div>

<style>
  .miner {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    margin: 0 4px 0 0;
    font-size: 1.05rem;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px 0 4px;
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.8rem;
  }
  .account:disabled {
    opacity: 1;
    cursor: default;
  }
  .account:not(:disabled):hover {
    background: var(--surface-hover);
  }
  .status {
    min-width: 0;
    overflow: hidden;
    color: var(--dim);
    font-size: 0.82rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status.on {
    color: var(--accent-text);
  }
  .switch {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .sw-label {
    color: var(--muted);
    font-size: 0.82rem;
  }
  .grow {
    flex: 1;
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 12px 20px 40px;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--muted);
    font-size: 0.85rem;
  }
</style>
