<!-- 1.26 One conversation: lines drawn like chat, older ones when scrolled up, Twitch's spam
     warning, thread options and the box. While it's on screen in a focused window it
     counts as read and doesn't alert. -->
<script lang="ts">
  import { tick, untrack } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenuAt, type MenuEntry } from '../../components/menu.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { WhisperConversation } from '../../ipc/bindings/WhisperConversation';
  import type { WhisperEdit } from '../../ipc/bindings/WhisperEdit';
  import { systemIpc } from '../../ipc/system';
  import { whispersIpc } from '../../ipc/whispers';
  import { formatDate } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import Line from '../chat/Line.svelte';
  import { withUndo } from '../chat/undo';
  import WhisperInput from './WhisperInput.svelte';
  import { whispers } from './whispers.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  interface Props {
    login: string;
    account: string | null;
    /** Can load older lines and change thread options (GQL sign-in). */
    history: boolean;
  }

  let { login, account, history }: Props = $props();

  /** Newest lines kept in the list. */
  const MAX_LINES = 2000;

  let conv = $state.raw<WhisperConversation | null>(null);
  let messages = $state.raw<ChatMessage[]>([]);
  let cursor = $state<string | null>(null);
  let error = $state('');
  let older = $state(false);
  let list = $state<HTMLElement>();
  let input = $state<ReturnType<typeof WhisperInput>>();
  /** Follows new lines while scrolled to the bottom. */
  let atBottom = true;

  const thread = $derived(whispers.state.threads.find((t) => t.id === conv?.thread.id) ?? conv?.thread ?? null);
  const color = $derived(thread ? `--cd:${thread.other.color.dark};--cl:${thread.other.color.light}` : '');

  $effect(() => {
    const l = login;
    untrack(() => void load(l));
  });

  /** `fresh` empties the list first; a refresh of the same thread keeps it until the new one lands. */
  async function load(l: string, fresh = true) {
    if (fresh) {
      conv = null;
      messages = [];
      cursor = null;
      older = false;
    }
    error = '';
    try {
      const c = await whispersIpc.open(l);
      if (l !== login) return;
      conv = c;
      messages = c.messages;
      cursor = c.cursor;
      await tick();
      bottom();
      read();
      input?.focus();
    } catch (e) {
      if (l === login) error = errorText(e);
    }
  }

  $effect(() => {
    const line = whispersIpc.onLine((l) => {
      if (!conv || l.threadId !== conv.thread.id || messages.some((m) => m.id === l.message.id)) return;
      messages = [...messages, l.message].slice(-MAX_LINES);
      if (atBottom) void tick().then(bottom);
      if (document.hasFocus()) read();
    });
    const refresh = whispersIpc.onRefresh((id) => {
      if (conv?.thread.id === id) void load(login, false);
    });
    return () => {
      void line.then((f) => f());
      void refresh.then((f) => f());
    };
  });

  // The thread on screen in a focused window gets no unread count or alerts.
  $effect(() => {
    const id = conv?.thread.id;
    if (!id) return;
    const update = () => {
      const seen = document.hasFocus() && !document.hidden;
      whispersIpc.viewing(seen ? id : null).catch(() => {});
      if (seen) read();
    };
    update();
    addEventListener('focus', update);
    addEventListener('blur', update);
    document.addEventListener('visibilitychange', update);
    return () => {
      removeEventListener('focus', update);
      removeEventListener('blur', update);
      document.removeEventListener('visibilitychange', update);
      whispersIpc.viewing(null).catch(() => {});
    };
  });

  function read() {
    const id = conv?.thread.id;
    const t = whispers.state.threads.find((x) => x.id === id);
    if (id && t && t.unread > 0) whispersIpc.read(id).catch(() => {});
  }

  function bottom() {
    if (list) list.scrollTop = list.scrollHeight;
  }

  async function onscroll() {
    if (!list) return;
    atBottom = list.scrollHeight - list.scrollTop - list.clientHeight < 40;
    if (list.scrollTop > 60 || !cursor || older || !conv) return;
    older = true;
    const before = list.scrollHeight;
    const id = conv.thread.id;
    try {
      const page = await whispersIpc.older(id, cursor);
      // Another thread opened meanwhile.
      if (conv?.thread.id !== id) return;
      const known = new Set(messages.map((m) => m.id));
      messages = [...page.messages.filter((m) => !known.has(m.id)), ...messages];
      cursor = page.cursor;
      await tick();
      list.scrollTop += list.scrollHeight - before;
    } catch (e) {
      if (conv?.thread.id === id) toast.error(errorText(e));
    } finally {
      if (conv?.thread.id === id) older = false;
    }
  }

  function click(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[data-link]');
    if (!el?.dataset.link) return;
    e.preventDefault();
    systemIpc.openChatLink(el.dataset.link).catch((err) => toast.error(errorText(err)));
  }

  async function edit(change: WhisperEdit, done?: string) {
    if (!conv) return;
    try {
      await whispersIpc.edit(conv.thread.id, change);
      if (done) toast.info(done);
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  /** Blocks on Twitch and archives the thread; Undo (V1052-01) lifts both, with no chat needed. */
  async function block() {
    if (!thread) return;
    const { id, other } = thread;
    await withUndo(
      `Blocked ${other.name}.`,
      async () => {
        await whispersIpc.block(other.login);
        await whispersIpc.edit(id, { archived: true });
      },
      async () => {
        await whispersIpc.unblock(other.login);
        await whispersIpc.edit(id, { archived: false });
      },
    );
  }

  function options(e: MouseEvent) {
    if (!thread) return;
    const items: MenuEntry[] = [];
    if (history) {
      items.push(
        {
          label: thread.muted ? 'Unmute alerts' : 'Mute alerts',
          icon: 'bellOff',
          run: () => void edit({ muted: !thread.muted }),
        },
        {
          label: thread.archived ? 'Move back to the list' : 'Archive',
          icon: 'archive',
          run: () => void edit({ archived: !thread.archived }),
        },
      );
      if (thread.temporary) {
        items.push({
          label: 'Take back their access',
          icon: 'lock',
          note: 'You whispered them, so they can whisper you for 48 hours.',
          run: () => void edit({ revoke: true }, 'They can’t whisper you anymore.'),
        });
      }
      items.push('separator');
    }
    items.push({ label: 'Block', icon: 'ban', note: 'Blocks them on Twitch', danger: true, run: () => void block() });
    openMenuAt(e, items);
  }

  /** V5 Days run together: one date row covers messages up to a day apart, over 3 days at most. */
  const DAY_MS = 86_400_000;
  const dayNo = (ts: number) => {
    const d = new Date(ts);
    return Math.round(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / DAY_MS);
  };
  /** The row text for the messages that start a group, by index. */
  const dates = $derived.by(() => {
    const out = new Map<number, string>();
    let start = 0;
    let last = 0;
    const flush = (end: number) => {
      const a = new Date(messages[start].ts);
      const b = new Date(messages[end].ts);
      const from = formatDate(a);
      const to = formatDate(b);
      out.set(start, from === to ? from : `${from} – ${a.getFullYear() === b.getFullYear() ? to.slice(5) : to}`);
    };
    messages.forEach((m, i) => {
      const day = dayNo(m.ts);
      if (i === 0) start = i;
      else if (day - dayNo(messages[last].ts) > 1 || day - dayNo(messages[start].ts) > 3) {
        flush(last);
        start = i;
      }
      last = i;
    });
    if (messages.length) flush(last);
    return out;
  });
</script>

<div class="conv">
  {#if thread}
    <header style={color}>
      <Avatar src={thread.other.avatar} name={thread.other.name} size={28} />
      <span class="name">{thread.other.name}</span>
      {#if thread.other.name.toLowerCase() !== thread.other.login}<span class="login">{thread.other.login}</span>{/if}
      {#if thread.muted}<span class="tag">Muted</span>{/if}
      <span class="grow"></span>
      <IconButton icon="more" label="Options" hint="Mute, archive or block." onclick={options} />
    </header>
  {/if}
  {#if thread?.spam}
    <div class="spam">
      <span>Twitch thinks this might be spam. Don’t share passwords or personal details.</span>
      <div class="row">
        <Button small onclick={() => void edit({ spam: false })}>Not spam</Button>
        <Button small variant="danger" onclick={() => void edit({ spam: true, archived: true }, 'Reported.')}>Report spam</Button>
        <Button small variant="ghost" onclick={() => void block()}>Block</Button>
      </div>
    </div>
  {/if}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="lines" bind:this={list} {onscroll} onclick={click}>
    {#if older}<Loading />{/if}
    {#if error}
      <p class="note err">{error} <Button small onclick={() => void load(login)}>Retry</Button></p>
    {:else if !conv}
      <Loading />
    {:else if !messages.length}
      <p class="note">
        {history ? 'No whispers yet. Say hi.' : 'Whispers from this session show here. Past ones need a GQL sign-in.'}
      </p>
    {/if}
    {#each messages as m, i (m.id)}
      {#if dates.has(i)}<div class="day"><span>{dates.get(i)}</span></div>{/if}
      <Line {m} channel="" inWhispers />
    {/each}
  </div>
  {#if conv}
    <WhisperInput bind:this={input} to={thread?.other.login ?? login} {account} />
  {/if}
</div>

<style>
  .conv {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }
  .name {
    color: var(--cd);
    font-weight: 700;
  }
  :global(:root[data-theme='light']) .name {
    color: var(--cl);
  }
  .login {
    color: var(--dim);
    font-size: 12px;
  }
  .tag {
    padding: 1px 6px;
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--dim);
    font-size: 11px;
  }
  .grow {
    flex: 1;
  }
  .spam {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 8px 10px 0;
    padding: 10px;
    border-left: 3px solid var(--yellow);
    background: color-mix(in srgb, var(--yellow) 10%, transparent);
    color: var(--text);
    font-size: 13px;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .lines {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }
  .note {
    margin: 16px;
    color: var(--dim);
    text-align: center;
  }
  .err {
    color: var(--red);
  }
  .day {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 10px 4px;
    color: var(--dim);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.06em;
    opacity: 0.8;
  }
  .day::before,
  .day::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border);
  }
</style>
