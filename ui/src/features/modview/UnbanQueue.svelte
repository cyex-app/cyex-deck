<!-- M2 Unban request queue: pending requests, newest first, each approved or denied with
     a note the viewer sees. V107-N3b A failed read says so in the panel with Retry (it was a
     toast on every open, also from the dock's menu, and then read as "No pending requests"). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Card from '../../components/PanelCard.svelte';
  import type { UnbanRequest } from '../../ipc/bindings/UnbanRequest';
  import { modIpc } from '../../ipc/moderation';
  import { formatDate, formatTime } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { errorText } from '../../lib/errors';

  let { account, login }: { account: string; login: string } = $props();

  let list = $state<UnbanRequest[]>([]);
  let loading = $state(true);
  let note = $state<Record<string, string>>({});
  let busy = $state<Record<string, boolean>>({});
  let problem = $state('');

  function load() {
    loading = true;
    modIpc
      .unbanRequests(account, login)
      .then((r) => {
        list = r;
        problem = '';
      })
      .catch((e) => (problem = errorText(e)))
      .finally(() => (loading = false));
  }

  /** As the mod actions stamp a line: date and 24 h time. */
  const when = (iso: string) => {
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? '' : `${formatDate(d)} ${formatTime(d)}`;
  };
  load();

  async function resolve(r: UnbanRequest, approve: boolean) {
    if (busy[r.id]) return;
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

<Card title="Unban requests" flush>
  {#snippet tools()}
    <IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={load} />
  {/snippet}
{#if problem && !list.length}
  <p class="empty">{problem} <Button small variant="link" onclick={load}>Retry</Button></p>
{:else if !list.length}
  <p class="empty">{loading ? 'Loading…' : 'No pending requests.'}</p>
{:else}
  <ul>
    {#each list as r (r.id)}
      <li>
        <div class="who"><b>{r.display}</b> · <span class="t">{when(r.createdAt)}</span></div>
        {#if r.text}<div class="msg">{r.text}</div>{/if}
        <input placeholder="Note (optional)" aria-label="Note for {r.display}" maxlength="500" bind:value={note[r.id]} />
        <div class="row">
          <Button small disabled={busy[r.id]} onclick={() => resolve(r, true)}>Unban</Button>
          <Button small variant="danger" disabled={busy[r.id]} onclick={() => resolve(r, false)}>Deny</Button>
        </div>
      </li>
    {/each}
  </ul>
{/if}
</Card>

<style>
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
  .t {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
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
    margin: 0;
    padding: 12px;
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
