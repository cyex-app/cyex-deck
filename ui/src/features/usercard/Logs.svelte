<!-- 1.36 Older messages from logs.cyex.app, newest first, a page at a time. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import { usercardIpc } from '../../ipc/rules';
  import { systemIpc } from '../../ipc/system';
  import { formatDate } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import Line from '../chat/Line.svelte';

  let { channel, userId, login }: { channel: string; userId: string; login: string } = $props();

  let lines = $state.raw<ChatMessage[]>([]);
  let status = $state<'loading' | 'lines' | 'none' | 'optedOut' | 'failed'>('loading');
  let more = $state(false);
  let busy = $state(false);
  let url = $state('');

  async function load() {
    busy = true;
    try {
      const page = await usercardIpc.logs(channel, userId, login, lines.length);
      lines = [...lines, ...page.messages];
      status = lines.length ? 'lines' : (page.state as typeof status);
      more = page.more;
      url = page.url;
    } catch (e) {
      if (!lines.length) status = 'failed';
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  void load();

  const day = (m: ChatMessage) => formatDate(new Date(m.ts));
</script>

<div class="head">
  <h3 class="label">Logs in {channel}</h3>
  {#if url}
    <button class="link" onclick={() => systemIpc.openLink(url)}>
      <Icon name="popout" size={12} /> logs.cyex.app
    </button>
  {/if}
</div>
<div class="messages">
  {#each lines as m, i (m.id)}
    {#if i === 0 || day(lines[i - 1]) !== day(m)}<div class="day">{day(m)}</div>{/if}
    <Line {m} {channel} />
  {:else}
    <p class="muted">
      {#if status === 'loading'}Loading…{:else if status === 'optedOut'}They opted out of logs.{:else if status === 'failed'}Couldn’t
        load logs.{:else}Nothing logged.{/if}
    </p>
  {/each}
</div>
{#if more}
  <div class="more"><Button small variant="ghost" disabled={busy} onclick={load}>Older</Button></div>
{/if}

<style>
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin: 10px 0 4px;
  }
  .head .label {
    margin: 0;
  }
  .link {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text);
    font-size: 0.8rem;
  }
  .link:hover {
    text-decoration: underline;
  }
  .messages {
    margin: 0 -14px;
  }
  .day {
    padding: 4px 14px 0;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
  }
  .muted {
    padding: 0 14px;
    color: var(--dim);
    font-size: 0.85rem;
  }
  .more {
    display: flex;
    justify-content: center;
    margin-top: 4px;
  }
</style>
