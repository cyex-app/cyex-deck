<!-- One chat line (1.8, 1.9): user message, notice or system line. Name colors are
     readable per theme (C8); deleted lines follow the C14 setting. Highlights (2.6),
     repeat counts (C2), near-duplicates (F6), the app it came from (C26), translations
     (C17), power-ups (1.42), link cards (C30) and inline whispers (1.26). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { Look } from '../../ipc/bindings/Look';
  import { formatDuration, formatNumber, formatTime } from '../../lib/format';
  import { modButtonsFor, type LineMod } from '../modview/lineMod';
  import { settings } from '../../lib/settings.svelte';
  import { streamer } from '../../lib/streamer.svelte';
  import { looks } from '../emotes/looks.svelte';
  import Badges from './Badges.svelte';
  import Body from './Body.svelte';
  import { CLIENTS, COSMIC, effectOf, highlightStyle } from './highlight';
  import LinkCard from './LinkCard.svelte';
  import { shownName } from './names';
  import { uploadLine } from './upload';

  interface Props {
    m: ChatMessage;
    channel: string;
    /** Show the reply button on hover. */
    canReply?: boolean;
    /** In the whispers view: no "Whisper" label over each line. */
    inWhispers?: boolean;
    /** V101-01 The tinted one of alternating lines. */
    stripe?: boolean;
    /** 1.57 Set only where this account moderates the chat. */
    mod?: LineMod | null;
    /** V1051-03 The cosmetics preview: draw this look instead of the sender's. */
    look?: Look | null;
  }

  let { m, channel, canReply = false, inWhispers = false, stripe = false, mod = null, look }: Props = $props();

  /** The hover buttons exist only under the pointer: a long history has none to carry. */
  let hot = $state(false);

  const c = $derived(settings.v.chat);
  const has = (flag: ChatMessage['flags'][number]) => m.flags.includes(flag);
  const sender = $derived(m.sender);
  const name = $derived(sender ? shownName(sender, c.names, c.users[sender.id]) : null);
  const action = $derived(has('action'));
  const deleted = $derived(has('deleted'));
  const notice = $derived(m.notice);
  /** V105-11 Our own upload line: both links are clickable. */
  const upLine = $derived(notice && !sender && notice.kind === 'info' ? uploadLine(notice.text) : null);
  const time = $derived(c.timestamps.show ? formatTime(new Date(m.ts), c.timestamps) : '');
  const colorVars = $derived(sender ? `--cd:${sender.color.dark};--cl:${sender.color.light}` : '');
  /** 1.37 The sender's 7TV paint, as a class, and its name for a tip (V101-36). */
  const paintId = $derived(sender && c.paints.seventv ? (look !== undefined ? (look?.paint ?? null) : (looks.look(sender.id)?.paint ?? null)) : null);
  const paint = $derived(looks.paintClass(paintId));
  const paintName = $derived(paint ? looks.paintName(paintId) : null);
  const recipients = $derived(notice?.recipients ?? []);
  const hl = $derived(highlightStyle(m));
  const client = $derived(c.showClient && m.client ? CLIENTS[m.client] : null);
  const effect = $derived(c.effects ? effectOf(m) : null);
  /** 1.26 A whisper shown in a chat: who it's with, for the label and a click. */
  const whisperWith = $derived(has('whisper') && !inWhispers ? (m.whisperTo ?? sender?.login ?? null) : null);
  /** V101-34 Any chat message can be pinned, a mod's or the broadcaster's too. */
  const canPin = $derived(!!mod && !!sender && !notice && !deleted && !has('history'));
  /** V101-33 Mod mode's buttons, for people this account can act on. */
  const modButtons = $derived(!notice && !deleted && modButtonsFor(mod, sender, channel));
  /** C30 The first link, for a card under the message. */
  const firstLink = $derived(
    c.linkCards && c.linkPreviews && !streamer.hides('hideLinkPreviews')
      ? m.body.find((f) => f.t === 'link' && !f.target)
      : undefined,
  );
</script>

<!-- The pointer only decides whether the hover buttons are drawn. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="line kind-{notice?.kind ?? 'chat'} {hl.cls}"
  class:stripe
  class:history={has('history')}
  class:similar={has('similar')}
  class:effect={!!effect}
  class:del-strike={deleted && c.deleted === 'strike'}
  class:del-faded={deleted && c.deleted === 'faded'}
  class:del-hidden={deleted && c.deleted === 'hidden'}
  class:system={notice && !sender}
  class:whisper={!!whisperWith}
  data-id={m.id}
  style="{colorVars};{hl.vars}"
  onpointerenter={() => (hot = true)}
  onpointerleave={() => (hot = false)}
>
  {#if effect}<div class="fx fx-{effect}" aria-hidden="true">
      {#if effect === 'cosmic-abyss'}<video src={COSMIC} autoplay loop muted playsinline></video>{/if}
    </div>{/if}
  {#if whisperWith}
    <div class="context" data-whisper={whisperWith}>
      <Icon name="whisper" size={12} />
      <span>{m.whisperTo ? `Whisper to ${m.whisperTo}` : 'Whisper'}</span>
    </div>
  {:else if m.reply}
    <div class="context" data-jump={m.reply.parentId}>
      <Icon name="reply" size={12} />
      <span>Replying to <b>@{m.reply.parentDisplay}</b>: {m.reply.parentBody}</span>
    </div>
  {/if}
  <!-- V105-06 A first message can also be a reply: both show. -->
  {#if has('firstMessage')}
    <div class="context">First message</div>
  {/if}
  {#if m.reward}
    <div class="context reward">
      <Icon name="points" size={12} />
      {#if m.reward.title}Redeemed <b>{m.reward.title}</b>{#if m.reward.cost} · {formatNumber(m.reward.cost)}{/if}{:else}Redeemed a reward{/if}
    </div>
  {/if}
  {#if notice}
    <div class="notice-text">
      {#if !sender && time}<span class="ts">{time}</span>{/if}{#if upLine}Uploaded <a class="up" href={upLine.link} data-upload={upLine.link}>{upLine.link}</a>{#if upLine.deletion}
        · delete: <a class="up" href={upLine.deletion} data-upload={upLine.deletion}>{upLine.deletion}</a>{/if}{:else}{notice.text}{/if}
      {#if recipients.length}<span class="recipients"> To {recipients.join(', ')}.</span>{/if}
    </div>
  {/if}
  {#if m.plugin && !sender && !notice}
    <!-- S7 A plugin's line: always its mark, never a name. -->
    <div class="msg">
      {#if time}<span class="ts">{time}</span>{/if}
      <span class="from-plugin" data-tip="From {m.plugin.name}" data-hint="A plugin"><Icon name="puzzle" size={12} /></span>
      <span class="body"><Body fragments={m.body} {channel} /></span>
    </div>
  {/if}
  {#if sender && name && (!notice || m.body.length)}
    <div class="msg">
      {#if modButtons && mod}
        <span class="mods" data-login={sender.login}>
          {#each mod.slots as s (s.key)}
            {#if s.kind === 'timeout'}
              <button
                class="mb"
                class:len={s.short}
                aria-label="Time out {formatDuration(s.secs)}"
                data-mod="timeout"
                data-secs={s.secs}
                data-tip="Time out {formatDuration(s.secs)}"
                >{#if s.short}{formatDuration(s.secs).replace(' ', '')}{:else}<Icon name="clock" size={13} />{/if}</button
              >
            {:else if s.kind === 'delete'}
              <button class="mb" aria-label="Delete" data-mod="delete" data-tip="Delete"><Icon name="trash" size={13} /></button>
            {:else if s.kind === 'warn'}
              <button class="mb" aria-label="Warn" data-mod="warn" data-tip="Warn"><Icon name="alert" size={13} /></button>
            {:else}
              <button class="mb ban" aria-label="Ban" data-mod="ban" data-tip="Ban"><Icon name="gavel" size={13} /></button>
            {/if}
          {/each}
        </span>
      {/if}
      {#if time}<span class="ts">{time}</span>{/if}
      {#if client}<span class="client" data-tip={client.label}><Icon name={client.icon} size={12} /></span>{/if}
      {#if m.source}<Badges badges={m.source.badges} {channel} />{/if}
      <Badges badges={sender.badges} {channel} userId={sender.id} {look} />
      <span
        class="name {paint ?? ''}"
        class:painted={!!paint}
        role="link"
        tabindex="-1"
        data-user={sender.login}
        data-tip={paintName}
        data-hint={paintName ? '7TV paint' : null}>{name.name}</span
      >{#if name.extra}<span class="extra">{` (${name.extra})`}</span>{/if}{#if !action}<span class="colon">{': '}</span>{:else}{' '}{/if}<span
        class="body"
        class:action><Body fragments={m.body} {channel} giant={has('gigantified')} /></span
      >{#if m.repeat}<span class="repeat" data-tip="Said {m.repeat} times in a row">×{m.repeat}</span>{/if}
    </div>
  {/if}
  {#if m.translation}
    <div class="translation" data-tip="Translated from {m.translation.from || 'another language'}">
      <Icon name="languages" size={12} />{m.translation.text}
    </div>
  {/if}
  {#if firstLink?.t === 'link'}<LinkCard url={firstLink.url} />{/if}
  {#if hot && canReply && sender && !notice}
    <button class="reply" aria-label="Reply" data-reply={m.id} data-tip="Reply">
      <Icon name="reply" size={14} />
    </button>
  {/if}
  {#if hot && canPin}
    <button class="reply pin" class:alone={!canReply} aria-label="Pin" data-pin={m.id} data-tip="Pin">
      <Icon name="pin" size={14} />
    </button>
  {/if}
</div>

<style>
  .line {
    position: relative;
    padding: 2px 10px;
    /* Text rows are normal height; a row with an emote grows to fit it (the emote sits on
       the text's middle, margins off). */
    font: var(--chat-size) / 1.45 var(--chat-font);
    overflow-wrap: anywhere;
    user-select: text;
    content-visibility: auto;
    contain-intrinsic-size: auto calc(1.45 * var(--chat-size) + 4px);
  }
  .line :global(.emote:not(.stack > .emote)),
  .line :global(.stack) {
    margin: 0;
    vertical-align: middle;
  }
  /* V101-01 Alternating lines (2.1): the stripe stays with its line (stripes.ts). */
  .stripe {
    background: var(--row-alt);
  }
  .line:hover {
    background: var(--row-hover);
  }
  .ts {
    margin-right: 6px;
    color: var(--dim);
    font-size: 0.85em;
    font-variant-numeric: tabular-nums;
  }
  .name {
    color: var(--cd);
    font-weight: 700;
    cursor: pointer;
  }
  :global(:root[data-theme='light']) .name,
  :global(:root[data-theme='light']) .action {
    color: var(--cl);
  }
  .name:hover {
    text-decoration: underline;
  }
  .extra {
    color: var(--dim);
    font-size: 0.9em;
  }
  .from-plugin {
    display: inline-flex;
    margin-right: 4px;
    color: var(--dim);
    vertical-align: -1px;
  }
  .action {
    color: var(--cd);
  }
  .context {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--dim);
    font-size: 0.85em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .context span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .context[data-jump],
  .context[data-whisper] {
    cursor: pointer;
  }
  .context[data-jump]:hover,
  .context[data-whisper]:hover {
    color: var(--muted);
  }
  .whisper {
    border-left: 3px solid var(--cyan);
    background: color-mix(in srgb, var(--cyan) 7%, transparent);
  }
  .reward {
    color: var(--twitch);
  }
  .notice-text {
    color: var(--muted);
  }
  .up {
    color: var(--accent-text);
    text-decoration: none;
    word-break: break-all;
  }
  .up:hover {
    text-decoration: underline;
  }
  .recipients {
    color: var(--dim);
  }
  .system .notice-text {
    color: var(--dim);
  }
  .kind-sub,
  .kind-gift,
  .kind-raid,
  .kind-announcement,
  .kind-bits,
  .kind-milestone,
  .kind-redemption {
    border-left: 3px solid var(--twitch);
    background: color-mix(in srgb, var(--twitch) 7%, transparent);
  }
  .kind-sub .notice-text,
  .kind-gift .notice-text,
  .kind-raid .notice-text,
  .kind-announcement .notice-text {
    color: var(--text);
    font-weight: 600;
  }
  .kind-error .notice-text {
    color: var(--red);
  }
  /* 2.6 Highlights: a rule's own color, or the kind's. */
  .hl {
    background: color-mix(in srgb, var(--hl) 18%, transparent);
    box-shadow: inset 3px 0 var(--hl);
  }
  .hl-mention {
    --hl: var(--accent);
  }
  .hl-rule {
    --hl: var(--red);
  }
  .hl-firstMessage {
    --hl: var(--green);
  }
  .hl-redeemed,
  .hl-sub,
  .hl-announcement {
    --hl: var(--twitch);
  }
  .hl-watchStreak {
    --hl: var(--cyan);
  }
  .hl-own {
    --hl: var(--accent-hover);
  }
  /* F6 */
  .similar {
    opacity: 0.45;
  }
  /* C2 */
  .repeat {
    margin-left: 6px;
    padding: 0 4px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--red) 18%, transparent);
    color: var(--red);
    font: 700 0.8em var(--font-mono);
  }
  /* C26 */
  .client {
    display: inline-flex;
    margin-right: 4px;
    vertical-align: -1px;
    color: var(--dim);
  }
  /* C17 */
  .translation {
    display: flex;
    gap: 5px;
    align-items: baseline;
    color: var(--muted);
    font-size: 0.92em;
  }
  .translation :global(svg) {
    flex: none;
    color: var(--dim);
  }
  /* 1.42 Message effects, after twitch.tv's. */
  .effect {
    margin: 4px 6px;
    padding: 6px 10px;
    border-radius: var(--radius);
    isolation: isolate;
    overflow: hidden;
  }
  .fx {
    position: absolute;
    inset: 0;
    z-index: -1;
    pointer-events: none;
  }
  .fx-simmer {
    background: linear-gradient(90deg, #3866dd, #ff4c5b);
  }
  .fx-simmer::after,
  .fx-cosmic-abyss::after,
  .fx-rainbow-eclipse::after {
    content: '';
    position: absolute;
    inset: 4px;
    border-radius: var(--radius);
    background: var(--bg);
    opacity: 0.88;
  }
  .fx-rainbow-eclipse {
    filter: blur(4px);
  }
  .fx-rainbow-eclipse::before {
    content: '';
    position: absolute;
    left: 50%;
    top: 50%;
    width: 200%;
    aspect-ratio: 1;
    translate: -50% -50%;
    background: conic-gradient(#b23ff8, #3cc890, #38a7ca, #b23ff8);
    animation: spin 4s linear infinite;
  }
  .fx-cosmic-abyss video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  :global(.still) .fx-rainbow-eclipse::before {
    animation-play-state: paused;
  }
  @keyframes spin {
    to {
      rotate: 1turn;
    }
  }
  .history {
    opacity: 0.55;
  }
  .del-strike .body {
    color: var(--dim);
    text-decoration: line-through;
  }
  .del-faded {
    opacity: 0.4;
  }
  .del-hidden {
    display: none;
  }
  .reply {
    position: absolute;
    top: 2px;
    right: 6px;
    display: none;
    place-items: center;
    width: 24px;
    height: 22px;
    padding: 0;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
  }
  .line:hover .reply {
    display: grid;
  }
  .reply:hover {
    color: var(--text);
  }
  /* V101-34 Left of Reply, or in its place. */
  .pin {
    right: 34px;
  }
  .pin.alone {
    right: 6px;
  }
  /* V101-33 Mod mode's buttons, in front of the line. */
  .mods {
    display: inline-flex;
    gap: 1px;
    margin-right: 5px;
    vertical-align: -3px;
  }
  .mb {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
  }
  .mb:hover {
    background: var(--row-hover);
    color: var(--text);
  }
  .mb.ban:hover {
    color: var(--red);
  }
  /* M10 Several timeouts: their lengths instead of the clock. */
  .mb.len {
    width: auto;
    padding: 0 3px;
    font: 600 0.7rem var(--font-mono);
  }
</style>
