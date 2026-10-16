<!-- 1.35 Start a prediction, lock betting, pay out the winning side or refund everyone.
     Paying out and refunding take a second click. Live state comes from the chat's vote
     watcher; the actions go GQL first, Helix otherwise (D30). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { SmPredictionEnd } from '../../ipc/bindings/SmPredictionEnd';
  import { smIpc } from '../../ipc/streammanager';
  import { formatDuration, formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { predictionLeft, votes } from '../chat/votes.svelte';
  import Card from '../../components/PanelCard.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import { shares } from './logic';
  import { errorText } from '../../lib/errors';
  import Select from '../../components/Select.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenuAt } from '../../components/menu.svelte';

  const WINDOWS = [30, 60, 120, 300, 600, 1800];

  let { login }: { login: string } = $props();

  let title = $state('');
  let outcomes = $state(['', '']);
  let betWindow = $state(120);
  let busy = $state(false);
  let now = $state(Date.now());

  $effect(() => votes.watch(login));
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  const v = $derived(votes.byChannel.get(login));
  const p = $derived(v?.prediction ?? null);
  const open = $derived(p && ['ACTIVE', 'LOCKED', 'RESOLVE_PENDING', 'CANCEL_PENDING'].includes(p.status));
  const left = $derived(v ? predictionLeft(v, now) : 0);
  const pcts = $derived(p ? shares(p.outcomes.map((o) => o.points)) : []);
  const ready = $derived(!!title.trim() && outcomes.filter((o) => o.trim()).length >= 2);

  function start() {
    busy = true;
    smIpc
      .createPrediction({ title, outcomes, window: betWindow })
      .then(() => {
        title = '';
        outcomes = ['', ''];
        toast.success('Prediction started.');
        votes.watch(login);
      })
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (busy = false));
  }

  function end(how: SmPredictionEnd, done: string) {
    if (!p) return;
    busy = true;
    smIpc
      .endPrediction(p.id, how)
      .then(() => {
        toast.success(done);
        votes.watch(login);
      })
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (busy = false));
  }

  const STATUS: Record<string, string> = {
    LOCKED: 'Betting closed. Pick the winner.',
    RESOLVE_PENDING: 'Paying out…',
    CANCEL_PENDING: 'Refunding…',
    RESOLVED: 'Paid out.',
    CANCELED: 'Refunded.',
  };
</script>

<Card title="Prediction">
  {#if p && open}
    <p class="q">{p.title}</p>
    <p class="note">{p.status === 'ACTIVE' ? `Betting closes in ${formatDuration(left)}` : STATUS[p.status]}</p>
    <ul class="outcomes">
      {#each p.outcomes as o, i (o.id)}
        <li class:blue={i === 0} class:pink={i === 1 && p.outcomes.length === 2}>
          <div class="top">
            <span class="name">{o.title}</span>
            <span class="pct">{pcts[i]}%</span>
          </div>
          <div class="sub">
            {formatNumber(o.points)} points · {formatNumber(o.users)} people
          </div>
          {#if p.status === 'LOCKED'}
            <ConfirmButton
              label="Winner"
              confirm="Click again to pay out"
              disabled={busy}
              onconfirm={() => end({ kind: 'resolve', winner: o.id }, `Paid out ${o.title}.`)}
            />
          {/if}
        </li>
      {/each}
    </ul>
    <div class="line">
      {#if p.status === 'ACTIVE'}
        <Button small disabled={busy} onclick={() => end({ kind: 'lock' }, 'Betting closed.')}>Close betting</Button>
      {/if}
      {#if p.status === 'ACTIVE' || p.status === 'LOCKED'}
        <span class="wide-only">
          <ConfirmButton
            label="Refund everyone"
            confirm="Click again to refund"
            variant="ghost"
            disabled={busy}
            onconfirm={() => end({ kind: 'cancel' }, 'Refunded.')}
          />
        </span>
        <span class="narrow-only">
          <IconButton
            icon="more"
            label="More"
            size={14}
            disabled={busy}
            onclick={(e) =>
              openMenuAt(e, [
                {
                  label: 'Refund everyone',
                  icon: 'x',
                  items: [{ label: 'Refund all points', icon: 'x', danger: true, run: () => end({ kind: 'cancel' }, 'Refunded.') }],
                },
              ])}
          />
        </span>
      {/if}
    </div>
  {:else}
    {#if p}<p class="note">Last one: {p.title} · {STATUS[p.status] ?? p.status}</p>{/if}
    <input class="field" placeholder="Question" maxlength="45" bind:value={title} />
    {#each outcomes as _, i (i)}
      <div class="line">
        <input class="field grow" placeholder="Outcome {i + 1}" maxlength="25" bind:value={outcomes[i]} />
        {#if outcomes.length > 2}
          <IconButton icon="x" label="Remove outcome" size={14} onclick={() => (outcomes = outcomes.filter((_, j) => j !== i))} />
        {/if}
      </div>
    {/each}
    <div class="line">
      {#if outcomes.length < 10}
        <Button small variant="link" onclick={() => (outcomes = [...outcomes, ''])}>Add an outcome</Button>
      {/if}
      <span class="grow"></span>
      <span class="note">Betting open</span>
      <Select
        label="Betting open"
        width="110px"
        value={String(betWindow)}
        options={WINDOWS.map((w) => ({ value: String(w), label: formatDuration(w) }))}
        onchange={(v) => (betWindow = Number(v))}
      />
    </div>
    <div class="line">
      <Button small variant="primary" disabled={!ready || busy} onclick={start}>Start prediction</Button>
    </div>
  {/if}
</Card>

<style>
  .q {
    margin: 0;
    font-weight: 700;
  }
  .outcomes {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .outcomes li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 8px;
    background: var(--bg);
    border-left: 3px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .outcomes li.blue {
    border-left-color: var(--cyan);
  }
  .outcomes li.pink {
    border-left-color: var(--twitch);
  }
  .top {
    display: flex;
    gap: 8px;
    font-size: 0.85rem;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
    font-weight: 600;
  }
  .pct,
  .sub {
    font-variant-numeric: tabular-nums;
  }
  .sub {
    font-size: 0.76rem;
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
</style>
