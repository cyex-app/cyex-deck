<!-- V108-N03 A channel's planned streams in About: the next ten, a vacation if there is one,
     and the whole schedule as an .ics file for a calendar. Nothing shows without a schedule. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { ChannelSchedule } from '../../ipc/bindings/ChannelSchedule';
  import { scheduleIpc } from '../../ipc/browse';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { segmentTime, when } from '../browse/schedule';

  let { login, name }: { login: string; name: string } = $props();

  let schedule = $state<ChannelSchedule | null>(null);
  let saving = $state(false);

  // A failed read shows nothing: About works without it.
  $effect(() => {
    const l = login;
    let alive = true;
    schedule = null;
    scheduleIpc
      .channel(l)
      .then((s) => alive && (schedule = s))
      .catch(() => {});
    return () => (alive = false);
  });

  const planned = $derived(schedule?.segments.some((s) => !s.cancelled) ?? false);

  async function save() {
    saving = true;
    try {
      if (await scheduleIpc.saveIcs(login, name)) toast.success('Schedule saved.');
    } catch (e) {
      toast.error(errorText(e, 'save the schedule'));
    } finally {
      saving = false;
    }
  }
</script>

{#if schedule && (schedule.segments.length || schedule.vacation)}
  <section class="schedule" aria-label="Schedule">
    <div class="head">
      <h3>Schedule</h3>
      {#if planned}<Button small disabled={saving} onclick={() => void save()}>Save as .ics</Button>{/if}
    </div>
    {#if schedule.vacation}
      <p class="break">
        On a break until {when(schedule.vacation.end)}{schedule.vacation.reason ? `: ${schedule.vacation.reason}` : ''}
      </p>
    {/if}
    <ul>
      {#each schedule.segments as s (s.id + s.start)}
        <li class:cancelled={s.cancelled}>
          <span class="time">{segmentTime(s)}</span>
          <span class="what">{s.title || s.category || 'Stream'}</span>
          {#if s.title && s.category}<span class="cat">{s.category}</span>{/if}
          {#if s.cancelled}<span class="cat">Skipped</span>{/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .schedule {
    margin: 12px 16px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 6px;
  }
  h3 {
    margin: 0;
    font-size: 0.95rem;
  }
  .break {
    margin: 0 0 6px;
    color: var(--muted);
    font-size: 0.82rem;
  }
  ul {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 10px;
    padding: 5px 0;
    border-bottom: 1px solid var(--border);
    font-size: 0.82rem;
  }
  .time {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .what {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .cat {
    color: var(--dim);
    font-size: 0.75rem;
  }
  .cancelled .time,
  .cancelled .what {
    color: var(--dim);
    text-decoration: line-through;
  }
</style>
