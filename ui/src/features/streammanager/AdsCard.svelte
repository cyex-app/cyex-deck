<!-- 1.35 Run an ad, see when the next scheduled one comes and snooze it. Ads only run
     while live; running one takes a second click. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { AdSchedule } from '../../ipc/bindings/AdSchedule';
  import { smIpc } from '../../ipc/streammanager';
  import { formatDuration } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import Card from './Card.svelte';
  import ConfirmButton from './ConfirmButton.svelte';
  import { adIn } from './logic';
  import { sm } from './streammanager.svelte';

  const LENGTHS = [30, 60, 90, 120, 180].map((v) => ({ value: v, label: `${v}s` }));

  let length = $state(60);
  let schedule = $state<AdSchedule | null>(null);
  let now = $state(Date.now());
  let busy = $state(false);

  const live = $derived(sm.stats?.live ?? false);
  const next = $derived(schedule ? adIn(schedule.nextAdAt, now) : null);

  function load() {
    smIpc
      .adSchedule()
      .then((s) => (schedule = s))
      .catch(() => (schedule = null));
  }

  // Read again when going live or offline.
  $effect(() => {
    void live;
    load();
  });

  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  function run() {
    busy = true;
    smIpc
      .commercial(length)
      .then((got) => {
        toast.info(`Ad running (${got}s).`);
        load();
      })
      .catch((e) => toast.error(String(e)))
      .finally(() => (busy = false));
  }

  function snooze() {
    busy = true;
    smIpc
      .snooze()
      .then(() => {
        toast.info('Next ad pushed back.');
        load();
      })
      .catch((e) => toast.error(String(e)))
      .finally(() => (busy = false));
  }
</script>

<Card title="Ads">
  <div class="line">
    <Segmented label="Ad length" options={LENGTHS} value={length} onchange={(v) => (length = v)} />
  </div>
  <div class="line">
    <ConfirmButton
      label="Run ad"
      confirm={`Click again to run a ${length}s ad`}
      disabled={!live || busy}
      onconfirm={run}
    />
  </div>
  {#if !live}
    <p class="note">Ads run only while you're live.</p>
  {:else if schedule}
    <dl>
      <div>
        <dt>Next ad</dt>
        <dd>{next == null ? 'None scheduled' : next === 0 ? 'Now' : `in ${formatDuration(next)}`}</dd>
      </div>
      {#if schedule.duration}<div><dt>Length</dt><dd>{schedule.duration}s</dd></div>{/if}
      {#if schedule.prerollFreeTime}<div><dt>Pre-roll free</dt><dd>{formatDuration(schedule.prerollFreeTime)}</dd></div>{/if}
      <div><dt>Snoozes left</dt><dd>{schedule.snoozeCount}</dd></div>
    </dl>
    <div class="line">
      <Button small disabled={busy || schedule.snoozeCount === 0 || next == null} onclick={snooze}>Snooze 5 minutes</Button>
    </div>
  {/if}
</Card>

<style>
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 12px;
    font-size: 0.82rem;
  }
  dl div {
    display: contents;
  }
  dt {
    color: var(--dim);
  }
  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
</style>
