<script lang="ts">
  import { enterKey } from '../../../lib/keys';
  import SectionHead from '../SectionHead.svelte';
  import DragList from '../../../components/DragList.svelte';
  import Icon from '../../../components/Icon.svelte';
  import type { IconName } from '../../../components/icons';
  import NumberField from '../../../components/NumberField.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { DeletedStyle } from '../../../ipc/bindings/DeletedStyle';
  import type { GifMode } from '../../../ipc/bindings/GifMode';
  import type { LinkThumbs } from '../../../ipc/bindings/LinkThumbs';
  import type { ModButton } from '../../../ipc/bindings/ModButton';
  import type { ModButtons } from '../../../ipc/bindings/ModButtons';
  import type { NameClick } from '../../../ipc/bindings/NameClick';
  import type { NameClicks } from '../../../ipc/bindings/NameClicks';
  import type { NameStyle } from '../../../ipc/bindings/NameStyle';
  import type { PauseKey } from '../../../ipc/bindings/PauseKey';
  import type { SuggestStyle } from '../../../ipc/bindings/SuggestStyle';
  import { formatDuration, formatNumber, formatSecs, formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { addTimeout, buttonKey, buttonOn, canAddButton, fullOrder, retimeButton, toggleButton } from '../../modview/lineMod';
  import { timeoutSecs } from '../../modview/timeoutLength';
  import ModePresets from './ModePresets.svelte';
  import { adSecs, followMinutes, slowSecs } from '../../modview/modePresets';
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
  const CLOCKS: { value: 'h24' | 'h12'; label: string }[] = [
    { value: 'h24', label: '24h' },
    { value: 'h12', label: '12h' },
  ];
  const DELETED: { value: DeletedStyle; label: string }[] = [
    { value: 'strike', label: 'Strike' },
    { value: 'faded', label: 'Faded' },
    { value: 'hidden', label: 'Hidden' },
  ];
  // V108-C20 Two choices over the one stored style (user, 2026/11/01): which name leads, and the other in brackets.
  const FIRST: { value: 'display' | 'username'; label: string }[] = [
    { value: 'display', label: 'Display name' },
    { value: 'username', label: 'Username' },
  ];
  const nameStyle = (first: 'display' | 'username', other: boolean): NameStyle =>
    first === 'display' ? (other ? 'displayLogin' : 'display') : other ? 'usernameDisplay' : 'username';
  // V110-M20 What a click on a name can do. Time out and Ban only work where you moderate; elsewhere the usercard opens.
  const NAME_ACTIONS: { value: NameClick; label: string }[] = [
    { value: 'usercard', label: 'Open usercard' },
    { value: 'reply', label: 'Reply' },
    { value: 'mention', label: 'Mention in the box' },
    { value: 'copy', label: 'Copy name' },
    { value: 'timeout', label: 'Time out (mods)' },
    { value: 'ban', label: 'Ban, asks first (mods)' },
    { value: 'logs', label: 'Open logs' },
    { value: 'none', label: 'Nothing' },
  ];
  const NAME_CLICKS: { key: keyof NameClicks; label: string }[] = [
    { key: 'plain', label: 'Click on a name' },
    { key: 'middle', label: 'Middle-click on a name' },
    { key: 'ctrl', label: 'Ctrl+click on a name' },
    { key: 'shift', label: 'Shift+click on a name' },
  ];
  const PAUSE_KEYS: { value: PauseKey; label: string }[] = [
    { value: 'none', label: 'None' },
    { value: 'ctrl', label: 'Ctrl' },
    { value: 'alt', label: 'Alt' },
    { value: 'shift', label: 'Shift' },
  ];
  const SUGGEST: { value: SuggestStyle; label: string }[] = [
    { value: 'list', label: 'List' },
    { value: 'row', label: 'Row' },
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
  // V107-B "Every cheer" is 1.
  const CHEER_ASK = [1, 100, 500, 1000, 5000, 10000].map((n) => ({ value: n, label: n === 1 ? 'Every cheer' : `${formatNumber(n)} Bits` }));
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

  /** V108-N20 Saved phrases: the box adds, a field edits (emptied removes), × removes. */
  const MAX_REPLIES = 20;
  let newReply = $state('');
  const setReplies = (list: string[]) => void settings.patch('chat', { quickReplies: list });
  function addReply() {
    const t = newReply.trim();
    if (!t || c.quickReplies.includes(t)) return void (newReply = '');
    setReplies([...c.quickReplies, t]);
    newReply = '';
  }
  function editReply(i: number, text: string) {
    const t = text.trim();
    // Emptied, or now the same as another phrase: this row goes (merged into the other).
    const drop = !t || c.quickReplies.some((r, j) => j !== i && r === t);
    setReplies(drop ? c.quickReplies.filter((_, j) => j !== i) : c.quickReplies.map((r, j) => (j === i ? t : r)));
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
  /** Clicking away keeps a length that works, as Enter does; anything else is dropped. */
  function blurEdit() {
    if (editing === null) return;
    const t = timeoutSecs(editText);
    if ('error' in t || typeof retimeButton(mb, editing, t.secs, modTimes) === 'string') editing = null;
    else saveEdit();
  }
  function editKey(e: KeyboardEvent) {
    if (enterKey(e)) {
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
<!-- V110-M28 -->
<Row setting="chat.readWhileBanned" label="Keep reading when banned" hint="A chat you're banned in keeps showing messages, read without your account. You still can't send.">
  <Toggle label="Keep reading when banned" checked={c.readWhileBanned} onchange={(v) => settings.patch('chat', { readWhileBanned: v })} />
</Row>
<Row setting="chat.historyLines" label="History to load" hint="Up to this many. The history service keeps 800 per channel.">
  <NumberField label="History to load" steppers width="110px" min={50} max={2000} step={50} disabled={!c.history} value={c.historyLines} onchange={(n) => settings.patch('chat', { historyLines: n })} />
</Row>
<Row setting="chat.scrollback" label="Messages kept" hint="Per chat. Older ones drop off the top; more uses more memory.">
  <NumberField label="Messages kept" steppers width="110px" min={100} max={8000} step={100} value={c.scrollback} onchange={(n) => settings.patch('chat', { scrollback: n })} />
</Row>

<SectionHead title="Messages" />
<Row setting="chat.names" label="Name first" hint="Usercards always show both.">
  <Segmented
    label="Name first"
    options={FIRST}
    value={c.names.startsWith('username') ? 'username' : 'display'}
    onchange={(v) => settings.patch('chat', { names: nameStyle(v, c.names === 'displayLogin' || c.names === 'usernameDisplay') })} />
</Row>
<Row label="Other name in brackets" hint="Only when it differs by more than capitals.">
  <Toggle
    label="Other name in brackets"
    checked={c.names === 'displayLogin' || c.names === 'usernameDisplay'}
    onchange={(v) => settings.patch('chat', { names: nameStyle(c.names.startsWith('username') ? 'username' : 'display', v) })} />
</Row>
{#each NAME_CLICKS as n, i (n.key)}
  <Row
    setting={`chat.nameClicks.${n.key}`}
    label={n.label}
    hint={i === 0 ? 'Right-click still opens the menu, and Shift+right-click on a line replies.' : undefined}
  >
    <Select
      label={n.label}
      width="220px"
      value={c.nameClicks[n.key]}
      options={n.key === 'plain' ? NAME_ACTIONS.filter((a) => a.value !== 'none') : NAME_ACTIONS}
      onchange={(v) => settings.patch('chat', { nameClicks: { ...c.nameClicks, [n.key]: v as NameClick } })}
    />
  </Row>
{/each}
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
<!-- C37 V108-C02 Each kind on its own; both only in chats under 1.000 viewers. -->
<Row setting="chat.joins" label="Show joins" hint="Who opens the chat, a line every 10 seconds. Only in chats under 1.000 viewers.">
  <Toggle label="Show joins" checked={c.joins} onchange={(v) => settings.patch('chat', { joins: v })} />
</Row>
<Row setting="chat.parts" label="Show leaves" hint="Who leaves the chat, a line every 10 seconds. Only in chats under 1.000 viewers.">
  <Toggle label="Show leaves" checked={c.parts} onchange={(v) => settings.patch('chat', { parts: v })} />
</Row>
<Row setting="chat.emoteChanges" label="Show emote changes" hint="A line when a channel adds, removes or renames a 7TV, BTTV or FFZ emote.">
  <Toggle label="Show emote changes" checked={c.emoteChanges} onchange={(v) => settings.patch('chat', { emoteChanges: v })} />
</Row>
<Row setting="chat.rateMeter" label="Show messages per second" hint="How busy the chat is, in the header.">
  <Toggle label="Show messages per second" checked={c.rateMeter} onchange={(v) => settings.patch('chat', { rateMeter: v })} />
</Row>
<Row setting="chat.hypeTrainBar" label="Show hype trains" hint="A bar over the chat with the level, progress and time left while a hype train runs.">
  <Toggle label="Show hype trains" checked={c.hypeTrainBar} onchange={(v) => settings.patch('chat', { hypeTrainBar: v })} />
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
<Row setting="chat.timestamps.clock" label="Time format">
  <Segmented
    label="Time format"
    options={CLOCKS}
    value={c.timestamps.clock}
    onchange={(v) => settings.patch('chat', { timestamps: { clock: v } })}
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
<Row setting="chat.scrollMarks" label="Scrollbar marks" hint="Small ticks on the scrollbar for highlights, mentions and events. Click one to jump there.">
  <Toggle label="Scrollbar marks" checked={c.scrollMarks} onchange={(v) => settings.patch('chat', { scrollMarks: v })} />
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
<Row
  setting="chat.linkPreviews"
  label="Previews"
  hint="Hover a link for its title and picture. Links from trusted sites load as they scroll into view, so the preview opens at once."
>
  <Toggle label="Previews" checked={c.linkPreviews} onchange={(v) => settings.patch('chat', { linkPreviews: v })} />
</Row>
<Row setting="chat.linkThumbnails" label="Preview size" hint="How big the picture is in the hover preview.">
  <Segmented
    label="Preview size"
    options={THUMBS}
    value={c.linkThumbnails}
    onchange={(v) => settings.patch('chat', { linkThumbnails: v })}
  />
</Row>
<Row
  setting="chat.linkCards"
  label="Cards under messages"
  hint="Shows a card below the message for links from trusted sites. Needs Previews."
>
  <Toggle label="Cards under messages" disabled={!c.linkPreviews} checked={c.linkCards} onchange={(v) => settings.patch('chat', { linkCards: v })} />
</Row>

<SectionHead title="GIFs and effects">Moving emotes and message effects.</SectionHead>
<Row setting="chat.gifs" label="GIFs">
  <Segmented label="GIFs" options={GIFS} value={c.gifs} onchange={(v) => settings.patch('chat', { gifs: v })} />
</Row>
<Row setting="chat.effects" label="Effects" hint="Animated messages, giant emotes and celebrations, as Twitch shows them.">
  <Toggle label="Effects" checked={c.effects} onchange={(v) => settings.patch('chat', { effects: v })} />
</Row>
<Row setting="chat.cheerConfirmBits" label="Ask before cheering" hint="Cheers from this much up ask first. Smaller ones go out on Enter.">
  <Select
    label="Ask before cheering"
    width="140px"
    value={String(c.cheerConfirmBits)}
    options={choicesOf(CHEER_ASK)}
    onchange={(v) => settings.patch('chat', { cheerConfirmBits: Number(v) })}
  />
</Row>

<SectionHead title="Writing and reading">The message box and reading aids.</SectionHead>
<Row setting="chat.sendButton" label="Send button" hint="A button in the message box. Enter still sends.">
  <Toggle label="Send button" checked={c.sendButton} onchange={(v) => settings.patch('chat', { sendButton: v })} />
</Row>
<Row setting="chat.suggestStyle" label="Suggestions" hint="Emote suggestions after a colon. A row sits above the box; Tab and the arrow keys move, Enter picks.">
  <Segmented label="Suggestions" options={SUGGEST} value={c.suggestStyle} onchange={(v) => settings.patch('chat', { suggestStyle: v })} />
</Row>
<Row setting="chat.quickReplies" label="Quick replies" hint="Up to 20 phrases, in a menu next to the message box. A click puts one in the box; it never sends." stacked>
  <div class="replies">
    {#each c.quickReplies as reply, i (i)}
      <div class="reply">
        <input
          class="field"
          maxlength="500"
          aria-label="Quick reply {i + 1}"
          value={reply}
          onchange={(e) => editReply(i, e.currentTarget.value)}
        />
        <button aria-label="Remove quick reply {i + 1}" onclick={() => editReply(i, '')}>×</button>
      </div>
    {/each}
    {#if c.quickReplies.length < MAX_REPLIES}
      <form
        onsubmit={(e) => {
          e.preventDefault();
          addReply();
        }}
      >
        <input class="field" maxlength="500" aria-label="Add a quick reply" placeholder="Add a phrase" bind:value={newReply} />
      </form>
    {/if}
  </div>
</Row>
<Row setting="chat.spellCheck" label="Spell check" hint="Underlines typos in the message box. Right-click one for fixes.">
  <Toggle label="Spell check" checked={c.spellCheck} onchange={(v) => settings.patch('chat', { spellCheck: v })} />
</Row>
<Row setting="chat.mentionComma" label="Mention users with a comma" hint="Tab at the start of a message adds a comma: “@name, ”.">
  <Toggle label="Mention users with a comma" checked={c.mentionComma} onchange={(v) => settings.patch('chat', { mentionComma: v })} />
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
<!-- V110-M23 Lengths for the mode menus and the ad picker in the stream manager. -->
<ModePresets
  setting="chat.slowPresets"
  label="Slow mode lengths"
  hint="Choices when you set slow mode. 3s to 2m."
  values={c.slowPresets}
  show={formatSecs}
  parse={slowSecs}
  placeholder="Add, e.g. 45s"
  onsave={(list) => void settings.patch('chat', { slowPresets: list })}
/>
<ModePresets
  setting="chat.followPresets"
  label="Followers-only ages"
  hint="Choices for how long someone must have followed. Any follower is always there. Up to 3 months."
  values={c.followPresets}
  show={(m) => formatSecs(m * 60)}
  parse={followMinutes}
  placeholder="Add, e.g. 2d"
  onsave={(list) => void settings.patch('chat', { followPresets: list })}
/>
<ModePresets
  setting="chat.adPresets"
  label="Ad lengths"
  hint="Choices for a commercial: 30s to 3m in steps of 30s."
  values={c.adPresets}
  show={formatSecs}
  parse={adSecs}
  placeholder="Add, e.g. 150s"
  max={6}
  onsave={(list) => void settings.patch('chat', { adPresets: list })}
/>
<Row setting="chat.adLength" label="Default ad length" hint="Picked when the stream manager opens.">
  <Select
    label="Default ad length"
    width="110px"
    value={String(c.adLength)}
    options={c.adPresets.map((v) => ({ value: String(v), label: formatSecs(v) }))}
    onchange={(v) => settings.patch('chat', { adLength: Number(v) })}
  />
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
                onblur={blurEdit}
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
    accent-color: var(--accent-line);
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
  .replies {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    max-width: 420px;
  }
  .reply {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .reply .field,
  .replies form .field {
    flex: 1;
    width: 100%;
    min-width: 0;
    height: 28px;
  }
  /* The add field ends where the rows' fields end (gap 4 + the 20 px ×). */
  .replies form {
    padding-right: 24px;
  }
  .reply button {
    position: relative;
    flex: none;
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .reply button::before {
    content: "";
    position: absolute;
    inset: -4px;
  }
  .reply button:hover {
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
  /* V110-F35 Picked: accent-dim fill and accent text (cyex-app), not a faint border. */
  .pick[aria-pressed='true'] {
    border-color: var(--accent-border);
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .pick[aria-pressed='true']:hover:not(:disabled) {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .pick:disabled {
    opacity: 0.5;
  }
</style>
