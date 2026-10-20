<!-- One channel in the miner (1.31, 1.33): avatar and name, whether it's being mined,
     watched in the app (W1), live or offline, its balance, streak and drops, and buttons
     to put it first (1.44), skip it (W3) or take it off the list. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import type { MinedChannel } from '../../ipc/bindings/MinedChannel';
  import type { TabChannel } from '../../ipc/bindings/TabChannel';
  import { formatNumber } from '../../lib/format';
  import { tooltip } from '../../components/tooltip.svelte';

  interface Props {
    channel: TabChannel;
    /** What Rust knows about it, once looked up. */
    info?: MinedChannel;
    first: boolean;
    /** Greyed out: a follow the miner skips (W3). */
    skipped?: boolean;
    /** The miner isn't running, so live or not isn't known. */
    idle?: boolean;
    onfirst?: (on: boolean) => void;
    onremove?: () => void;
    onskip?: (skip: boolean) => void;
    /** A drag handle, for the list. */
    handle?: Snippet;
  }

  let { channel, info, first, skipped = false, idle = false, onfirst, onremove, onskip, handle }: Props = $props();

  const avatar = $derived(info?.avatar ?? null);
</script>

<div class="row" class:skipped class:mining={info?.mining}>
  {#if handle}{@render handle()}{/if}
  <Avatar src={avatar} name={channel.name} size={28} />
  <div class="who">
    <span class="name">{channel.name}</span>
    <span class="sub">
      {#if info?.mining}
        <span class="state on"><Icon name="pickaxe" size={12} />Mining</span>
      {:else if info?.watching}
        <span class="state on"><Icon name="tv" size={12} />Watching here</span>
      {:else if info?.live}
        <span class="state"><span class="dot"></span>Live</span>
      {:else if info && !idle}
        <span class="state off">Offline</span>
      {/if}
      {#if info?.live && info.game}<span class="game">{info.game}</span>{/if}
    </span>
  </div>
  {#if info?.streak}<span class="chip" use:tooltip={'A watch streak is still to be had this stream.'}>Streak</span>{/if}
  {#if info?.drops}<span class="chip" use:tooltip={'Drops run here.'}>Drops</span>{/if}
  {#if info?.balance != null}
    <span class="balance" use:tooltip={'Channel points'}><Icon name="points" size={13} />{formatNumber(info.balance)}</span>
  {/if}
  {#if onfirst}
    <IconButton
      icon={first ? 'starFilled' : 'star'}
      label={first ? 'Not first anymore' : 'Mine first'}
      hint="Two channels can go before all others, e.g. for drops."
      size={14}
      active={first}
      onclick={() => onfirst(!first)}
    />
  {/if}
  {#if onskip}
    <IconButton
      icon={skipped ? 'eye' : 'eyeOff'}
      label={skipped ? 'Mine again' : 'Skip'}
      hint={skipped ? undefined : "Skipped channels aren’t mined."}
      size={14}
      onclick={() => onskip(!skipped)}
    />
  {/if}
  {#if onremove}<IconButton icon="x" label="Take off the list" size={14} onclick={onremove} />{/if}
  <!-- v1.07 T4: a first channel in follows mode has no last button; keep its star in line. -->
  {#if onfirst && !onremove && !onskip}<span class="slot"></span>{/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .row:hover {
    background: var(--surface-hover);
  }
  .row.mining {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  .slot {
    flex-shrink: 0;
    width: 30px;
  }
  .skipped {
    opacity: 0.55;
  }
  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: var(--dim);
    font-size: 0.78rem;
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .state.on {
    color: var(--accent-text);
    font-weight: 600;
  }
  .dot {
    width: 7px;
    height: 7px;
    background: var(--live);
    border-radius: 50%;
  }
  .game {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chip {
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.72rem;
  }
  .balance {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
</style>
