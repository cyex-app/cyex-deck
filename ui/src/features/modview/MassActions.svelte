<!-- 1.48, 1.55 Mass actions: paste a list of logins and ban, time out or unban them. Helix bans
     one at a time (slow); a GQL sign-in batches 30 at a time (fast), so Helix-only users
     get a hint. M1 /nuke lives on the chat input; here is the bulk-list tool for
     followbots and raids. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
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
  import { tallyDetail, tallyParts } from './massTally.ts';

  let { account, login, channelId }: { account: string; login: string; channelId: string | null } = $props();

  let text = $state('');
  let reason = $state('');
  // V107-R21b Duration as number + unit; 0 bans.
  let amount = $state(0);
  let unit = $state<DurationUnit>('minutes');
  const err = $derived(durationError(amount, unit));
  let running = $state(false);
  let progress = $state<MassbanProgress | null>(null);
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
        if (p.channelId === channelId) {
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

  // V107-R21 Unban asks once, right here.
  let confirmUnban = $state(false);

  function go(unban = false) {
    if (!logins.length || (!unban && err)) return;
    confirmUnban = false;
    const secs = unban ? null : timeoutSecs(amount, unit);
    kind = unban ? 'unban' : secs ? 'timeout' : 'ban';
    running = true;
    modIpc
      .massban(account, login, logins, secs, unban ? '' : reason, unban)
      .then((n) => toast.success(`Done: ${formatNumber(n)} of ${formatNumber(logins.length)}.`))
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (running = false));
  }

  function stop() {
    if (channelId) void modIpc.stopMassban(channelId);
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
        {amount > 0 ? 'Time out' : 'Ban'} {logins.length || ''}
      </Button>
      {#if confirmUnban}
        <span class="ask">Unban {formatNumber(logins.length)}?</span>
        <Button small variant="primary" onclick={() => go(true)}>Unban</Button>
        <Button small variant="ghost" onclick={() => (confirmUnban = false)}>Cancel</Button>
      {:else}
        <Button small variant="ghost" disabled={logins.length === 0} onclick={() => (confirmUnban = true)}>Unban</Button>
      {/if}
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
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
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
    align-items: center;
    gap: 8px;
  }
  .ask {
    font-size: 0.82rem;
    color: var(--text);
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
