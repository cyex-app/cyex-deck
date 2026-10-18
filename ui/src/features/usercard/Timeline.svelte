<!-- V101-05 The usercard's messages in one scroll window of its own, oldest at the top as in
     chat: logs.cyex.app by day, then what's still loaded in the open chat under "Live"
     (the 7TV extension's way). Scrolling to the top loads older logs. M8 Where
     logs.cyex.app has none, a mod gets Twitch's own mod logs instead. Emotes, names, links
     and badges act as in chat (lineEvents.ts); an emote's card opens over the card. -->
<script lang="ts">
  import { onDestroy, tick } from 'svelte';

  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import { usercardIpc } from '../../ipc/rules';
  import { formatDate } from '../../lib/format';
  import { openMenu } from '../../components/menu.svelte';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import type { CardTarget } from '../emotes/emotes';
  import { emoteMenu, enterClicks, lineTips, notEmote, openPart, PARTS, type LineHost } from '../chat/lineEvents';
  import Line from '../chat/Line.svelte';
  import Loading from '../../components/Loading.svelte';

  interface Props {
    channel: string;
    userId: string;
    login: string;
    name: string;
    live: ChatMessage[];
    /** The logs' page on logs.cyex.app, once known. */
    url?: string;
    /** The account that moderates here, for Twitch's mod logs (M8). */
    modAccount?: string | null;
    /** The account reading this chat, for 7TV edits from an emote's menu. */
    account?: string | null;
  }

  let { channel, userId, login, name, live, url = $bindable(''), modAccount = null, account = null }: Props = $props();

  /** 1.41 The emote card on screen. */
  let card = $state<CardTarget | null>(null);
  const host = (): LineHost => ({ channel, account, senderOf: () => userId, openCard: (t) => (card = t), openMenu });
  const tips = lineTips(() => inner);
  onDestroy(tips.drop);

  function onclick(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>(PARTS);
    if (el) openPart(e, el, host());
  }

  function oncontextmenu(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[data-emote]');
    if (el) void emoteMenu(e, el, host());
    else notEmote();
  }

  /** From the logs, oldest first. */
  let older = $state.raw<ChatMessage[]>([]);
  let fetched = 0;
  let status = $state<'loading' | 'lines' | 'none' | 'optedOut' | 'failed'>('loading');
  /** M8 Twitch's mod logs, once logs.cyex.app had nothing. */
  let fromMod = $state(false);
  let cursor: string | null = null;
  let more = $state(true);
  let busy = false;
  let box = $state<HTMLElement>();
  let inner = $state<HTMLElement>();
  let atBottom = true;

  const liveIds = $derived(new Set(live.map((m) => m.id)));
  /** The open chat's lines are the newest part; logs fill in only what is older than them
   *  (the two overlap, and their ids can differ), so the order never goes back in time. */
  const liveFrom = $derived(live.reduce((min, m) => Math.min(min, m.ts), Infinity));
  const sections = $derived.by(() => {
    const out: { label: string; live: boolean; items: ChatMessage[] }[] = [];
    for (const m of older) {
      if (liveIds.has(m.id) || m.ts >= liveFrom) continue;
      const label = formatDate(new Date(m.ts));
      const last = out.at(-1);
      if (last?.label === label) last.items.push(m);
      else out.push({ label, live: false, items: [m] });
    }
    if (live.length) out.push({ label: 'Live', live: true, items: [...live].sort((a, b) => a.ts - b.ts) });
    return out;
  });

  async function loadOlder() {
    if (busy || !more) return;
    busy = true;
    const from = box ? box.scrollHeight - box.scrollTop : 0;
    try {
      const page =
        fromMod && modAccount
          ? await usercardIpc.modLogs(modAccount, channel, userId, cursor)
          : await usercardIpc.logs(channel, userId, login, fetched);
      fetched += page.messages.length;
      cursor = page.cursor;
      older = [...[...page.messages].reverse(), ...older];
      status = older.length ? 'lines' : (page.state as typeof status);
      more = page.more;
      if (!fromMod) url = page.url;
    } catch {
      if (!older.length) status = 'failed';
      more = false;
    } finally {
      busy = false;
    }
    await tick();
    // Older lines go in above without moving what's on screen.
    if (box) box.scrollTop = box.scrollHeight - from;
    // Too few to scroll yet: keep going.
    if (box && more && box.scrollHeight <= box.clientHeight) void loadOlder();
  }

  function onscroll() {
    if (!box) return;
    tips.drop();
    atBottom = box.scrollHeight - box.scrollTop - box.clientHeight < 24;
    if (box.scrollTop < 40) void loadOlder();
  }

  // Starts at the newest line and stays there while new ones come in and while pictures
  // load and make lines taller.
  $effect(() => {
    if (!box || !inner) return;
    const list = box;
    const ro = new ResizeObserver(() => {
      if (atBottom) list.scrollTop = list.scrollHeight;
    });
    ro.observe(inner);
    return () => ro.disconnect();
  });

  // M8 Nothing on logs.cyex.app: a mod reads Twitch's own logs, once the account is known.
  $effect(() => {
    if (modAccount && !fromMod && !older.length && (status === 'none' || status === 'optedOut' || status === 'failed')) {
      fromMod = true;
      more = true;
      status = 'loading';
      void loadOlder();
    }
  });

  void loadOlder();
</script>

<!-- Clicks, keys and hovers are delegated from the lines' emotes, names and links. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_mouse_events_have_key_events -->
<div
  class="list"
  bind:this={box}
  {onscroll}
  {onclick}
  {oncontextmenu}
  onkeydown={enterClicks}
  onmouseover={tips.onmouseover}
  onmouseout={tips.onmouseout}
>
  <div bind:this={inner}>
    {#if status === 'optedOut'}<p class="note">They opted out of logs.</p>{/if}
    {#if fromMod && older.length}<p class="note">From Twitch’s mod logs.</p>{/if}
    {#if status === 'failed'}<p class="note">Couldn’t load logs.</p>{/if}
    {#each sections as s (s.label)}
      <div class="day" class:live={s.live}><span>{s.label}</span></div>
      {#each s.items as m (m.id)}<Line {m} {channel} />{/each}
    {:else}
      {#if status === 'loading'}<Loading />{:else}<p class="empty">{name} hasn’t chatted here.</p>{/if}
    {/each}
  </div>
</div>
{#if card}
  {#key card}<EmoteCard target={card} login={channel} {account} onclose={() => (card = null)} />{/key}
{/if}

<style>
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 4px;
  }
  .day {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 14px 2px;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
  }
  .day::before,
  .day::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border);
  }
  .day.live {
    color: var(--red);
  }
  .note,
  .empty {
    margin: 0;
    padding: 8px 14px;
    color: var(--dim);
    font-size: 0.82rem;
  }
  .empty {
    padding-top: 40px;
    text-align: center;
  }
</style>
