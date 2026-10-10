<!-- One chat line (1.8, 1.9): user message, notice or system line. Name colors are
     readable per theme (C8); deleted lines follow the C14 setting. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import { formatNumber, formatTime } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { looks } from '../emotes/looks.svelte';
  import Badges from './Badges.svelte';
  import Body from './Body.svelte';
  import { shownName } from './names';

  interface Props {
    m: ChatMessage;
    channel: string;
    /** Show the reply button on hover. */
    canReply?: boolean;
  }

  let { m, channel, canReply = false }: Props = $props();

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
</script>

<div
  class="line kind-{notice?.kind ?? 'chat'}"
  class:mention={has('mention')}
  class:history={has('history')}
  class:highlighted={has('highlighted')}
  class:first={has('firstMessage')}
  class:del-strike={deleted && c.deleted === 'strike'}
  class:del-faded={deleted && c.deleted === 'faded'}
  class:del-hidden={deleted && c.deleted === 'hidden'}
  class:system={notice && !sender}
  data-id={m.id}
  style={colorVars}
>
  {#if m.reply}
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
      {#if m.source}<Badges badges={m.source.badges} {channel} />{/if}
      <Badges badges={sender.badges} {channel} userId={sender.id} />
      <span class="name {paint ?? ''}" class:painted={!!paint} role="link" tabindex="-1" data-user={sender.login}
        >{name.name}</span
      >{#if name.extra}<span class="extra">{` (${name.extra})`}</span>{/if}{#if !action}<span class="colon">{': '}</span>{:else}{' '}{/if}<span
        class="body"
        class:action><Body fragments={m.body} {channel} /></span
      >
    </div>
  {/if}
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
  .context[data-jump] {
    cursor: pointer;
  }
  .context[data-jump]:hover {
    color: var(--muted);
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
  .kind-milestone {
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
  .mention {
    background: var(--accent-dim);
    box-shadow: inset 3px 0 var(--accent);
  }
  .highlighted {
    background: color-mix(in srgb, var(--twitch) 16%, transparent);
    box-shadow: inset 3px 0 var(--twitch);
  }
  .first {
    box-shadow: inset 3px 0 var(--green);
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
