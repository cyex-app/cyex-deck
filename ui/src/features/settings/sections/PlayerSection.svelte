<!-- Player settings (1.1, 1.2, P10). The proxy address can hold a password, so it's kept
     in the Windows Credential Manager, never in settings files or exports. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { Quality } from '../../../ipc/bindings/Quality';
  import { playerIpc } from '../../../ipc/player';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import ResetSection from '../ResetSection.svelte';

  const p = $derived(settings.v.player);

  const QUALITIES: { value: Quality; label: string }[] = [
    { value: 'auto', label: 'Auto: fits the player' },
    { value: 'source', label: 'Source' },
    { value: '1080', label: '1080p' },
    { value: '720', label: '720p' },
    { value: '480', label: '480p' },
    { value: '360', label: '360p' },
    { value: '160', label: '160p' },
  ];

  let proxy = $state<string | null>(null);
  let draft = $state('');

  void playerIpc.proxy().then((p) => (proxy = p));

  async function saveProxy(url: string | null) {
    try {
      await playerIpc.setProxy(url);
      proxy = await playerIpc.proxy();
      draft = '';
      toast.info(url ? 'Proxy saved. New streams use it.' : 'Proxy removed.');
    } catch (e) {
      toast.error(String(e));
    }
  }
</script>

<h2>Player</h2>

<Row setting="player.blockAds" label="Block ads" hint="Cuts Twitch’s ads out of streams, or plays a clean copy while they run.">
  <Toggle label="Block ads" checked={p.blockAds} onchange={(v) => settings.patch('player', { blockAds: v })} />
</Row>

<Row setting="player.lowLatency" label="Low latency" hint="A few seconds behind the stream instead of about ten.">
  <Toggle label="Low latency" checked={p.lowLatency} onchange={(v) => settings.patch('player', { lowLatency: v })} />
</Row>

<Row setting="player.quality" label="Quality" hint="What players start in. Each player’s gear menu changes its own.">
  <select
    class="field"
    value={p.quality}
    onchange={(e) => settings.patch('player', { quality: e.currentTarget.value as Quality })}
  >
    {#each QUALITIES as q (q.value)}<option value={q.value}>{q.label}</option>{/each}
  </select>
</Row>

<Row
  setting="player:proxy"
  label="Ad-block proxy"
  hint="When no clean copy is found, streams load through this HTTP proxy, e.g. one in a country without ads."
  stacked
>
  {#if proxy}
    <span class="current">Using {proxy}</span>
    <Button small variant="ghost" onclick={() => void saveProxy(null)}>Remove</Button>
  {:else}
    <input class="field wide" type="password" placeholder="http://user:password@host:8080" bind:value={draft} />
    <Button small disabled={!draft.trim()} onclick={() => void saveProxy(draft)}>Save</Button>
  {/if}
</Row>

<ResetSection partition="player" />

<style>
  .field {
    width: 200px;
  }
  .wide {
    width: 320px;
  }
  .current {
    font-family: var(--font-mono);
    font-size: 0.85rem;
  }
</style>
