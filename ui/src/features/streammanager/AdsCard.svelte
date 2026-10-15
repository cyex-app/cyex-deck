<!-- 1.35 Run an ad, see when the next scheduled one comes and snooze it. Ads only run
     while live; running one takes a second click. D5 The schedule's settings (sign-in with
     a code): on or off, how long, how often, a reminder before each. D7 Changes on
     Twitch's side show at once. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import type { AdSchedule } from '../../ipc/bindings/AdSchedule';
  import type { SmAdEdit } from '../../ipc/bindings/SmAdEdit';
  import type { SmAdSettings } from '../../ipc/bindings/SmAdSettings';
  import { smIpc } from '../../ipc/streammanager';
  import { formatDuration } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import Card from '../../components/PanelCard.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import { adIn, adIntervals, adLengths } from './logic';
  import { sm } from './streammanager.svelte';

  const LENGTHS = [30, 60, 90, 120, 180].map((v) => ({ value: v, label: `${v}s` }));

  let length = $state(60);
  let schedule = $state<AdSchedule | null>(null);
  let adSet = $state<SmAdSettings | null>(null);
  /** D5 Minutes before an ad for the reminder. */
  const REMINDERS = [1, 2, 3, 5, 10, 15, 20];
  let now = $state(Date.now());
  let busy = $state(false);

  const live = $derived(sm.stats?.live ?? false);
  const next = $derived(schedule ? adIn(schedule.nextAdAt, now) : null);

  function load() {
    smIpc
      .adSchedule()
      .then((s) => (schedule = s))
      .catch(() => (schedule = null));
    smIpc
      .adSettings()
      .then((s) => (adSet = s))
      .catch(() => (adSet = null));
  }

  $effect(() => {
    const off = smIpc.onAds(load);
    return () => void off.then((f) => f());
  });

  /** D5 Shows one change at once and saves it; a refusal reads Twitch's settings again. */
  function save(edit: SmAdEdit) {
    if (adSet) {
      const s = adSet;
      adSet = {
        ...s,
        enabled: edit.enabled ?? s.enabled,
        duration: { ...s.duration, value: edit.duration ?? s.duration.value },
        interval: { ...s.interval, value: edit.interval ?? s.interval.value },
        reminder: edit.reminder ?? s.reminder,
        reminderSecs: edit.reminderSecs ?? s.reminderSecs,
      };
    }
    smIpc.setAdSettings(edit).catch((e) => {
      toast.error(String(e));
      load();
    });
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
    <p class="note">Ads run only while you’re live.</p>
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
  {#if adSet}
    {@const s = adSet}
    <div class="line">
      <Toggle label="Scheduled ads" checked={s.enabled} onchange={(v) => save({ enabled: v })} />
      <span>Scheduled ads</span>
    </div>
    {#if s.enabled}
      <div class="line">
        <select class="field" aria-label="Ad length" value={s.duration.value} onchange={(e) => save({ duration: Number(e.currentTarget.value) })}>
          {#each adLengths(s.duration.min, s.duration.max, s.duration.value) as v (v)}<option value={v}>{formatDuration(v)} ads</option>{/each}
        </select>
        <select class="field" aria-label="How often" value={s.interval.value} onchange={(e) => save({ interval: Number(e.currentTarget.value) })}>
          {#each adIntervals(s.interval.min, s.interval.max, s.interval.value) as v (v)}<option value={v}>every {formatDuration(v)}</option>{/each}
        </select>
      </div>
      <div class="line">
        <Toggle label="Remind me" checked={s.reminder} onchange={(v) => save({ reminder: v })} />
        <span>Remind me</span>
        {#if s.reminder}
          <select
            class="field"
            aria-label="How long before"
            value={s.reminderSecs}
            onchange={(e) => save({ reminderSecs: Number(e.currentTarget.value) })}
          >
            {#each REMINDERS.map((m) => m * 60).filter((v) => !s.reminderMax || v <= s.reminderMax) as v (v)}
              <option value={v}>{formatDuration(v)} before</option>
            {/each}
          </select>
        {/if}
      </div>
    {/if}
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
