<!-- Miner settings (08): what goes first, what it claims, watching in the app (W1),
     community goals (W2), sitting in chats, and prediction bets (off by default). The
     channels themselves are in the miner. -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
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
  import NumberField from '../../../components/NumberField.svelte';

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

</script>

<SectionHead title="Miner" level={2}>Collects channel points while you watch and claims bonus chests.</SectionHead>

<SectionHead title="What goes first" setting="miner.order" />
<p class="note">
  Twitch counts two channels at a time. After the two you star in the miner, these decide which; then the list order.
</p>
{#each rules as r (r.value)}
  {@const on = m.order.includes(r.value)}
  <div class="rule" class:off={!on}>
    <Checkbox label={r.label} checked={on} onchange={(v) => toggleRule(r.value, v)} />
    <span class="hint">{r.hint}</span>
    <!-- P6 The arrows keep their room when a rule is off, so rows don't jump. -->
    <span class="arrows" class:gone={!on} inert={!on}>
      <IconButton icon="chevronUp" label="Earlier" hint="Rules higher up win." size={14} onclick={() => moveRule(r.value, -1)} />
      <IconButton icon="chevronDown" label="Later" size={14} onclick={() => moveRule(r.value, 1)} />
    </span>
  </div>
{/each}

<SectionHead title="Claims">Bonus chests the miner picks up for you.</SectionHead>
<Row setting="miner.claimInTabs" label="Bonus chests in open tabs" hint="With each tab’s account, not only in mined channels.">
  <Toggle label="Bonus chests in open tabs" checked={m.claimInTabs} onchange={(v) => save({ claimInTabs: v })} />
</Row>
<Row setting="miner.claimMoments" label="Moments" hint="Picks up the bonus when a Moment shows up.">
  <Toggle label="Moments" checked={m.claimMoments} onchange={(v) => save({ claimMoments: v })} />
</Row>
<Row setting="miner.joinRaids" label="Join raids" hint="Goes along with a raid for its points.">
  <Toggle label="Join raids" checked={m.joinRaids} onchange={(v) => save({ joinRaids: v })} />
</Row>
<Row setting="miner.claimDrops" label="Drops" hint="Twitch wants the game account linked first.">
  <Toggle label="Drops" checked={m.claimDrops} onchange={(v) => save({ claimDrops: v })} />
</Row>

<SectionHead title="More" />
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

<SectionHead title="Predictions">Bet your points on Twitch predictions automatically.</SectionHead>
<Row setting="miner.bets.enabled" label="Bet on predictions" hint="Spends points. The first bet accepts Twitch’s prediction terms.">
  <Toggle label="Bet on predictions" checked={b.enabled} onchange={(enabled) => bet({ enabled })} />
</Row>
{#if b.enabled}
  <Row setting="miner.bets.strategy" label="Pick" hint="How the miner chooses a side.">
    <Select label="Pick" width="260px" value={b.strategy} options={STRATEGIES} onchange={(v) => bet({ strategy: v as Strategy })} />
  </Row>
  {#if b.strategy === 'outcome'}
    <Row setting="miner.bets.outcome" label="Side" hint="1 is the first (blue) side.">
      <NumberField label="Side" min={1} max={10} width="90px" value={b.outcome} onchange={(n) => bet({ outcome: n })} />
    </Row>
  {/if}
  {#if b.strategy === 'smart'}
    <Row setting="miner.bets.gap" label="Close is under" hint="Gap between the two sides’ voters, in percent. Close bets go to the better payout.">
      <NumberField label="Close is under" min={0} max={100} width="90px" unit="%" value={b.gap} onchange={(n) => bet({ gap: n })} />
    </Row>
  {/if}
  <Row setting="miner.bets.percent" label="Bet" hint="Percent of your balance.">
    <NumberField label="Bet" min={1} max={100} width="90px" unit="%" value={b.percent} onchange={(n) => bet({ percent: n })} />
  </Row>
  <Row setting="miner.bets.maxPoints" label="At most" hint="Points per bet.">
    <NumberField label="At most" min={10} max={1e9} width="110px" value={b.maxPoints} onchange={(n) => bet({ maxPoints: n })} />
  </Row>
  <Row setting="miner.bets.keep" label="Keep" hint="No bets while the balance is at or under this.">
    <NumberField label="Keep" min={0} max={1e9} width="110px" value={b.keep} onchange={(n) => bet({ keep: n })} />
  </Row>
  <Row setting="miner.bets.stealth" label="Stay small" hint="Never bet more than the biggest bet on that side.">
    <Toggle label="Stay small" checked={b.stealth} onchange={(stealth) => bet({ stealth })} />
  </Row>
  <Row setting="miner.bets.delay" label="When" hint="How long after a prediction opens to bet.">
    <span class="pair">
      <NumberField label="Delay" min={0} max={b.delayMode === 'percent' ? 100 : 1800} width="90px" value={b.delay} onchange={(n) => bet({ delay: n })} />
      <Select label="When" width="220px" value={b.delayMode} options={DELAYS} onchange={(v) => bet({ delayMode: v as DelayMode })} />
    </span>
  </Row>
  <Row setting="miner.bets.filter" label="Only if" hint="Skips predictions that don’t fit.">
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
        <NumberField label="Value" min={-1e9} max={1e9} step={0.01} width="110px" value={b.filter.value} onchange={(n) => bet({ filter: { ...b.filter, value: n } })} />
      {/if}
    </span>
  </Row>
{/if}

<style>
  /* rb-9 A note: the hint size. */
  .note {
    margin: 0 0 6px;
    color: var(--muted);
    font-size: var(--text-hint);
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
    font-size: var(--text-hint);
  }
  .arrows {
    display: inline-flex;
    flex: none;
  }
  .arrows.gone {
    visibility: hidden;
  }
  .rule.off .hint {
    opacity: 0.7;
  }
  .pair {
    display: inline-flex;
    gap: 6px;
  }
</style>
