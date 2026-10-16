<!-- Miner settings (08): what goes first, what it claims, watching in the app (W1),
     community goals (W2), sitting in chats, and prediction bets (off by default). The
     channels themselves are in the miner. -->
<script lang="ts">
  import Checkbox from '../../../components/Checkbox.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { Bets } from '../../../ipc/bindings/Bets';
  import type { ChatPresence } from '../../../ipc/bindings/ChatPresence';
  import type { DelayMode } from '../../../ipc/bindings/DelayMode';
  import type { FilterBy } from '../../../ipc/bindings/FilterBy';
  import type { FilterCmp } from '../../../ipc/bindings/FilterCmp';
  import type { Miner } from '../../../ipc/bindings/Miner';
  import type { MinerRule } from '../../../ipc/bindings/MinerRule';
  import type { Strategy } from '../../../ipc/bindings/Strategy';
  import { settings } from '../../../lib/settings.svelte';
  import Select from '../../../components/Select.svelte';

  const m = $derived(settings.v.miner);
  const b = $derived(m.bets);
  const save = (patch: Partial<Miner>) => void settings.patch('miner', patch);
  const bet = (patch: Partial<Bets>) => save({ bets: { ...b, ...patch } });

  const RULES: { value: MinerRule; label: string; hint: string }[] = [
    { value: 'streak', label: 'Watch streaks', hint: 'Streams where the streak isn’t earned yet.' },
    { value: 'drops', label: 'Drops', hint: 'Channels with drops running.' },
    { value: 'multiplier', label: 'Point multipliers', hint: 'Channels where you earn more, e.g. as a sub.' },
    { value: 'fewestPoints', label: 'Fewest points', hint: 'Catch up where you have little.' },
    { value: 'mostPoints', label: 'Most points', hint: 'Keep growing the biggest balance.' },
  ];
  const rules = $derived([
    ...m.order.map((v) => RULES.find((r) => r.value === v)!),
    ...RULES.filter((r) => !m.order.includes(r.value)),
  ]);

  function toggleRule(v: MinerRule, on: boolean) {
    save({ order: on ? [...m.order, v] : m.order.filter((x) => x !== v) });
  }

  function moveRule(v: MinerRule, by: number) {
    const order = [...m.order];
    const at = order.indexOf(v);
    const to = at + by;
    if (at < 0 || to < 0 || to >= order.length) return;
    [order[at], order[to]] = [order[to], order[at]];
    save({ order });
  }

  const PRESENCE: { value: ChatPresence; label: string }[] = [
    { value: 'never', label: 'Never' },
    { value: 'live', label: 'While live' },
    { value: 'always', label: 'Always' },
  ];
  const STRATEGIES: { value: Strategy; label: string }[] = [
    { value: 'smart', label: 'Most voters, unless it’s close' },
    { value: 'mostVoted', label: 'Most voters' },
    { value: 'highOdds', label: 'Best payout' },
    { value: 'mostPoints', label: 'Most points' },
    { value: 'smartMoney', label: 'Where the biggest bet is' },
    { value: 'outcome', label: 'Always the same side' },
  ];
  const DELAYS: { value: DelayMode; label: string }[] = [
    { value: 'beforeEnd', label: 'Seconds before it closes' },
    { value: 'afterStart', label: 'Seconds after it opens' },
    { value: 'percent', label: 'Percent of the way in' },
  ];
  const FILTERS: { value: FilterBy; label: string }[] = [
    { value: 'none', label: 'Always bet' },
    { value: 'totalUsers', label: 'Voters in total' },
    { value: 'totalPoints', label: 'Points in total' },
    { value: 'chosenUsers', label: 'Voters on that side' },
    { value: 'chosenPoints', label: 'Points on that side' },
    { value: 'odds', label: 'Payout of that side' },
    { value: 'topPoints', label: 'Biggest bet on that side' },
  ];
  const CMPS: { value: FilterCmp; label: string }[] = [
    { value: 'above', label: 'is above' },
    { value: 'atLeast', label: 'is at least' },
    { value: 'below', label: 'is below' },
    { value: 'atMost', label: 'is at most' },
  ];

  /** A whole number from a field, kept between `lo` and `hi`. */
  const num = (e: Event, lo: number, hi: number) =>
    Math.min(hi, Math.max(lo, Math.round(Number((e.currentTarget as HTMLInputElement).value) || lo)));
</script>

<h2>Miner</h2>

<h3 class="label" data-setting="miner.order">What goes first</h3>
<p class="note">
  Twitch counts two channels at a time. After the two you star in the miner, these decide which; then the list order.
</p>
{#each rules as r (r.value)}
  {@const on = m.order.includes(r.value)}
  <div class="rule" class:off={!on}>
    <Checkbox label={r.label} checked={on} onchange={(v) => toggleRule(r.value, v)} />
    <span class="hint">{r.hint}</span>
    {#if on}
      <IconButton icon="chevronUp" label="Earlier" hint="Rules higher up win." size={14} onclick={() => moveRule(r.value, -1)} />
      <IconButton icon="chevronDown" label="Later" size={14} onclick={() => moveRule(r.value, 1)} />
    {/if}
  </div>
{/each}

<h3 class="label">Claims</h3>
<Row setting="miner.claimInTabs" label="Bonus chests in open tabs" hint="With each tab’s account, not only in mined channels.">
  <Toggle label="Bonus chests in open tabs" checked={m.claimInTabs} onchange={(v) => save({ claimInTabs: v })} />
</Row>
<Row setting="miner.claimMoments" label="Moments">
  <Toggle label="Moments" checked={m.claimMoments} onchange={(v) => save({ claimMoments: v })} />
</Row>
<Row setting="miner.joinRaids" label="Join raids" hint="Goes along with a raid for its points.">
  <Toggle label="Join raids" checked={m.joinRaids} onchange={(v) => save({ joinRaids: v })} />
</Row>
<Row setting="miner.claimDrops" label="Drops" hint="Twitch wants the game account linked first.">
  <Toggle label="Drops" checked={m.claimDrops} onchange={(v) => save({ claimDrops: v })} />
</Row>

<h3 class="label">More</h3>
<Row
  setting="miner.watchingEarns"
  label="Watching here earns points"
  hint="Streams you watch in the app count like on twitch.tv, drops included. The miner uses the slots left."
>
  <Toggle label="Watching here earns points" checked={m.watchingEarns} onchange={(v) => save({ watchingEarns: v })} />
</Row>
<Row setting="miner.goals" label="Community goals" hint="Puts points toward a channel’s goals on its own. Spends points.">
  <Toggle label="Community goals" checked={m.goals} onchange={(v) => save({ goals: v })} />
</Row>
<Row setting="miner.chat" label="Sit in their chats" hint="Some channel bots reward people in chat. Each chat counts toward the 100.">
  <Segmented label="Sit in their chats" options={PRESENCE} value={m.chat} onchange={(chat) => save({ chat })} />
</Row>

<h3 class="label">Predictions</h3>
<Row setting="miner.bets.enabled" label="Bet on predictions" hint="Spends points. The first bet accepts Twitch’s prediction terms.">
  <Toggle label="Bet on predictions" checked={b.enabled} onchange={(enabled) => bet({ enabled })} />
</Row>
{#if b.enabled}
  <Row setting="miner.bets.strategy" label="Pick">
    <Select label="Pick" width="260px" value={b.strategy} options={STRATEGIES} onchange={(v) => bet({ strategy: v as Strategy })} />
  </Row>
  {#if b.strategy === 'outcome'}
    <Row setting="miner.bets.outcome" label="Side" hint="1 is the first (blue) side.">
      <input class="field num" type="number" min="1" max="10" value={b.outcome} onchange={(e) => bet({ outcome: num(e, 1, 10) })} />
    </Row>
  {/if}
  {#if b.strategy === 'smart'}
    <Row setting="miner.bets.gap" label="Close is under" hint="Gap between the two sides’ voters, in percent. Close bets go to the better payout.">
      <input class="field num" type="number" min="0" max="100" value={b.gap} onchange={(e) => bet({ gap: num(e, 0, 100) })} />
    </Row>
  {/if}
  <Row setting="miner.bets.percent" label="Bet" hint="Percent of your balance.">
    <input class="field num" type="number" min="1" max="100" value={b.percent} onchange={(e) => bet({ percent: num(e, 1, 100) })} />
  </Row>
  <Row setting="miner.bets.maxPoints" label="At most" hint="Points per bet.">
    <input class="field num" type="number" min="10" value={b.maxPoints} onchange={(e) => bet({ maxPoints: num(e, 10, 1e9) })} />
  </Row>
  <Row setting="miner.bets.keep" label="Keep" hint="No bets while the balance is at or under this.">
    <input class="field num" type="number" min="0" value={b.keep} onchange={(e) => bet({ keep: num(e, 0, 1e9) })} />
  </Row>
  <Row setting="miner.bets.stealth" label="Stay small" hint="Never bet more than the biggest bet on that side.">
    <Toggle label="Stay small" checked={b.stealth} onchange={(stealth) => bet({ stealth })} />
  </Row>
  <Row setting="miner.bets.delay" label="When">
    <span class="pair">
      <input
        class="field num"
        type="number"
        min="0"
        max={b.delayMode === 'percent' ? 100 : 1800}
        value={b.delay}
        onchange={(e) => bet({ delay: num(e, 0, b.delayMode === 'percent' ? 100 : 1800) })}
      />
      <Select label="When" width="220px" value={b.delayMode} options={DELAYS} onchange={(v) => bet({ delayMode: v as DelayMode })} />
    </span>
  </Row>
  <Row setting="miner.bets.filter" label="Only if">
    <span class="pair">
      <Select
        label="Only if"
        width="220px"
        value={b.filter.by}
        options={FILTERS}
        onchange={(v) => bet({ filter: { ...b.filter, by: v as FilterBy } })}
      />
      {#if b.filter.by !== 'none'}
        <Select
          label="Compared"
          width="130px"
          value={b.filter.cmp}
          options={CMPS}
          onchange={(v) => bet({ filter: { ...b.filter, cmp: v as FilterCmp } })}
        />
        <input
          class="field num"
          type="number"
          step="any"
          aria-label="Value"
          value={b.filter.value}
          onchange={(e) => bet({ filter: { ...b.filter, value: Number(e.currentTarget.value) || 0 } })}
        />
      {/if}
    </span>
  </Row>
{/if}

<style>
  .note {
    margin: 0 0 6px;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .rule {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 38px;
    font-size: 0.85rem;
  }
  .rule .hint {
    flex: 1;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .rule.off .hint {
    opacity: 0.7;
  }
  .num {
    width: 90px;
  }
  .pair {
    display: inline-flex;
    gap: 6px;
  }
</style>
