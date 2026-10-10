<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { DeletedStyle } from '../../../ipc/bindings/DeletedStyle';
  import type { GifMode } from '../../../ipc/bindings/GifMode';
  import type { LinkThumbs } from '../../../ipc/bindings/LinkThumbs';
  import type { NameStyle } from '../../../ipc/bindings/NameStyle';
  import type { PauseKey } from '../../../ipc/bindings/PauseKey';
  import { formatDuration, formatTime, parseDuration } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import ResetSection from '../ResetSection.svelte';

  const c = $derived(settings.v.chat);

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
  const SIMILAR: { value: number; label: string }[] = [
    { value: 60, label: 'Loosely' },
    { value: 80, label: 'Mostly' },
    { value: 100, label: 'Exactly' },
  ];
  const THUMBS: { value: LinkThumbs; label: string }[] = [
    { value: 'off', label: 'None' },
    { value: 'small', label: 'Small' },
    { value: 'medium', label: 'Medium' },
    { value: 'large', label: 'Large' },
  ];
  /** C19 FFZ's steps, in tenths of a second. */
  const DELAYS = [0, 3, 12, 50, 100, 150, 200, 300, 600].map((t) => ({
    value: t,
    label: t === 0 ? 'Off' : `${(t / 10).toString().replace('.', ',')} s`,
  }));
  const LANGUAGES = [
    { value: '', label: 'Same as Windows' },
    { value: 'en', label: 'English' },
    { value: 'de', label: 'Deutsch' },
    { value: 'es', label: 'Español' },
    { value: 'fr', label: 'Français' },
    { value: 'it', label: 'Italiano' },
    { value: 'pt', label: 'Português' },
    { value: 'nl', label: 'Nederlands' },
    { value: 'pl', label: 'Polski' },
    { value: 'tr', label: 'Türkçe' },
    { value: 'ru', label: 'Русский' },
    { value: 'uk', label: 'Українська' },
    { value: 'sv', label: 'Svenska' },
    { value: 'ja', label: '日本語' },
    { value: 'ko', label: '한국어' },
    { value: 'zh-CN', label: '中文' },
    { value: 'ar', label: 'العربية' },
  ];

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

<h3 class="label">GIFs and effects</h3>
<Row label="GIFs">
  <Segmented label="GIFs" options={GIFS} value={c.gifs} onchange={(v) => settings.patch('chat', { gifs: v })} />
</Row>
<Row label="Effects" hint="Animated messages, giant emotes and celebrations, as Twitch shows them.">
  <Toggle label="Effects" checked={c.effects} onchange={(v) => settings.patch('chat', { effects: v })} />
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

<Row label="Count repeats" hint="Someone saying the same thing again adds a count to their line.">
  <Toggle label="Count repeats" checked={c.repeats} onchange={(v) => settings.patch('chat', { repeats: v })} />
</Row>
{#if c.repeats}
  <Row label="Counts as the same">
    <Segmented
      label="Counts as the same"
      options={SIMILAR}
      value={c.repeatSimilarity}
      onchange={(v) => settings.patch('chat', { repeatSimilarity: v })}
    />
  </Row>
{/if}
<Row label="Show where it was sent from" hint="An icon for messages from the website or the phone apps.">
  <Toggle label="Show where it was sent from" checked={c.showClient} onchange={(v) => settings.patch('chat', { showClient: v })} />
</Row>
<Row label="Hold chat back" hint="Waits this long before showing messages, to match the stream's delay.">
  <select
    class="field"
    value={c.delayTenths}
    onchange={(e) => settings.patch('chat', { delayTenths: Number(e.currentTarget.value) })}
  >
    {#each DELAYS as d (d.value)}<option value={d.value}>{d.label}</option>{/each}
  </select>
</Row>

<h3 class="label">Links</h3>
<Row label="Previews" hint="Hover a link for its title and picture.">
  <Toggle label="Previews" checked={c.linkPreviews} onchange={(v) => settings.patch('chat', { linkPreviews: v })} />
</Row>
<Row label="Picture size">
  <Segmented
    label="Picture size"
    options={THUMBS}
    value={c.linkThumbnails}
    onchange={(v) => settings.patch('chat', { linkThumbnails: v })}
  />
</Row>
<Row label="Cards under messages" hint="Also shows the preview below the message.">
  <Toggle label="Cards under messages" checked={c.linkCards} onchange={(v) => settings.patch('chat', { linkCards: v })} />
</Row>

<h3 class="label">Writing and reading</h3>
<Row label="Spell check" hint="Underlines typos in the message box. Right-click one for fixes.">
  <Toggle label="Spell check" checked={c.spellCheck} onchange={(v) => settings.patch('chat', { spellCheck: v })} />
</Row>
<Row label="Translate into" hint="For Translate and /translate. Each chat's menu can translate everything.">
  <select
    class="field"
    value={c.translateTo}
    onchange={(e) => settings.patch('chat', { translateTo: e.currentTarget.value })}
  >
    {#each LANGUAGES as l (l.value)}<option value={l.value}>{l.label}</option>{/each}
  </select>
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
