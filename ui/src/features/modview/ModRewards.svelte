<!-- V107-R43 Reward requests, as in Twitch's mod view: the channel's channel points
     redemptions that wait, newest first, by reward. Complete or reject (the points go back)
     one, the ticked ones or all shown, paced, with Stop: the stream manager's queue
     (RedemptionQueue) on a channel the account moderates. Reads again by itself when a
     redemption comes in or is settled (the chat's live points feed), and on Refresh. Bits
     power-ups don't show: they can't be refunded. Needs the sign-in with a code (GQL). -->
<script lang="ts">
  import EmptyState from '../../components/EmptyState.svelte';
  import GateFlag from '../../components/GateFlag.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import Card from '../../components/PanelCard.svelte';
  import Select from '../../components/Select.svelte';
  import type { SmRewards } from '../../ipc/bindings/SmRewards';
  import { modRewardsIpc } from '../../ipc/rewards';
  import { accounts } from '../../lib/accounts.svelte';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import RedemptionQueue from '../streammanager/RedemptionQueue.svelte';
  import { type QueueSource, rewardOptions } from '../streammanager/rewardsLogic';
  import SmBar from '../streammanager/SmBar.svelte';
  import SmProblem from '../streammanager/SmProblem.svelte';

  let { account, login }: { account: string; login: string } = $props();

  const me = $derived(accounts.list.find((a) => a.userId === account));
  const can = $derived(Boolean(me?.gql));

  let data = $state<SmRewards | null>(null);
  let loading = $state(true);
  let failed = $state('');
  let filter = $state('');
  let pulse = $state(0);
  let queue = $state<RedemptionQueue>();
  let countsLater: ReturnType<typeof setTimeout> | undefined;

  // svelte-ignore state_referenced_locally
  const source: QueueSource = {
    queue: (reward, cursor) => modRewardsIpc.queue(account, login, reward, cursor),
    setStatus: (ids, status) => modRewardsIpc.setStatus(account, login, ids, status),
    channel: login,
  };

  const rewards = $derived(data?.rewards ?? []);
  const options = $derived(rewardOptions(rewards));
  const total = $derived(rewards.reduce((n, r) => n + r.waiting, 0));
  const more = $derived(rewards.some((r) => r.waitingMore));

  /** Only the newest read lands. */
  let seq = 0;
  async function load() {
    const mine = ++seq;
    loading = true;
    try {
      const r = await modRewardsIpc.list(account, login);
      if (mine !== seq) return;
      data = r;
      failed = '';
      if (filter && !r.rewards.some((x) => x.id === filter)) filter = '';
    } catch (e) {
      if (mine === seq) failed = errorText(e);
    } finally {
      if (mine === seq) loading = false;
    }
  }

  $effect(() => {
    if (can) void load();
  });

  // Live: a redemption on this channel came in or was settled; the counts follow quietly.
  $effect(() => {
    const off = modRewardsIpc.onMoved((l) => {
      if (l !== login.toLowerCase()) return;
      pulse++;
      clearTimeout(countsLater);
      countsLater = setTimeout(() => void load(), 1500);
    });
    return () => {
      clearTimeout(countsLater);
      void off.then((f) => f());
    };
  });

  function refresh() {
    queue?.reload();
    void load();
  }
</script>

<Card title="Reward requests" flush>
  {#snippet tools()}
    {#if can}<IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={refresh} />{/if}
  {/snippet}
  {#if !can}
    <p class="gate"><GateFlag kind="gql" /><span>Reward requests need the sign-in with a code.</span></p>
  {:else if loading && !data}
    <Loading rows={4} label="Loading rewards" />
  {:else if failed && !data}
    <SmProblem text={failed} onretry={() => void load()} />
  {:else if !data?.enabled}
    <EmptyState icon="gift" text="Channel points are off on this channel." />
  {:else}
    <SmBar>
      <div class="filters">
        <Select label="Which reward" width="100%" value={filter} {options} onchange={(id) => (filter = id)} />
        {#if total}<span class="count">{formatNumber(total)}{more ? '+' : ''} waiting</span>{/if}
      </div>
    </SmBar>
    <RedemptionQueue bind:this={queue} {filter} {source} {pulse} onchanged={() => void load()} />
  {/if}
</Card>

<style>
  .gate {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 10px;
    color: var(--muted);
    font-size: 0.78rem;
  }
  .filters > :global(:first-child) {
    flex: 1;
    min-width: 0;
  }
</style>
