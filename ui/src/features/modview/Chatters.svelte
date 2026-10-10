<!-- 1.48 The viewer list, grouped (broadcaster, mods, VIPs, staff, a sample of viewers)
     with the total Twitch reports. Over GQL it's grouped; over Helix it's a flat sample. -->
<script lang="ts">
  import type { Chatters } from '../../ipc/bindings/Chatters';
  import { modIpc } from '../../ipc/moderation';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';

  let { account, login }: { account: string; login: string } = $props();

  let data = $state<Chatters | null>(null);
  let loading = $state(true);

  function load() {
    loading = true;
    modIpc
      .chatters(account, login)
      .then((c) => (data = c))
      .catch((e) => toast.error(String(e)))
      .finally(() => (loading = false));
  }
  load();

  const groups = $derived.by(() => {
    if (!data) return [];
    return [
      { label: 'Moderators', list: data.moderators },
      { label: 'VIPs', list: data.vips },
      { label: 'Staff', list: data.staff },
      { label: 'Viewers', list: data.viewers },
    ].filter((g) => g.list.length);
  });
</script>

<div class="bar">
  <span>{data ? `${formatNumber(Number(data.count))} in chat` : 'Viewers'}</span>
  <button class="link" onclick={load} disabled={loading}>{loading ? '…' : 'Refresh'}</button>
</div>
{#if data}
  {#each groups as g (g.label)}
    <div class="group">
      <div class="gl">{g.label} · {g.list.length}</div>
      <div class="names">{g.list.join(', ')}</div>
    </div>
  {/each}
{/if}

<style>
  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    font-size: 0.82rem;
  }
  .link {
    background: none;
    border: 0;
    color: var(--muted);
    cursor: pointer;
  }
  .link:hover {
    color: var(--text);
  }
  .group {
    padding: 6px 10px;
  }
  .gl {
    font-weight: 700;
    font-size: 0.78rem;
    color: var(--muted);
    margin-bottom: 2px;
  }
  .names {
    font-size: 0.82rem;
    overflow-wrap: anywhere;
    line-height: 1.5;
  }
</style>
