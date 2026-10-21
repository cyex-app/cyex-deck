<!-- V107-R36..R40 When something happened, in a stream manager list: how long ago ("3 days
     ago", or "5m" when `short`), the date and time (24 h) on hover. `day` shows the date
     instead, how long ago on hover. -->
<script lang="ts">
  import { tooltip } from '../../components/tooltip.svelte';
  import { ago } from '../../lib/format';
  import { day as dayOf, relative, stamp } from './community';

  interface Props {
    at: string;
    now?: number;
    short?: boolean;
    day?: boolean;
  }

  let { at, now = Date.now(), short = false, day = false }: Props = $props();

  const ms = $derived(Date.parse(at));
  const shown = $derived(day ? dayOf(at) : short && Number.isFinite(ms) ? ago(ms, now) : relative(at, now));
  const tip = $derived(day ? `${relative(at, now)} · ${stamp(at)}` : stamp(at));
</script>

{#if shown}<time class="when" datetime={at} use:tooltip={tip}>{shown}</time>{/if}

<style>
  .when {
    flex: none;
    color: var(--dim);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
