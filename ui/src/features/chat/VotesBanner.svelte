<!-- C4 The chat's poll and prediction above the messages: results as they come in,
     voting and betting when signed in. Can be folded, hidden into the events button while
     it runs (V103-04), or closed until the next one. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Outcome } from '../../ipc/bindings/Outcome';
  import { chatxIpc } from '../../ipc/chatx';
  import { formatDuration, formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { pollKey, pollLive, predictionKey, predictionLive, shows } from './eventStrip';
  import { eventStrip } from './eventStrip.svelte';
  import { pollLeft, predictionLeft, votes } from './votes.svelte';

  let { channel, view, signedIn }: { channel: string; view: () => number; signedIn: boolean } = $props();

  const v = $derived(votes.byChannel.get(channel));
  const poll = $derived(v?.poll && !['ARCHIVED', 'MODERATED'].includes(v.poll.status) ? v.poll : null);
  const pred = $derived(v?.prediction && v.prediction.status !== 'CANCELED' ? v.prediction : null);

  const strip = $derived(eventStrip.of(channel));

  let now = $state(Date.now());
  let folded = $state(false);
  let busy = $state(false);
  /** A paid extra vote waiting for a yes. */
  let extra = $state<string | null>(null);
  let bet = $state(100);

  $effect(() => {
    if (poll?.status !== 'ACTIVE' && pred?.status !== 'ACTIVE') return;
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  const pct = (part: number, total: number) => (total ? Math.round((part / total) * 100) : 0);

  async function act(run: () => Promise<void>) {
    busy = true;
    try {
      await run();
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  function vote(choice: string) {
    if (!poll || poll.status !== 'ACTIVE' || !signedIn) return;
    if (poll.mine.length && poll.pointsCost) {
      extra = choice;
      return;
    }
    void act(() => chatxIpc.vote(view(), choice));
  }

  function voteExtra() {
    const choice = extra;
    extra = null;
    if (choice && poll?.pointsCost) void act(() => chatxIpc.vote(view(), choice, poll.pointsCost ?? undefined));
  }

  function predict(o: Outcome) {
    if (bet < 1) return;
    void act(() => chatxIpc.predict(view(), o.id, Math.floor(bet)));
  }

  const PRED_STATUS: Record<string, string> = {
    LOCKED: 'Locked',
    RESOLVE_PENDING: 'Waiting for the result',
    CANCEL_PENDING: 'Being canceled',
  };
</script>

{#snippet head(icon: 'poll' | 'points', title: string, info: string, key: string, live: boolean)}
  <div class="head">
    <Icon name={icon} size={14} />
    <button class="title" onclick={() => (folded = !folded)} aria-expanded={!folded}>{title}</button>
    <span class="info">{info}</span>
    {#if live}
      <button
        class="close"
        aria-label="Hide"
        use:tooltip={{ label: 'Hide', hint: 'Tucks it into the events button.' }}
        onclick={() => eventStrip.hide(channel, { key, live })}
      >
        <Icon name="chevronUp" size={13} />
      </button>
    {/if}
    <button class="close" aria-label="Close" use:tooltip={'Close'} onclick={() => eventStrip.close(channel, key)}>
      <Icon name="x" size={13} />
    </button>
  </div>
{/snippet}

{#if poll && shows(strip, pollKey(poll.id))}
  {@const left = pollLeft(v!, now)}
  {@const total = poll.totalVotes}
  <div class="box" class:folded>
    {@render head(
      'poll',
      poll.title,
      poll.status === 'ACTIVE' ? formatDuration(left) : 'Ended',
      pollKey(poll.id),
      pollLive(poll.status),
    )}
    {#if !folded}
      {@const top = Math.max(...poll.choices.map((c) => c.votes))}
      <div class="rows">
        {#each poll.choices as c, i (c.id)}
          <button
            class="row"
            class:mine={poll.mine.includes(c.id)}
            class:won={poll.status !== 'ACTIVE' && c.votes === top && top > 0}
            disabled={busy || poll.status !== 'ACTIVE' || !signedIn}
            onclick={() => vote(c.id)}
            style:--fill="{pct(c.votes, total)}%"
          >
            <span class="n">{i + 1}</span>
            <span class="label">{c.title}</span>
            <span class="num">{pct(c.votes, total)}% · {formatNumber(c.votes)}</span>
          </button>
        {/each}
      </div>
      {#if extra}
        <div class="confirm">
          <span>Another vote costs {formatNumber(poll.pointsCost ?? 0)} points.</span>
          <Button small variant="primary" onclick={voteExtra}>Vote</Button>
          <Button small variant="ghost" onclick={() => (extra = null)}>Cancel</Button>
        </div>
      {/if}
      <p class="foot">
        {formatNumber(poll.totalVoters)} voted{#if poll.pointsCost} · Extra votes {formatNumber(poll.pointsCost)} points{/if}
      </p>
    {/if}
  </div>
{/if}

{#if pred && shows(strip, predictionKey(pred.id))}
  {@const left = predictionLeft(v!, now)}
  {@const total = pred.outcomes.reduce((s, o) => s + o.points, 0)}
  {@const open = pred.status === 'ACTIVE' && left > 0}
  {@const winner = pred.outcomes.find((o) => o.id === pred.winner)}
  <div class="box" class:folded>
    {@render head(
      'points',
      pred.title,
      open ? formatDuration(left) : winner ? `${winner.title} won` : (PRED_STATUS[pred.status] ?? 'Closed'),
      predictionKey(pred.id),
      predictionLive(pred.status),
    )}
    {#if !folded}
      <div class="rows">
        {#each pred.outcomes as o (o.id)}
          <div
            class="row out"
            class:pink={o.color === 'PINK'}
            class:won={o.id === pred.winner}
            class:mine={pred.mine?.outcome === o.id}
            style:--fill="{pct(o.points, total)}%"
          >
            <span class="label">{o.title}</span>
            <span class="num">
              {pct(o.points, total)}% · {formatNumber(o.points)} · 1:{o.points ? (total / o.points).toFixed(2) : '–'}
            </span>
            {#if open && signedIn && (!pred.mine || pred.mine.outcome === o.id)}
              <Button small disabled={busy} onclick={() => predict(o)}>Bet</Button>
            {/if}
          </div>
          {#if o.top}<p class="top">Top: {o.top[0]} {formatNumber(o.top[1])}</p>{/if}
        {/each}
      </div>
      {#if pred.mine}
        <p class="foot">
          You bet {formatNumber(pred.mine.points)} on {pred.outcomes.find((o) => o.id === pred.mine?.outcome)?.title ?? '?'}{#if pred.mine.result === 'WIN'}
            · won {formatNumber(pred.mine.won ?? 0)}{:else if pred.mine.result === 'LOSE'} · lost{:else if pred.mine.result === 'REFUND'}
            · refunded{/if}
        </p>
      {/if}
      {#if open && signedIn}
        <label class="foot amount">
          Points
          <input type="number" min="1" max={v?.balance ?? undefined} bind:value={bet} />
          {#if v?.balance !== null && v?.balance !== undefined}<span>of {formatNumber(v.balance)}</span>{/if}
        </label>
        {#if !v?.accepted}<p class="foot">Betting accepts Twitch’s prediction terms.</p>{/if}
      {/if}
    {/if}
  </div>
{/if}

<style>
  .box {
    margin: 6px 8px 2px;
    padding: 6px 8px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    font-size: 0.82rem;
    --blue: #387aff;
    --pink: #f5009b;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }
  .title {
    flex: 1;
    min-width: 0;
    padding: 0;
    background: none;
    border: 0;
    color: var(--text);
    font-weight: 600;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .info {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .close {
    display: grid;
    padding: 2px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .close:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-top: 6px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 26px;
    padding: 2px 6px;
    background: linear-gradient(to right, var(--accent-wash) var(--fill), transparent var(--fill));
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
  }
  button.row:not(:disabled):hover {
    border-color: var(--accent-border);
  }
  .row:disabled {
    opacity: 1;
    cursor: default;
  }
  .out {
    background: linear-gradient(
      to right,
      color-mix(in srgb, var(--blue) 22%, transparent) var(--fill),
      transparent var(--fill)
    );
  }
  .out.pink {
    background: linear-gradient(
      to right,
      color-mix(in srgb, var(--pink) 22%, transparent) var(--fill),
      transparent var(--fill)
    );
  }
  .mine {
    border-color: var(--accent-border);
  }
  .won {
    font-weight: 700;
  }
  .won .label::after {
    content: ' ✓';
    color: var(--green);
  }
  .n {
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .num {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .top {
    margin: 0 0 0 6px;
    color: var(--dim);
    font-size: 0.75rem;
  }
  .confirm {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
  }
  .confirm span {
    flex: 1;
  }
  .foot {
    margin: 5px 0 0;
    color: var(--muted);
    font-size: 0.76rem;
  }
  .amount {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .amount input {
    width: 90px;
    padding: 2px 6px;
  }
</style>
