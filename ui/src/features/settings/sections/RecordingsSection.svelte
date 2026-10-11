<!-- Chat recordings (X6): start one from a chat's ⋯ menu, play or delete them here. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import type { RecordingMeta } from '../../../ipc/bindings/RecordingMeta';
  import { chatxIpc } from '../../../ipc/chatx';
  import { formatDate, formatDuration, formatNumber, formatTime } from '../../../lib/format';
  import { toast } from '../../../lib/toast.svelte';
  import { popOut } from '../../../app/panels';

  let list = $state.raw<RecordingMeta[] | null>(null);
  /** Asked once more before deleting. */
  let deleting = $state<string | null>(null);

  function load() {
    chatxIpc
      .recordings()
      .then((r) => (list = r))
      .catch((e) => toast.error(String(e)));
  }
  load();

  function remove(id: string) {
    deleting = null;
    chatxIpc
      .deleteRecording(id)
      .then(load)
      .catch((e) => toast.error(String(e)));
  }

  const when = (ms: number) => `${formatDate(new Date(ms))} ${formatTime(new Date(ms))}`;
  const long = (r: RecordingMeta) => formatDuration(Math.max(0, Math.floor((r.endedMs - r.startedMs) / 1000)));
</script>

<h2 data-setting="recordings:list">Recordings</h2>

{#if list === null}
  <p class="empty">Loading…</p>
{:else}
  {#each list as r (r.id)}
    <Row label="#{r.channel}" hint="{when(r.startedMs)} · {long(r)} · {formatNumber(r.messages)} messages">
      <div class="actions">
        {#if deleting === r.id}
          <Button small variant="danger" onclick={() => remove(r.id)}>Delete</Button>
          <Button small variant="ghost" onclick={() => (deleting = null)}>Keep</Button>
        {:else}
          <Button small icon="play" onclick={() => void popOut(`recording/${r.id}`, `#${r.channel} recording`)}>Play</Button>
          <IconButton icon="trash" label="Delete" size={14} onclick={() => (deleting = r.id)} />
        {/if}
      </div>
    </Row>
  {:else}
    <p class="empty">None yet. Record a chat from its ⋯ menu. Pictures are saved too, so replays work offline.</p>
  {/each}
{/if}

<style>
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .empty {
    margin: 6px 0;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
