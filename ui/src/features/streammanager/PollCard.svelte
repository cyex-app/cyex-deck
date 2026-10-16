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
  import { errorText } from '../../lib/errors';
  import Select from '../../components/Select.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import { openMenuAt } from '../../components/menu.svelte';

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
        toast.success('Poll started.');
        votes.watch(login);
      })
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (busy = false));
  }

  function end(archive: boolean) {
    if (!poll) return;
    busy = true;
    smIpc
      .endPoll(poll.id, archive)
      .then(() => votes.watch(login))
      .catch((e) => toast.error(errorText(e)))
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
        <IconButton
          icon="more"
          label="More"
          size={14}
          disabled={busy}
          onclick={(e) => openMenuAt(e, [{ label: 'End and hide', icon: 'x', run: () => end(true) }])}
        />
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
          <IconButton icon="x" label="Remove choice" size={14} onclick={() => (choices = choices.filter((_, j) => j !== i))} />
        {/if}
      </div>
    {/each}
    <div class="line">
      {#if choices.length < 5}
        <Button small variant="link" onclick={() => (choices = [...choices, ''])}>Add a choice</Button>
      {/if}
      <span class="grow"></span>
      <Select
        label="How long"
        width="110px"
        value={String(duration)}
        options={DURATIONS.map((d) => ({ value: String(d), label: formatDuration(d) }))}
        onchange={(v) => (duration = Number(v))}
      />
    </div>
    <div class="line">
      <Checkbox label="Extra votes for channel points" checked={extra} onchange={(on) => (extra = on)} />
      {#if extra}
        <NumberField label="Points per vote" min={1} max={1000000} width="110px" value={cost} onchange={(n) => (cost = n)} />
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
    flex: none;
    white-space: nowrap;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .grow {
    flex: 1;
  }
</style>
