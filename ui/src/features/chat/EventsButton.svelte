<!-- V103-04 The button a hidden pinned message, poll and prediction tuck into ("2 Events"),
     at the top of the chat; a click shows them again. It also keeps the count right as
     they come and end (rules in eventStrip.ts). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Pinned } from '../../ipc/bindings/Pinned';
  import { chatEvents, eventsLabel } from './eventStrip';
  import { eventStrip } from './eventStrip.svelte';
  import { votes } from './votes.svelte';

  let { channel, pin }: { channel: string; pin: Pinned | null } = $props();

  /** Bumped when a timed pin runs out, so it leaves the count. */
  let clock = $state(0);
  $effect(() => {
    const left = pin?.endsAt ? Date.parse(pin.endsAt) - Date.now() : NaN;
    if (!(left > 0)) return;
    const t = setTimeout(() => clock++, Math.min(left + 100, 2 ** 31 - 1));
    return () => clearTimeout(t);
  });

  $effect(() => {
    void clock;
    eventStrip.settle(channel, chatEvents(pin, votes.byChannel.get(channel), Date.now()));
  });

  const count = $derived(eventStrip.of(channel).hidden.size);
</script>

{#if count}
  <button class="events" onclick={() => eventStrip.reopen(channel)} use:tooltip={'Show them again'}>
    <Icon name="layers" size={14} />
    <span>{eventsLabel(count)}</span>
    <Icon name="chevronDown" size={13} />
  </button>
{/if}

<style>
  .events {
    display: flex;
    align-items: center;
    gap: 6px;
    width: calc(100% - 16px);
    height: 28px;
    margin: 6px 8px 2px;
    padding: 0 8px 0 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
  }
  .events span {
    flex: 1;
    color: var(--text);
    text-align: left;
  }
  .events:hover {
    background: var(--surface-hover);
  }
</style>
