<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { GifMode } from '../../../ipc/bindings/GifMode';
  import type { UnlistedEmotes } from '../../../ipc/bindings/UnlistedEmotes';
  import { formatDuration, formatTime, parseDuration } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import ResetSection from '../ResetSection.svelte';

  const c = $derived(settings.v.chat);

  const BADGES = [
    { key: 'twitch', label: 'Twitch' },
    { key: 'seventv', label: '7TV' },
    { key: 'bttv', label: 'BTTV' },
    { key: 'ffz', label: 'FFZ' },
  ] as const;
  const PAINTS = [
    { key: 'seventv', label: '7TV' },
    { key: 'bttv', label: 'BTTV' },
  ] as const;
  const UNLISTED: { value: UnlistedEmotes; label: string }[] = [
    { value: 'hide', label: 'Hide' },
    { value: 'blur', label: 'Blur' },
    { value: 'show', label: 'Show' },
  ];
  const GIFS: { value: GifMode; label: string }[] = [
    { value: 'show', label: 'Show GIF' },
    { value: 'link', label: 'Link' },
    { value: 'text', label: 'Plain text' },
  ];
  const SAMPLE_TIME = new Date(2026, 0, 1, 14, 3, 27);

  let newPreset = $state('');

  function addPreset() {
    const secs = parseDuration(newPreset);
    if (secs === null || secs > 1_209_600) {
      toast.error('Use a duration like 30s, 10m, 2h or 7d, up to 14 days.');
      return;
    }
    void settings.patch('chat', { timeoutPresets: [...c.timeoutPresets, secs] });
    newPreset = '';
  }

  function removePreset(secs: number) {
    void settings.patch('chat', { timeoutPresets: c.timeoutPresets.filter((s) => s !== secs) });
  }
</script>

<h2>Chat</h2>

<h3 class="label">Badges</h3>
{#each BADGES as b (b.key)}
  <Row label="{b.label} badges">
    <Toggle
      label="{b.label} badges"
      checked={c.badges[b.key]}
      onchange={(v) => settings.patch('chat', { badges: { [b.key]: v } })}
    />
  </Row>
{/each}

<h3 class="label">Name paints</h3>
{#each PAINTS as p (p.key)}
  <Row label="{p.label} name paints">
    <Toggle
      label="{p.label} name paints"
      checked={c.paints[p.key]}
      onchange={(v) => settings.patch('chat', { paints: { [p.key]: v } })}
    />
  </Row>
{/each}

<h3 class="label">Emotes</h3>
<Row label="Twitch emotes">
  <Toggle label="Twitch emotes" checked={c.twitchEmotes} onchange={(v) => settings.patch('chat', { twitchEmotes: v })} />
</Row>
<Row label="7TV, BTTV and FFZ emotes">
  <Toggle
    label="7TV, BTTV and FFZ emotes"
    checked={c.thirdPartyEmotes}
    onchange={(v) => settings.patch('chat', { thirdPartyEmotes: v })}
  />
</Row>
<Row label="Unlisted 7TV emotes" hint="Emotes 7TV hasn't approved for public listing.">
  <Segmented
    label="Unlisted 7TV emotes"
    options={UNLISTED}
    value={c.unlistedEmotes}
    onchange={(v) => settings.patch('chat', { unlistedEmotes: v })}
  />
</Row>
<Row label="GIFs">
  <Segmented label="GIFs" options={GIFS} value={c.gifs} onchange={(v) => settings.patch('chat', { gifs: v })} />
</Row>

<h3 class="label">Timestamps</h3>
<Row label="Show timestamps" hint={c.timestamps.show ? `Looks like ${formatTime(SAMPLE_TIME, c.timestamps)}` : undefined}>
  <Toggle
    label="Show timestamps"
    checked={c.timestamps.show}
    onchange={(v) => settings.patch('chat', { timestamps: { show: v } })}
  />
</Row>
<Row label="Seconds">
  <Toggle
    label="Seconds"
    disabled={!c.timestamps.show}
    checked={c.timestamps.seconds}
    onchange={(v) => settings.patch('chat', { timestamps: { seconds: v } })}
  />
</Row>
<Row label="12-hour clock">
  <Toggle
    label="12-hour clock"
    disabled={!c.timestamps.show}
    checked={c.timestamps.hour12}
    onchange={(v) => settings.patch('chat', { timestamps: { hour12: v } })}
  />
</Row>

<h3 class="label">Moderation</h3>
<Row label="Timeout buttons" hint="Quick durations in chat and on user cards. Up to 8." stacked>
  {#each c.timeoutPresets as secs (secs)}
    <span class="chip">
      {formatDuration(secs)}
      <button aria-label="Remove {formatDuration(secs)}" onclick={() => removePreset(secs)}>×</button>
    </span>
  {/each}
  {#if c.timeoutPresets.length < 8}
    <form
      onsubmit={(e) => {
        e.preventDefault();
        addPreset();
      }}
    >
      <input class="field add" placeholder="Add, e.g. 30m" bind:value={newPreset} />
    </form>
  {/if}
</Row>

<ResetSection partition="chat" />

<style>
  h3 {
    margin: 22px 0 2px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 4px 0 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    font: 600 0.82rem var(--font-mono);
  }
  .chip button {
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .chip button:hover {
    color: var(--red);
  }
  .add {
    width: 130px;
    height: 28px;
  }
</style>
