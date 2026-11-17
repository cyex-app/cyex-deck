<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import NumberField from '../../../components/NumberField.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { DeletedStyle } from '../../../ipc/bindings/DeletedStyle';
  import type { GifMode } from '../../../ipc/bindings/GifMode';
  import type { LinkThumbs } from '../../../ipc/bindings/LinkThumbs';
  import type { NameClick } from '../../../ipc/bindings/NameClick';
  import type { NameClicks } from '../../../ipc/bindings/NameClicks';
  import type { NameStyle } from '../../../ipc/bindings/NameStyle';
  import type { PauseKey } from '../../../ipc/bindings/PauseKey';
  import type { SuggestStyle } from '../../../ipc/bindings/SuggestStyle';
  import { formatNumber, formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { scrollbackHint } from '../scrollbackHint';
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

</script>

<SectionHead title="Chat" level={2} page="chat" />

<SectionHead title="History">Older messages loaded when you open a chat.</SectionHead>
<Row setting="chat.history" label="Recent history" hint="Shows the last messages, dimmed, when you open a chat.">
  <Toggle label="Recent history" checked={c.history} onchange={(v) => settings.patch('chat', { history: v })} />
</Row>
<!-- V110-M28 -->
<Row setting="chat.readWhileBanned" label="Keep reading when banned" hint="A chat you're banned in keeps showing messages, read without your account. You still can't send.">
  <Toggle label="Keep reading when banned" checked={c.readWhileBanned} onchange={(v) => settings.patch('chat', { readWhileBanned: v })} />
</Row>
<!-- V111-09 -->
<Row setting="chat.historyFromLogs" label="Fill gaps from logs.cyex.app" hint="When the history service has nothing, chat history and missed messages come from logs.cyex.app.">
  <Toggle label="Fill gaps from logs.cyex.app" checked={c.historyFromLogs} onchange={(v) => settings.patch('chat', { historyFromLogs: v })} />
</Row>
<Row setting="chat.historyLines" label="History to load" hint="Up to this many. The history service keeps 800 per channel.">
  <NumberField label="History to load" steppers width="110px" min={50} max={2000} step={50} disabled={!c.history} value={c.historyLines} onchange={(n) => settings.patch('chat', { historyLines: n })} />
</Row>
<Row setting="chat.scrollback" label="Messages kept" hint={scrollbackHint(c.scrollback)}>
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
<Row setting="chat.parts" label="Show parts" hint="Who parts (leaves) the chat, a line every 10 seconds. Only in chats under 1.000 viewers.">
  <Toggle label="Show parts" checked={c.parts} onchange={(v) => settings.patch('chat', { parts: v })} />
</Row>
<!-- V111-07 Every name shows; this folds a long line to 5 with Show more / Show less. -->
<Row setting="chat.minimizePresence" label="Minimize join/part messages" hint="A line shows 5 names, the rest behind Show more.">
  <Toggle label="Minimize join/part messages" checked={c.minimizePresence} onchange={(v) => settings.patch('chat', { minimizePresence: v })} />
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
<Row setting="chat.plainNames" label="Clickable names without @" hint="A name typed without @ opens the usercard when that person is in the chat.">
  <Toggle label="Clickable names without @" checked={c.plainNames} onchange={(v) => settings.patch('chat', { plainNames: v })} />
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
<Row setting="chat.screenReader" label="Screen reader mode" hint="Tab reaches the chat, and new messages are read out a few at a time.">
  <Toggle label="Screen reader mode" checked={c.screenReader} onchange={(v) => settings.patch('chat', { screenReader: v })} />
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
  input[type='range'] {
    width: 160px;
    accent-color: var(--accent-line);
  }
  .num {
    min-width: 44px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
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
</style>
