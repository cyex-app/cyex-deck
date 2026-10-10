<!-- One chat line (1.8, 1.9): user message, notice or system line. Name colors are
     readable per theme (C8); deleted lines follow the C14 setting. Highlights (2.6),
     repeat counts (C2), near-duplicates (F6), the app it came from (C26), translations
     (C17), power-ups (1.42), link cards (C30) and inline whispers (1.26). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import { formatNumber, formatTime } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { streamer } from '../../lib/streamer.svelte';
  import { looks } from '../emotes/looks.svelte';
  import Badges from './Badges.svelte';
  import Body from './Body.svelte';
  import { CLIENTS, COSMIC, effectOf, highlightStyle } from './highlight';
  import LinkCard from './LinkCard.svelte';
  import { shownName } from './names';

  interface Props {
    m: ChatMessage;
    channel: string;
    /** Show the reply button on hover. */
    canReply?: boolean;
    /** In the whispers view: no "Whisper" label over each line. */
    inWhispers?: boolean;
  }

  let { m, channel, canReply = false, inWhispers = false }: Props = $props();

  const c = $derived(settings.v.chat);
  const has = (flag: ChatMessage['flags'][number]) => m.flags.includes(flag);
  const sender = $derived(m.sender);
  const name = $derived(sender ? shownName(sender, c.names, c.users[sender.id]) : null);
  const action = $derived(has('action'));
  const deleted = $derived(has('deleted'));
  const notice = $derived(m.notice);
  const time = $derived(c.timestamps.show ? formatTime(new Date(m.ts), c.timestamps) : '');
  const colorVars = $derived(sender ? `--cd:${sender.color.dark};--cl:${sender.color.light}` : '');
  /** 1.37 The sender's 7TV paint, as a class. */
  const paint = $derived(sender && c.paints.seventv ? looks.paintClass(looks.look(sender.id)?.paint ?? null) : null);
  const recipients = $derived(notice?.recipients ?? []);
  const hl = $derived(highlightStyle(m));
  const client = $derived(c.showClient && m.client ? CLIENTS[m.client] : null);
  const effect = $derived(c.effects ? effectOf(m) : null);
  /** 1.26 A whisper shown in a chat: who it's with, for the label and a click. */
  const whisperWith = $derived(has('whisper') && !inWhispers ? (m.whisperTo ?? sender?.login ?? null) : null);
  /** C30 The first link, for a card under the message. */
  const firstLink = $derived(
    c.linkCards && c.linkPreviews && !streamer.hides('hideLinkPreviews')
      ? m.body.find((f) => f.t === 'link' && !f.target)
      : undefined,
  );
</script>

<div
  class="line kind-{notice?.kind ?? 'chat'} {hl.cls}"
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
  {:else if has('firstMessage')}
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
      {#if !sender && time}<span class="ts">{time}</span>{/if}{notice.text}
      {#if recipients.length}<span class="recipients"> To {recipients.join(', ')}.</span>{/if}
    </div>
  {/if}
  {#if sender && name && (!notice || m.body.length)}
    <div class="msg">
      {#if time}<span class="ts">{time}</span>{/if}
      {#if client}<span class="client" data-tip={client.label}><Icon name={client.icon} size={12} /></span>{/if}
      {#if m.source}<Badges badges={m.source.badges} {channel} />{/if}
      <Badges badges={sender.badges} {channel} userId={sender.id} />
      <span class="name {paint ?? ''}" class:painted={!!paint} role="link" tabindex="-1" data-user={sender.login}
        >{name.name}</span
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
  {#if canReply && sender && !notice}
    <button class="reply" aria-label="Reply" data-reply={m.id} data-tip="Reply">
      <Icon name="reply" size={14} />
    </button>
  {/if}
</div>

<style>
  .line {
    position: relative;
    padding: 3px 10px;
    font: var(--chat-size) / 1.55 var(--chat-font);
    overflow-wrap: anywhere;
    user-select: text;
    content-visibility: auto;
    contain-intrinsic-size: auto 24px;
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
</style>
