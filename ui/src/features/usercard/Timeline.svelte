<!-- V101-05 The usercard's messages in one scroll window of its own, oldest at the top as in
     chat: logs.cyex.app by day, then what's still loaded in the open chat under "Live"
     (the 7TV extension's way). Scrolling to the top loads older logs. -->
<script lang="ts">
  import { tick } from 'svelte';

  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import { usercardIpc } from '../../ipc/rules';
  import { formatDate } from '../../lib/format';
  import Line from '../chat/Line.svelte';

  interface Props {
    channel: string;
    userId: string;
    login: string;
    name: string;
    live: ChatMessage[];
    /** The logs' page on logs.cyex.app, once known. */
    url?: string;
  }

  let { channel, userId, login, name, live, url = $bindable('') }: Props = $props();

  /** From the logs, oldest first. */
  let older = $state.raw<ChatMessage[]>([]);
  let fetched = 0;
  let status = $state<'loading' | 'lines' | 'none' | 'optedOut' | 'failed'>('loading');
  let more = $state(true);
  let busy = false;
  let box = $state<HTMLElement>();
  let inner = $state<HTMLElement>();
  let atBottom = true;

  const liveIds = $derived(new Set(live.map((m) => m.id)));
  const sections = $derived.by(() => {
    const out: { label: string; live: boolean; items: ChatMessage[] }[] = [];
    for (const m of older) {
      if (liveIds.has(m.id)) continue;
      const label = formatDate(new Date(m.ts));
      const last = out.at(-1);
      if (last?.label === label) last.items.push(m);
      else out.push({ label, live: false, items: [m] });
    }
    if (live.length) out.push({ label: 'Live', live: true, items: live });
    return out;
  });

  async function loadOlder() {
    if (busy || !more) return;
    busy = true;
    const from = box ? box.scrollHeight - box.scrollTop : 0;
    try {
      const page = await usercardIpc.logs(channel, userId, login, fetched);
      fetched += page.messages.length;
      older = [...[...page.messages].reverse(), ...older];
      status = older.length ? 'lines' : (page.state as typeof status);
      more = page.more;
      url = page.url;
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

  void loadOlder();
</script>

<div class="list" bind:this={box} {onscroll}>
  <div bind:this={inner}>
    {#if status === 'optedOut'}<p class="note">They opted out of logs.</p>{/if}
    {#if status === 'failed'}<p class="note">Couldn’t load logs.</p>{/if}
    {#each sections as s (s.label)}
      <div class="day" class:live={s.live}><span>{s.label}</span></div>
      {#each s.items as m (m.id)}<Line {m} {channel} />{/each}
    {:else}
      <p class="empty">{status === 'loading' ? 'Loading…' : `${name} hasn't chatted here.`}</p>
    {/each}
  </div>
</div>

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
