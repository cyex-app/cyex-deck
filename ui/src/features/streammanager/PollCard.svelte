<!-- 1.35 Start a poll and watch it run: live results, end it early or hide it. Results
     come from the chat's vote watcher (GQL, then Hermes); starting and ending
     go GQL first, Helix otherwise (D30). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import { smIpc } from '../../ipc/streammanager';
  import { formatDuration, formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { pollLeft, votes } from '../chat/votes.svelte';
  import Card from '../../components/PanelCard.svelte';

  const DURATIONS = [60, 120, 180, 300, 600, 900];

  let { login }: { login: string } = $props();

  let title = $state('');
  let choices = $state(['', '']);
  let duration = $state(120);
  let extra = $state(false);
  let cost = $state(100);
  let busy = $state(false);
  let now = $state(Date.now());

  $effect(() => votes.watch(login));
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  const v = $derived(votes.byChannel.get(login));
  const poll = $derived(v?.poll ?? null);
  const active = $derived(poll?.status === 'ACTIVE');
  const left = $derived(v ? pollLeft(v, now) : 0);
  const ready = $derived(!!title.trim() && choices.filter((c) => c.trim()).length >= 2);

  function start() {
    busy = true;
    smIpc
      .createPoll({ title, choices, duration, pointsPerVote: extra ? cost : null })
      .then(() => {
        title = '';
        choices = ['', ''];
        toast.info('Poll started.');
        votes.watch(login);
      })
      .catch((e) => toast.error(String(e)))
      .finally(() => (busy = false));
  }

  function end(archive: boolean) {
    if (!poll) return;
    busy = true;
    smIpc
      .endPoll(poll.id, archive)
      .then(() => votes.watch(login))
      .catch((e) => toast.error(String(e)))
      .finally(() => (busy = false));
  }

  const pct = (n: number, total: number) => (total ? Math.round((n * 100) / total) : 0);
</script>

<Card title="Poll">
  {#if poll}
    <p class="q">{poll.title}</p>
    <p class="note">
      {active ? `${formatDuration(left)} left` : 'Ended'} · {formatNumber(poll.totalVotes)} votes
    </p>
    <ul class="results">
      {#each poll.choices as c (c.id)}
        {@const p = pct(c.votes, poll.totalVotes)}
        <li>
          <div class="bar" style:width="{p}%"></div>
          <span class="name">{c.title}</span>
          <span class="n">{p}% · {formatNumber(c.votes)}</span>
        </li>
      {/each}
    </ul>
    <div class="line">
      {#if active}
        <Button small disabled={busy} onclick={() => end(false)}>End now</Button>
        <Button small variant="ghost" disabled={busy} onclick={() => end(true)}>End and hide</Button>
      {:else}
        <Button small disabled={busy} onclick={() => end(true)}>Hide and start a new one</Button>
      {/if}
    </div>
  {:else}
    <input class="field" placeholder="Question" maxlength="60" bind:value={title} />
    {#each choices as _, i (i)}
      <div class="line">
        <input class="field grow" placeholder="Choice {i + 1}" maxlength="25" bind:value={choices[i]} />
        {#if choices.length > 2}
          <button class="x" aria-label="Remove choice" onclick={() => (choices = choices.filter((_, j) => j !== i))}>×</button>
        {/if}
      </div>
    {/each}
    <div class="line">
      {#if choices.length < 5}
        <button class="link" onclick={() => (choices = [...choices, ''])}>Add a choice</button>
      {/if}
      <span class="grow"></span>
      <select class="field small" bind:value={duration} aria-label="How long">
        {#each DURATIONS as d (d)}<option value={d}>{formatDuration(d)}</option>{/each}
      </select>
    </div>
    <div class="line">
      <Checkbox label="Extra votes for channel points" checked={extra} onchange={(on) => (extra = on)} />
      {#if extra}
        <input class="field num" type="number" min="1" max="1000000" bind:value={cost} aria-label="Points per vote" />
      {/if}
    </div>
    <div class="line">
      <Button small variant="primary" disabled={!ready || busy} onclick={start}>Start poll</Button>
    </div>
  {/if}
</Card>

<style>
  .q {
    margin: 0;
    font-weight: 700;
  }
  .results {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .results li {
    position: relative;
    display: flex;
    gap: 8px;
    padding: 5px 8px;
    background: var(--bg);
    border-radius: var(--radius);
    overflow: hidden;
    font-size: 0.82rem;
  }
  .bar {
    position: absolute;
    inset: 0 auto 0 0;
    background: var(--accent-dim);
  }
  .name,
  .n {
    position: relative;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .n {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .grow {
    flex: 1;
  }
  .small {
    height: 26px;
    padding: 0 6px;
  }
  .num {
    width: 96px;
    height: 26px;
  }
  .x,
  .link {
    background: none;
    border: 0;
    padding: 0 4px;
    color: var(--muted);
    cursor: pointer;
  }
  .x {
    font-size: 1rem;
  }
  .link {
    font-size: 0.8rem;
  }
  .x:hover,
  .link:hover {
    color: var(--text);
  }
</style>
