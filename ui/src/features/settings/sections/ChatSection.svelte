<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { DeletedStyle } from '../../../ipc/bindings/DeletedStyle';
  import type { GifMode } from '../../../ipc/bindings/GifMode';
  import type { NameStyle } from '../../../ipc/bindings/NameStyle';
  import type { PauseKey } from '../../../ipc/bindings/PauseKey';
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
  const DELETED: { value: DeletedStyle; label: string }[] = [
    { value: 'strike', label: 'Strike' },
    { value: 'faded', label: 'Faded' },
    { value: 'hidden', label: 'Hidden' },
  ];
  const NAMES: { value: NameStyle; label: string }[] = [
    { value: 'display', label: 'Display name' },
    { value: 'login', label: 'Login' },
    { value: 'both', label: 'Both' },
  ];
  const PAUSE_KEYS: { value: PauseKey; label: string }[] = [
    { value: 'none', label: 'None' },
    { value: 'ctrl', label: 'Ctrl' },
    { value: 'alt', label: 'Alt' },
    { value: 'shift', label: 'Shift' },
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

<h3 class="label">Messages</h3>
<Row label="Recent history" hint="Shows the last messages, dimmed, when you open a chat.">
  <Toggle label="Recent history" checked={c.history} onchange={(v) => settings.patch('chat', { history: v })} />
</Row>
<Row label="Deleted messages" hint="Removed by a mod, or from someone timed out or banned.">
  <Segmented
    label="Deleted messages"
    options={DELETED}
    value={c.deleted}
    onchange={(v) => settings.patch('chat', { deleted: v })}
  />
</Row>
<Row label="Names" hint="Both adds the login when the display name is in another script.">
  <Segmented label="Names" options={NAMES} value={c.names} onchange={(v) => settings.patch('chat', { names: v })} />
</Row>
<Row label="Alternate line backgrounds">
  <Toggle
    label="Alternate line backgrounds"
    checked={c.alternateLines}
    onchange={(v) => settings.patch('chat', { alternateLines: v })}
  />
</Row>
<Row label="Lines between messages">
  <Toggle label="Lines between messages" checked={c.separators} onchange={(v) => settings.patch('chat', { separators: v })} />
</Row>

<h3 class="label">Scrolling</h3>
<Row label="Pause on hover" hint="Chat stops moving while the mouse is over it.">
  <Toggle label="Pause on hover" checked={c.pauseOnHover} onchange={(v) => settings.patch('chat', { pauseOnHover: v })} />
</Row>
<Row label="Pause while holding">
  <Segmented
    label="Pause while holding"
    options={PAUSE_KEYS}
    value={c.pauseKey}
    onchange={(v) => settings.patch('chat', { pauseKey: v })}
  />
</Row>
<Row label="Last-read line" hint="Marks where you were when you switched away.">
  <Toggle label="Last-read line" checked={c.lastReadLine} onchange={(v) => settings.patch('chat', { lastReadLine: v })} />
</Row>

<h3 class="label">Mentions</h3>
<Row label="Sound" hint="Plays when someone mentions you or replies to you.">
  <Toggle label="Mention sound" checked={c.mentionSound} onchange={(v) => settings.patch('chat', { mentionSound: v })} />
</Row>
<Row label="Flash the taskbar" hint="When the app isn't in front.">
  <Toggle label="Flash the taskbar" checked={c.mentionFlash} onchange={(v) => settings.patch('chat', { mentionFlash: v })} />
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
