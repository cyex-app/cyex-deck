<!-- M2 Unban request queue: pending requests, newest first, each approved or denied with
     a note the viewer sees. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { UnbanRequest } from '../../ipc/bindings/UnbanRequest';
  import { modIpc } from '../../ipc/moderation';
  import { formatDate } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { errorText } from '../../lib/errors';

  let { account, login }: { account: string; login: string } = $props();

  let list = $state<UnbanRequest[]>([]);
  let loading = $state(true);
  let note = $state<Record<string, string>>({});
  let busy = $state<Record<string, boolean>>({});

  function load() {
    loading = true;
    modIpc
      .unbanRequests(account, login)
      .then((r) => (list = r))
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (loading = false));
  }
  load();

  async function resolve(r: UnbanRequest, approve: boolean) {
    busy[r.id] = true;
    try {
      await modIpc.resolveUnban(account, login, r.id, approve, note[r.id] ?? '');
      list = list.filter((x) => x.id !== r.id);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy[r.id] = false;
    }
  }
</script>

<div class="bar">
  <Button small variant="link" onclick={load} disabled={loading}>Refresh</Button>
</div>
{#if !loading && list.length === 0}
  <p class="empty">No pending requests.</p>
{:else}
  <ul>
    {#each list as r (r.id)}
      <li>
        <div class="who"><b>{r.display}</b> · {formatDate(new Date(r.createdAt))}</div>
        {#if r.text}<div class="msg">{r.text}</div>{/if}
        <input placeholder="Note (optional)" bind:value={note[r.id]} />
        <div class="row">
          <Button small disabled={busy[r.id]} onclick={() => resolve(r, true)}>Unban</Button>
          <Button small variant="danger" disabled={busy[r.id]} onclick={() => resolve(r, false)}>Deny</Button>
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .bar {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
  }
  ul {
    margin: 0;
    padding: 6px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    padding: 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .who {
    font-size: 0.82rem;
  }
  .msg {
    margin: 4px 0;
    font-size: 0.86rem;
    overflow-wrap: anywhere;
  }
  input {
    width: 100%;
    margin: 4px 0 6px;
    padding: 4px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.8rem;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .empty {
    padding: 12px;
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
