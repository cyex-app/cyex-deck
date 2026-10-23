<!-- 1.48, 1.55 Mass actions: paste a list of logins and ban, time out or unban them. Helix bans
     one at a time (slow); a GQL sign-in batches 30 at a time (fast), so Helix-only users
     get a hint. M1 /nuke lives on the chat input; here is the bulk-list tool for
     followbots and raids. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { MassbanProgress } from '../../ipc/bindings/MassbanProgress';
  import { modIpc } from '../../ipc/moderation';
  import { accounts } from '../../lib/accounts.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { errorText } from '../../lib/errors';
  import NumberField from '../../components/NumberField.svelte';
  import Select from '../../components/Select.svelte';
  import { formatNumber } from '../../lib/format.ts';
  import { DURATION_UNITS, type DurationUnit, durationError, timeoutSecs } from './massDuration.ts';
  import { massEnd, tallyDetail, tallyParts } from './massTally.ts';

  let { account, login, channelId }: { account: string; login: string; channelId: string | null } = $props();

  let text = $state('');
  let reason = $state('');
  // V107-R21b Duration as number + unit; 0 bans.
  let amount = $state(0);
  let unit = $state<DurationUnit>('minutes');
  const err = $derived(durationError(amount, unit));
  let running = $state(false);
  let progress = $state<MassbanProgress | null>(null);
  // V107-R21 The run this panel follows: its account's, the newest one.
  let job: number | null = null;
  // V107-R21 What the running (or last) job was, for the outcome words.
  let kind = $state<'ban' | 'timeout' | 'unban'>('ban');

  // A GQL sign-in batches; Helix-only bans one at a time.
  const hasGql = $derived(accounts.list.find((a) => a.userId === account)?.features.includes('points') ?? false);

  // v1.07 T4: each login once ("bob @Bob" is one person).
  const logins = $derived([
    ...new Set(
      text
        .split(/[\s,]+/)
        .map((s) => s.trim().replace(/^@/, '').toLowerCase())
        .filter((s) => /^[a-z0-9_]{1,25}$/.test(s)),
    ),
  ]);

  $effect(() => {
    if (!channelId) return;
    let stop: (() => void) | undefined;
    let gone = false;
    void modIpc
      .onMassban((p) => {
        if (p.channelId === channelId && p.account === account && (job === null || p.job >= job)) {
          job = p.job;
          progress = p;
          running = p.running;
        }
      })
      .then((u) => (gone ? u() : (stop = u)));
    return () => {
      gone = true;
      stop?.();
    };
  });

  // V107-R21 Unban asks once, right here (a second click within 3 s).
  let confirmUnban = $state(false);

  // V107-N12b Stop was pressed for the running job: its end says so.
  let stopping = false;

  function go(unban = false) {
    if (running || !logins.length || (!unban && err)) return;
    job = null;
    stopping = false;
    confirmUnban = false;
    const secs = unban ? null : timeoutSecs(amount, unit);
    kind = unban ? 'unban' : secs ? 'timeout' : 'ban';
    const total = logins.length;
    running = true;
    modIpc
      .massban(account, login, logins, secs, unban ? '' : reason, unban)
      .then((n) => {
        // The last progress has how many were tried (`n` is how many went through).
        const end = massEnd(n, progress?.job === job ? progress.done : n, total, stopping);
        if (end.stopped) toast.info(end.line);
        else toast.success(end.line);
      })
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (running = false));
  }

  function stop() {
    stopping = true;
    if (channelId) void modIpc.stopMassban(channelId, account);
  }
</script>

<div class="mass">
  <textarea placeholder="Logins, separated by spaces, commas or lines" bind:value={text} rows={4}></textarea>
  <div class="row">
    <span class="lab">Timeout</span>
    <NumberField label="Timeout duration" min={0} max={99999999} width="96px" value={amount} onchange={(n) => (amount = n)} />
    <Select label="Unit" width="104px" options={DURATION_UNITS} value={unit} onchange={(v) => (unit = v as DurationUnit)} />
    <span class="note">0 = ban</span>
  </div>
  {#if err}<p class="err">{err}</p>{/if}
  <input class="reason" placeholder="Reason (optional)" bind:value={reason} />
  <div class="go">
    {#if running}
      <Button small variant="danger" onclick={stop}>Stop</Button>
    {:else}
      <Button small disabled={logins.length === 0 || confirmUnban || !!err} onclick={() => go()}>
        {amount > 0 ? 'Time out' : 'Ban'} {logins.length ? formatNumber(logins.length) : ''}
      </Button>
      <ConfirmButton
        label="Unban"
        confirm="Unban {formatNumber(logins.length)}?"
        variant="ghost"
        disabled={logins.length === 0}
        bind:armed={confirmUnban}
        onconfirm={() => go(true)}
      />
    {/if}
    {#if progress}
      <span class="prog">{formatNumber(progress.done)}/{formatNumber(progress.total)}</span>
    {/if}
  </div>
  {#if progress && progress.done > 0}
    <p class="tally" use:tooltip={tallyDetail(progress.tally)}>{tallyParts(progress.tally, progress.unban ? 'unban' : kind === 'unban' ? 'ban' : kind).join(' · ')}</p>
  {/if}
  {#if !hasGql}
    <p class="hint">Add a GQL sign-in to this account in Settings → Accounts to ban far faster.</p>
  {/if}
</div>

<style>
  .mass {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  textarea,
  .reason {
    width: 100%;
    padding: 6px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.82rem;
    resize: vertical;
  }
  /* The shared field's focus: the border, not a ring outside the box. */
  textarea:focus-visible,
  .reason:focus-visible {
    outline: none;
    border-color: var(--focus-ring);
    box-shadow: 0 0 0 1px var(--focus-ring);
  }
  textarea::placeholder,
  .reason::placeholder {
    color: var(--dim);
  }
  /* A narrow panel wraps the unit and its note under the number. */
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 8px;
    min-height: 28px;
  }
  .lab {
    width: 56px;
    flex: none;
    font-size: 0.82rem;
    color: var(--muted);
  }
  .note {
    font-size: 0.76rem;
    color: var(--muted);
    white-space: nowrap;
  }
  .err {
    margin: 0;
    font-size: 0.76rem;
    color: var(--red);
  }
  .go {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 8px;
  }
  .prog {
    font-size: 0.8rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .tally {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .hint {
    margin: 0;
    font-size: 0.76rem;
    color: var(--muted);
  }
</style>
