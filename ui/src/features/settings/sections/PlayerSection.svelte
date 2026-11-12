<!-- Player settings (1.1, 1.2, P10). The proxy address can hold a password, so it's kept
     in the Windows Credential Manager, never in settings files or exports. -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { Quality } from '../../../ipc/bindings/Quality';
  import { playerIpc } from '../../../ipc/player';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import ResetSection from '../ResetSection.svelte';
  import { errorText } from '../../../lib/errors';
  import Select from '../../../components/Select.svelte';
  import { stepChoice } from '../../player/liveKeys';
  import { REPLAY_MINUTES, replayChoice } from '../../player/liveEdge';
  import { AWAY_SECS, awayChoice } from '../../player/away';
  import type { AwayMode } from '../../../ipc/bindings/AwayMode';

  const p = $derived(settings.v.player);

  const STEPS = [5, 10, 15, 30].map((n) => ({ value: String(n), label: `${n} seconds` }));
  // V108-N24
  const REPLAYS = REPLAY_MINUTES.map((n) => ({ value: String(n), label: n ? `${n} minutes` : 'Off' }));

  // V108-N22
  const AWAY_MODES: { value: AwayMode; label: string }[] = [
    { value: 'lowest', label: 'Lowest quality' },
    { value: 'audio', label: 'Audio only' },
    { value: 'off', label: 'Keep playing as is' },
  ];
  const AWAYS = AWAY_SECS.map((n) => ({ value: String(n), label: n < 60 ? `${n} seconds` : n === 60 ? '1 minute' : `${n / 60} minutes` }));

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
      toast.success(url ? 'Proxy saved. New streams use it.' : 'Proxy removed.');
    } catch (e) {
      toast.error(errorText(e));
    }
  }
</script>

<SectionHead title="Player" level={2}>Ads, latency, rewind and how much the player uses.</SectionHead>

<Row setting="player.blockAds" label="Block ads" hint="Cuts Twitch’s ads out of streams, or plays a clean copy while they run.">
  <Toggle label="Block ads" checked={p.blockAds} onchange={(v) => settings.patch('player', { blockAds: v })} />
</Row>

<Row setting="player.lowLatency" label="Low latency" hint="A few seconds behind the stream instead of about ten.">
  <Toggle label="Low latency" checked={p.lowLatency} onchange={(v) => settings.patch('player', { lowLatency: v })} />
</Row>

<Row setting="player.liveRewind" label="Live VODs" hint="Go back in live streams where Twitch allows it: Turbo, or a sub without ads.">
  <Toggle label="Live VODs" checked={p.liveRewind} onchange={(v) => settings.patch('player', { liveRewind: v })} />
</Row>

<Row setting="player.replayMinutes" label="Replay" hint="Go back in what played here, where Twitch has no rewind. It replays what you watched, so time spent on lowest quality or audio only is in it. Dropped after a pause of over 10 minutes.">
  <Select label="Replay" width="160px" value={String(replayChoice(p.replayMinutes))} options={REPLAYS} onchange={(v) => settings.patch('player', { replayMinutes: Number(v) })} />
</Row>

<Row setting="player.seekStep" label="Skip step" hint="How far Left and Right, or J and L, jump.">
  <Select label="Skip step" width="160px" value={String(stepChoice(p.seekStep))} options={STEPS} onchange={(v) => settings.patch('player', { seekStep: Number(v) })} />
</Row>

<Row setting="player.quality" label="Quality" hint="What players start in. Each player’s gear menu changes its own.">
  <Select label="Quality" width="160px" value={p.quality} options={QUALITIES} onchange={(v) => settings.patch('player', { quality: v as Quality })} />
</Row>

<SectionHead title="Save resources" level={3} />

<Row setting="player.awayMode" label="Tabs out of sight" hint="A stream in a background tab or a minimized window. Picture in picture counts as in sight.">
  <Select label="Tabs out of sight" width="160px" value={p.awayMode} options={AWAY_MODES} onchange={(v) => settings.patch('player', { awayMode: v as AwayMode })} />
</Row>

{#if p.awayMode !== 'off'}
  <Row setting="player.awaySecs" label="After" hint="Back before that, nothing changes. Back later, the picture returns within a few seconds.">
    <Select label="After" width="160px" value={String(awayChoice(p.awaySecs))} options={AWAYS} onchange={(v) => settings.patch('player', { awaySecs: Number(v) })} />
  </Row>
{/if}

<Row setting="player.pauseHidden" label="Pause animations when hidden" hint="Animated emotes and chat effects rest while the window is hidden or minimized.">
  <Toggle label="Pause animations when hidden" checked={p.pauseHidden} onchange={(v) => settings.patch('player', { pauseHidden: v })} />
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
    <!-- P5 Saves when you leave the field or press Enter, like every other setting. -->
    <input
      class="field wide"
      type="password"
      aria-label="Ad-block proxy"
      placeholder="e.g. http://user:password@host:8080"
      bind:value={draft}
      onchange={() => draft.trim() && void saveProxy(draft)}
    />
  {/if}
</Row>

<ResetSection partition="player" page="player" />

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
