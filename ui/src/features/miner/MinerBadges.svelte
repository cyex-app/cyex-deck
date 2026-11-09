<!-- V110-U68 Badges the miner can earn: each open campaign with its art, game,
     progress and end date, a tick to pick it, and "All" to earn every one on its own.
     Earned ones fold away below. -->
<script lang="ts">
  import Checkbox from '../../components/Checkbox.svelte';
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { minerIpc } from '../../ipc/miner';
  import { errorText } from '../../lib/errors';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { endDate, percent, progress } from './badges';
  import { miner } from './miner.svelte';

  const auto = $derived(settings.v.miner.badges?.auto ?? true);
  const list = $derived(miner.badges);
  const earned = $derived(miner.earned);
  let fold = $state(false);

  async function run(job: Promise<void>) {
    try {
      await job;
    } catch (e) {
      toast.error(errorText(e));
    }
  }
</script>

<section class="badges">
  <h2>
    Badges <span class="count">{list.length}</span>
    <span class="grow"></span>
    <Checkbox label="All" checked={auto} onchange={(on) => void run(minerIpc.badgesAuto(on))} />
  </h2>
  {#each list as b (b.campaignId)}
    <div class="badge" class:live={miner.focus === b.campaignId}>
      {#if b.image}<img src={b.image} alt="" width="28" height="28" loading="lazy" />{:else}<span class="art"></span>{/if}
      <div class="who">
        <Checkbox
          label={b.name}
          hint="Earn this badge"
          ellipsis
          checked={b.picked}
          disabled={auto}
          onchange={(on) => void run(minerIpc.badgePick(b.campaignId, on))}
        />
        <span class="sub">
          {b.game}
          {#if b.helperChannels}<span use:tooltip={'Channels outside your list, watched only for this badge.'}>{` · ${b.helperChannels} extra`}</span>{/if}
        </span>
      </div>
      <span class="bar"><span style:width="{percent(b)}%"></span></span>
      <span class="mins">{progress(b)}</span>
      <span class="ends" use:tooltip={'Last day to earn it'}>{endDate(b.endsAt)}</span>
    </div>
  {:else}
    <p class="empty">No badges to earn right now. The miner looks again every half hour.</p>
  {/each}

  {#if earned.length}
    <button class="fold" aria-expanded={fold} onclick={() => (fold = !fold)}>
      <Icon name={fold ? 'chevronDown' : 'chevronRight'} size={13} />Earned this session <span class="count">{earned.length}</span>
    </button>
    {#if fold}
      {#each earned as e (e.campaignId)}
        <div class="badge done">
          {#if e.image}<img src={e.image} alt="" width="28" height="28" loading="lazy" />{:else}<span class="art"></span>{/if}
          <div class="who">
            <span class="name">{e.name} <Icon name="check" size={13} /></span>
            <span class="sub">{e.game}{e.channel ? ` · ${e.channel}` : ''}</span>
          </div>
          <span class="mins">{endDate(e.earnedAt)}</span>
        </div>
      {/each}
    {/if}
  {/if}
</section>

<style>
  .badges {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 8px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 4px 0 6px;
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
  .count {
    color: var(--dim);
    font-weight: 600;
  }
  .badge {
    display: grid;
    grid-template-columns: 28px minmax(0, 1fr) 140px 11ch 10ch;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .badge.done {
    grid-template-columns: 28px minmax(0, 1fr) 10ch;
  }
  .badge:hover {
    background: var(--surface-hover);
  }
  .badge.live {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  img,
  .art {
    width: 28px;
    height: 28px;
    border-radius: var(--radius);
    object-fit: contain;
  }
  .art {
    background: var(--surface);
  }
  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    padding-left: 24px;
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .done .sub {
    padding-left: 0;
  }
  .bar {
    height: 6px;
    background: var(--surface);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent-line);
  }
  .mins,
  .ends {
    color: var(--dim);
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .fold {
    display: flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
    margin: 6px 0 2px;
    padding: 2px 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.82rem;
    font-weight: 600;
  }
  .fold:hover {
    background: var(--surface-hover);
  }
  .empty {
    margin: 6px 0;
    color: var(--dim);
    font-size: 0.82rem;
  }
</style>
