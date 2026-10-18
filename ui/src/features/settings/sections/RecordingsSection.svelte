<!-- Chat recordings (X6): start one from a chat's ⋯ menu, play or delete them here. -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import type { RecordingMeta } from '../../../ipc/bindings/RecordingMeta';
  import { chatxIpc } from '../../../ipc/chatx';
  import { formatDate, formatDuration, formatNumber, formatTime } from '../../../lib/format';
  import { toast } from '../../../lib/toast.svelte';
  import { popOut } from '../../../app/panels';
  import { errorText } from '../../../lib/errors';
  import EmptyState from '../../../components/EmptyState.svelte';
  import Loading from '../../../components/Loading.svelte';
  import ConfirmButton from '../../../components/ConfirmButton.svelte';

  let list = $state.raw<RecordingMeta[] | null>(null);

  function load() {
    chatxIpc
      .recordings()
      .then((r) => (list = r))
      .catch((e) => toast.error(errorText(e)));
  }
  load();

  function remove(id: string) {
    chatxIpc
      .deleteRecording(id)
      .then(load)
      .catch((e) => toast.error(errorText(e)));
  }

  const when = (ms: number) => `${formatDate(new Date(ms))} ${formatTime(new Date(ms))}`;
  const long = (r: RecordingMeta) => formatDuration(Math.max(0, Math.floor((r.endedMs - r.startedMs) / 1000)));
</script>

<SectionHead title="Recordings" level={2} setting="recordings:list" />

{#if list === null}
  <Loading rows={3} />
{:else}
  {#each list as r (r.id)}
    <Row label="#{r.channel}" hint="{when(r.startedMs)} · {long(r)} · {formatNumber(r.messages)} messages">
      <div class="actions">
        <Button small icon="play" onclick={() => void popOut(`recording/${r.id}`, `#${r.channel} recording`)}>Play</Button>
        <ConfirmButton label="Delete" confirm="Click again to delete" icon="trash" variant="ghost" onconfirm={() => remove(r.id)} />
      </div>
    </Row>
  {:else}
    <EmptyState align="start" icon="film" text="None yet. Record a chat from its ⋯ menu. Pictures are saved too, so replays work offline." />
  {/each}
{/if}

<style>
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
</style>
