<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import DragList from '../../../components/DragList.svelte';
  import Icon from '../../../components/Icon.svelte';
  import type { IconName } from '../../../components/icons';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { DeletedStyle } from '../../../ipc/bindings/DeletedStyle';
  import type { GifMode } from '../../../ipc/bindings/GifMode';
  import type { LinkThumbs } from '../../../ipc/bindings/LinkThumbs';
  import type { ModButton } from '../../../ipc/bindings/ModButton';
  import type { ModButtons } from '../../../ipc/bindings/ModButtons';
  import type { NameStyle } from '../../../ipc/bindings/NameStyle';
  import type { PauseKey } from '../../../ipc/bindings/PauseKey';
  import { formatDuration, formatNumber, formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { addTimeout, buttonKey, buttonOn, canAddButton, fullOrder, retimeButton, toggleButton } from '../../modview/lineMod';
  import { timeoutSecs } from '../../modview/timeoutLength';
  import ResetSection from '../ResetSection.svelte';
  import Select from '../../../components/Select.svelte';
  import { choicesOf } from '../../../components/select.ts';

  const c = $derived(settings.v.chat);
  const o = $derived(c.overlay);

  /** U7 Seconds before overlay lines fade. */
  const FADES: { value: number; label: string }[] = [
    { value: 0, label: 'Never' },
    { value: 15, label: '15s' },
    { value: 30, label: '30s' },
    { value: 60, label: '60s' },
    { value: 300, label: '5m' },
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
  /** V101-02, V101-03 Chatterino goes up to 8.000 kept and 2.000 loaded. */
  const KEPT = [100, 200, 500, 1000, 2000, 4000, 8000].map((n) => ({ value: n, label: formatNumber(n) }));
  const LOADED = [50, 100, 200, 500, 800, 1000, 2000].map((n) => ({ value: n, label: formatNumber(n) }));
  /** V101-15 Tenths of a second; 0 waits for the mouse to leave. */
  const STILL = [0, 5, 10, 20, 50].map((t) => ({
    value: t,
    label: t === 0 ? 'Until the mouse leaves' : `${(t / 10).toString().replace('.', ',')}s`,
  }));
  /** C19 FFZ's steps, in tenths of a second. */
  const DELAYS = [0, 3, 12, 50, 100, 150, 200, 300, 600].map((t) => ({
    value: t,
    label: t === 0 ? 'Off' : `${(t / 10).toString().replace('.', ',')}s`,
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
  /** V107-R22 What a typed length was refused for, under its field. */
  let presetError = $state('');

  function addPreset() {
    const t = timeoutSecs(newPreset);
    if ('error' in t) {
      presetError = t.error;
      return;
    }
    void settings.patch('chat', { timeoutPresets: [...c.timeoutPresets, t.secs] });
    newPreset = '';
  }

  function removePreset(secs: number) {
    void settings.patch('chat', { timeoutPresets: c.timeoutPresets.filter((s) => s !== secs) });
  }

  /** M10 Mod mode's buttons: your timeout lengths plus any already picked. */
  const mb = $derived(c.modButtons);
  const modTimes = $derived([...new Set([...c.timeoutPresets, ...mb.timeouts])].sort((a, b) => a - b));
  const flipMod = (patch: Partial<ModButtons>) => void settings.patch('chat', { modButtons: patch });

  function flipTime(secs: number) {
    const on = mb.timeouts.includes(secs);
    flipMod({ timeouts: on ? mb.timeouts.filter((s) => s !== secs) : [...mb.timeouts, secs] });
  }

  /** V105-15 The order list: every button with its place, as it looks in chat. */
  const modOrder = $derived(fullOrder(mb, modTimes));
  const modName = (b: ModButton) =>
    typeof b !== 'string' ? `Time out ${formatDuration(b.timeout)}` : b === 'delete' ? 'Delete' : b === 'warn' ? 'Warn' : b === 'unban' ? 'Unban' : 'Ban';
  const modIcon = (b: ModButton): IconName =>
    typeof b !== 'string' ? 'clock' : b === 'delete' ? 'trash' : b === 'warn' ? 'alert' : b === 'unban' ? 'undo' : 'gavel';
  const modOn = (b: ModButton) => buttonOn(mb, b);

  /** V107-R22 A timeout button typed in: any length Twitch takes. */
  let newButton = $state('');
  let buttonError = $state('');
  function addButton() {
    const t = timeoutSecs(newButton);
    const patch = 'error' in t ? t.error : addTimeout(mb, t.secs);
    if (typeof patch === 'string') {
      buttonError = patch;
      return;
    }
    flipMod(patch);
    newButton = '';
  }

  /** V107-R22 The timeout row being given another length, and what was typed. */
  let editing = $state<number | null>(null);
  let editText = $state('');
  let editError = $state('');
  function startEdit(secs: number) {
    editing = secs;
    editText = formatDuration(secs).replace(' ', '');
    editError = '';
  }
  function saveEdit() {
    if (editing === null) return;
    const from = editing;
    const t = timeoutSecs(editText);
    const patch = 'error' in t ? t.error : retimeButton(mb, from, t.secs, modTimes);
    if (typeof patch === 'string') {
      editError = patch;
      return;
    }
    editing = null;
    if ('error' in t || !patch.order) return; // the same length: nothing to save
    const to = t.secs;
    // A preset's own row changes the preset too, or the old length would come back as a row.
    const timeoutPresets = c.timeoutPresets.map((s) => (s === from ? to : s));
    void settings.patch('chat', { modButtons: patch, timeoutPresets });
  }
  function editKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      saveEdit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      editing = null;
    }
  }
  const focus = (el: HTMLInputElement) => {
    el.focus();
    el.select();
  };
</script>

<SectionHead title="Chat" level={2} />

<SectionHead title="History">Older messages loaded when you open a chat.</SectionHead>
<Row setting="chat.history" label="Recent history" hint="Shows the last messages, dimmed, when you open a chat.">
  <Toggle label="Recent history" checked={c.history} onchange={(v) => settings.patch('chat', { history: v })} />
</Row>
<Row setting="chat.historyLines" label="History to load" hint="Up to this many. The history service keeps 800 per channel.">
  <Select
    label="History to load"
    width="120px"
    disabled={!c.history}
    value={String(c.historyLines)}
    options={choicesOf(LOADED)}
    onchange={(v) => settings.patch('chat', { historyLines: Number(v) })}
  />
</Row>
<Row setting="chat.scrollback" label="Messages kept" hint="Per chat. Older ones drop off the top; more uses more memory.">
  <Select
    label="Messages kept"
    width="120px"
    value={String(c.scrollback)}
    options={choicesOf(KEPT)}
    onchange={(v) => settings.patch('chat', { scrollback: Number(v) })}
  />
</Row>

<SectionHead title="Messages" />
<Row setting="chat.names" label="Names" hint="Both adds the login when the display name is in another script.">
  <Segmented label="Names" options={NAMES} value={c.names} onchange={(v) => settings.patch('chat', { names: v })} />
</Row>
<Row setting="chat.deleted" label="Deleted messages" hint="Removed by a mod, or from someone timed out or banned.">
  <Segmented
    label="Deleted messages"
    options={DELETED}
    value={c.deleted}
    onchange={(v) => settings.patch('chat', { deleted: v })}
  />
</Row>
<Row setting="chat.repeats" label="Count repeats" hint="Someone saying the same thing again adds a count to their line.">
  <Toggle label="Count repeats" checked={c.repeats} onchange={(v) => settings.patch('chat', { repeats: v })} />
</Row>
{#if c.repeats}
  <Row setting="chat.repeatSimilarity" label="Counts as the same" hint="How alike two messages must be.">
    <Segmented
      label="Counts as the same"
      options={SIMILAR}
      value={c.repeatSimilarity}
      onchange={(v) => settings.patch('chat', { repeatSimilarity: v })}
    />
  </Row>
{/if}
<Row setting="chat.joinsLeaves" label="Joins and leaves" hint="Who opens or leaves the chat, a line every 10 seconds. Twitch only says so in chats under 1.000 people.">
  <Toggle label="Joins and leaves" checked={c.joinsLeaves} onchange={(v) => settings.patch('chat', { joinsLeaves: v })} />
</Row>
<Row setting="chat.showClient" label="Show where it was sent from" hint="An icon for messages from the website or the phone apps.">
  <Toggle label="Show where it was sent from" checked={c.showClient} onchange={(v) => settings.patch('chat', { showClient: v })} />
</Row>
<Row setting="chat.alternateLines" label="Alternate line backgrounds">
  <Toggle
    label="Alternate line backgrounds"
    checked={c.alternateLines}
    onchange={(v) => settings.patch('chat', { alternateLines: v })}
  />
</Row>
<Row setting="chat.separators" label="Lines between messages">
  <Toggle label="Lines between messages" checked={c.separators} onchange={(v) => settings.patch('chat', { separators: v })} />
</Row>

<SectionHead title="Timestamps" />
<Row setting="chat.timestamps.show" label="Show timestamps" hint={c.timestamps.show ? `Looks like ${formatTime(SAMPLE_TIME, c.timestamps)}` : undefined}>
  <Toggle
    label="Show timestamps"
    checked={c.timestamps.show}
    onchange={(v) => settings.patch('chat', { timestamps: { show: v } })}
  />
</Row>
<Row setting="chat.timestamps.seconds" label="Seconds">
  <Toggle
    label="Seconds"
    disabled={!c.timestamps.show}
    checked={c.timestamps.seconds}
    onchange={(v) => settings.patch('chat', { timestamps: { seconds: v } })}
  />
</Row>
<Row setting="chat.timestamps.hour12" label="12-hour clock">
  <Toggle
    label="12-hour clock"
    disabled={!c.timestamps.show}
    checked={c.timestamps.hour12}
    onchange={(v) => settings.patch('chat', { timestamps: { hour12: v } })}
  />
</Row>

<SectionHead title="Scrolling">When chat moves and when it holds still.</SectionHead>
<Row setting="chat.pauseOnHover" label="Pause on hover" hint="Chat stops moving while the mouse is over it.">
  <Toggle label="Pause on hover" checked={c.pauseOnHover} onchange={(v) => settings.patch('chat', { pauseOnHover: v })} />
</Row>
<Row setting="chat.pauseStillTenths" label="Pause lasts" hint="Once the mouse stops moving over chat. Moving it pauses again.">
  <Select
    label="Pause lasts"
    width="200px"
    disabled={!c.pauseOnHover}
    value={String(c.pauseStillTenths)}
    options={choicesOf(STILL)}
    onchange={(v) => settings.patch('chat', { pauseStillTenths: Number(v) })}
  />
</Row>
<Row setting="chat.pauseKey" label="Pause while holding" hint="Hold this key to stop chat for a moment.">
  <Segmented
    label="Pause while holding"
    options={PAUSE_KEYS}
    value={c.pauseKey}
    onchange={(v) => settings.patch('chat', { pauseKey: v })}
  />
</Row>
<Row setting="chat.smoothNewMessages" label="Smooth new messages" hint="New messages slide in. Fast chats skip it, so you never fall behind.">
  <Toggle
    label="Smooth new messages"
    checked={c.smoothNewMessages}
    onchange={(v) => settings.patch('chat', { smoothNewMessages: v })}
  />
</Row>
<Row setting="chat.lastReadLine" label="Last-read line" hint="Marks where you were when you switched away.">
  <Toggle label="Last-read line" checked={c.lastReadLine} onchange={(v) => settings.patch('chat', { lastReadLine: v })} />
</Row>
<Row setting="chat.delayTenths" label="Hold chat back" hint="Waits this long before showing messages, to match the stream’s delay.">
  <Select
    label="Hold chat back"
    width="140px"
    value={String(c.delayTenths)}
    options={choicesOf(DELAYS)}
    onchange={(v) => settings.patch('chat', { delayTenths: Number(v) })}
  />
</Row>

<SectionHead title="Links" />
<Row setting="chat.linkPreviews" label="Previews" hint="Hover a link for its title and picture.">
  <Toggle label="Previews" checked={c.linkPreviews} onchange={(v) => settings.patch('chat', { linkPreviews: v })} />
</Row>
<Row setting="chat.linkThumbnails" label="Picture size">
  <Segmented
    label="Picture size"
    options={THUMBS}
    value={c.linkThumbnails}
    onchange={(v) => settings.patch('chat', { linkThumbnails: v })}
  />
</Row>
<Row setting="chat.linkCards" label="Cards under messages" hint="Also shows the preview below the message.">
  <Toggle label="Cards under messages" checked={c.linkCards} onchange={(v) => settings.patch('chat', { linkCards: v })} />
</Row>

<SectionHead title="GIFs and effects">Moving emotes and message effects.</SectionHead>
<Row setting="chat.gifs" label="GIFs">
  <Segmented label="GIFs" options={GIFS} value={c.gifs} onchange={(v) => settings.patch('chat', { gifs: v })} />
</Row>
<Row setting="chat.effects" label="Effects" hint="Animated messages, giant emotes and celebrations, as Twitch shows them.">
  <Toggle label="Effects" checked={c.effects} onchange={(v) => settings.patch('chat', { effects: v })} />
</Row>

<SectionHead title="Writing and reading">The message box and reading aids.</SectionHead>
<Row setting="chat.sendButton" label="Send button" hint="A button in the message box. Enter still sends.">
  <Toggle label="Send button" checked={c.sendButton} onchange={(v) => settings.patch('chat', { sendButton: v })} />
</Row>
<Row setting="chat.spellCheck" label="Spell check" hint="Underlines typos in the message box. Right-click one for fixes.">
  <Toggle label="Spell check" checked={c.spellCheck} onchange={(v) => settings.patch('chat', { spellCheck: v })} />
</Row>
<Row setting="chat.translateTo" label="Translate into" hint="For Translate and /translate. Each chat’s menu can translate everything.">
  <Select
    label="Translate into"
    width="200px"
    value={c.translateTo}
    options={LANGUAGES}
    onchange={(v) => settings.patch('chat', { translateTo: v })}
  />
</Row>

<SectionHead title="Moderation">Shortcuts for channels you moderate.</SectionHead>
<Row setting="chat.timeoutPresets" label="Timeout buttons" hint="Quick durations in chat and on user cards. Up to 8." stacked>
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
      <input
        class="field add"
        class:bad={!!presetError}
        aria-label="Add a timeout length"
        aria-invalid={!!presetError}
        placeholder="Add, e.g. 30m"
        bind:value={newPreset}
        oninput={() => (presetError = '')}
      />
    </form>
  {/if}
  {#if presetError}<span class="field-error" role="alert">{presetError}</span>{/if}
</Row>
<Row setting="chat.modButtons" label="Mod mode buttons" hint="In front of lines while mod mode is on. Up to 4 timeouts." stacked>
  <button class="pick" aria-pressed={mb.delete} onclick={() => flipMod({ delete: !mb.delete })}>Delete</button>
  {#each modTimes as secs (secs)}
    {@const on = mb.timeouts.includes(secs)}
    <button class="pick" aria-pressed={on} disabled={!on && mb.timeouts.length >= 4}
      title={!on && mb.timeouts.length >= 4 ? '4 timeouts max' : undefined}
      onclick={() => flipTime(secs)}>{formatDuration(secs)}</button
    >
  {/each}
  <button class="pick" aria-pressed={mb.warn} onclick={() => flipMod({ warn: !mb.warn })}>Warn</button>
  <button class="pick" aria-pressed={mb.ban} onclick={() => flipMod({ ban: !mb.ban })}>Ban</button>
  <!-- V107-R22b Lifts a ban or timeout; off by default. -->
  <button class="pick" aria-pressed={mb.unban} onclick={() => flipMod({ unban: !mb.unban })}>Unban</button>
  {#if mb.timeouts.length >= 4 && modTimes.some((t) => !mb.timeouts.includes(t))}
    <span class="slot-hint" title="Remove a timeout to pick another">4 timeouts max</span>
  {:else if mb.timeouts.length < 4}
    <!-- V107-R22 Any length, straight onto a button. -->
    <form
      onsubmit={(e) => {
        e.preventDefault();
        addButton();
      }}
    >
      <input
        class="field add"
        class:bad={!!buttonError}
        aria-label="Add a timeout button"
        aria-invalid={!!buttonError}
        placeholder="Timeout, e.g. 7m"
        bind:value={newButton}
        oninput={() => (buttonError = '')}
      />
    </form>
  {/if}
  {#if buttonError}<span class="field-error" role="alert">{buttonError}</span>{/if}
</Row>
<Row setting="chat.modButtons.order" label="Button order" hint="Drag them into the order they show in chat, or Alt+↑ and Alt+↓." stacked>
  <div class="order">
    <DragList key="modButtons" items={modOrder} id={buttonKey} label={modName} onmove={(order) => flipMod({ order })}>
      {#snippet row(b)}
        <div class="slot" class:off={!modOn(b)}>
          <span class="grip"><Icon name="grip" size={14} /></span>
          <Icon name={modIcon(b)} size={14} />
          {#if typeof b !== 'string' && editing === b.timeout}
            <!-- V107-R22 Another length for this button, typed. -->
            <span class="slot-name">
              Time out
              <input
                class="field len"
                class:bad={!!editError}
                aria-label="Length"
                aria-invalid={!!editError}
                bind:value={editText}
                use:focus
                oninput={() => (editError = '')}
                onkeydown={editKey}
                onblur={() => (editing = null)}
              />
              {#if editError}<span class="field-error" role="alert">{editError}</span>{/if}
            </span>
          {:else if typeof b !== 'string'}
            {@const secs = b.timeout}
            <span class="slot-name">
              Time out
              <button class="len-btn" data-tip="Change the length" aria-label="Change the length of {modName(b)}" onclick={() => startEdit(secs)}
                >{formatDuration(secs)}</button
              >
            </span>
          {:else}
            <span class="slot-name">{modName(b)}</span>
          {/if}
          {#if !modOn(b) && !canAddButton(mb, b)}<span class="slot-hint">4 timeouts max</span>{/if}
          {#if typeof b === 'string' || editing !== b.timeout}<button
            class="slot-act"
            disabled={!canAddButton(mb, b)}
            title={canAddButton(mb, b) ? undefined : '4 timeouts max'}
            onclick={() => flipMod(toggleButton(mb, b))}
            aria-label={`${modOn(b) ? 'Remove' : 'Add'} ${modName(b)}`}
            >{modOn(b) ? 'Remove' : 'Add'}</button
          >{/if}
        </div>
      {/snippet}
    </DragList>
  </div>
</Row>

<SectionHead title="Overlay">A chat over games: ⋯ on a chat → Overlay. It stays on top and can let clicks through.</SectionHead>
<Row setting="chat.overlay.background" label="Background" hint="0 % is fully see-through.">
  <input
    type="range"
    min="0"
    max="100"
    step="5"
    value={o.background}
    aria-label="Overlay background"
    oninput={(e) => settings.patch('chat', { overlay: { background: Number(e.currentTarget.value) } })}
  />
  <span class="num">{o.background} %</span>
</Row>
<Row setting="chat.overlay.scale" label="Text size" hint="Compared to chat.">
  <input
    type="range"
    min="50"
    max="300"
    step="10"
    value={o.scale}
    aria-label="Overlay text size"
    oninput={(e) => settings.patch('chat', { overlay: { scale: Number(e.currentTarget.value) } })}
  />
  <span class="num">{o.scale} %</span>
</Row>
<Row setting="chat.overlay.fadeAfter" label="Fade lines after" hint="Old lines fade out so the game stays clear.">
  <Segmented
    label="Fade lines after"
    options={FADES}
    value={o.fadeAfter}
    onchange={(v) => settings.patch('chat', { overlay: { fadeAfter: v } })}
  />
</Row>
<Row setting="chat.overlay.outline" label="Outline letters" hint="Readable over anything.">
  <Toggle
    label="Outline letters"
    checked={o.outline}
    onchange={(v) => settings.patch('chat', { overlay: { outline: v } })}
  />
</Row>
<Row setting="chat.overlay.badges" label="Badges">
  <Toggle label="Badges" checked={o.badges} onchange={(v) => settings.patch('chat', { overlay: { badges: v } })} />
</Row>

<ResetSection partition="chat" page="chat" />

<style>
  .order {
    width: 100%;
    max-width: 320px;
  }
  .slot {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 10px 0 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-weight: 600;
  }
  .slot.off {
    color: var(--dim);
  }
  .grip {
    display: grid;
    place-items: center;
    color: var(--dim);
  }
  .slot-name {
    flex: 1;
  }
  .slot-act {
    padding: 0 6px;
    background: none;
    border: 0;
    color: var(--dim);
    font: inherit;
    font-weight: 400;
    font-size: var(--text-hint);
    cursor: pointer;
  }
  .slot-act:hover:not(:disabled) {
    color: var(--text);
  }
  .slot-act:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .slot-hint {
    color: var(--dim);
    font-size: 0.74rem;
  }
  input[type='range'] {
    width: 160px;
    accent-color: var(--accent);
  }
  .num {
    min-width: 44px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
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
    position: relative;
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .chip button::before {
    content: "";
    position: absolute;
    inset: -3px;
  }
  .chip button:hover {
    color: var(--red);
  }
  .add {
    width: 130px;
    height: 28px;
  }
  /* V107-R22 Typed lengths: a refusal reads in red next to its field. */
  .field.bad,
  .field.bad:focus-visible {
    border-color: var(--red);
    box-shadow: 0 0 0 1px var(--red);
  }
  .field-error {
    white-space: nowrap;
    color: var(--red);
    font-size: 0.76rem;
    font-weight: 400;
  }
  .slot-name {
    display: flex;
    white-space: nowrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .len-btn {
    height: 22px;
    padding: 0 6px;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: inherit;
    font: inherit;
    text-decoration: underline dotted var(--dim);
    text-underline-offset: 3px;
    cursor: text;
  }
  .len-btn:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
    text-decoration: none;
  }
  .len {
    width: 72px;
    height: 22px;
    padding: 0 6px;
    font: inherit;
  }
  .pick {
    height: 28px;
    padding: 0 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-weight: 600;
  }
  .pick:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }
  .pick[aria-pressed='true'] {
    border-color: var(--border-strong);
    background: var(--surface-hover);
    box-shadow: 0 0 0 1px var(--border-strong);
    color: var(--text);
  }
  .pick:disabled {
    opacity: 0.5;
  }
</style>
