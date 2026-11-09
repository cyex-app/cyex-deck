<!-- V108-N10 Messages per second in the chat header. Looks every 2 s while the window is
     visible; setting "Show messages per second" (off by default). -->
<script lang="ts">
  import { tooltip } from '../../components/tooltip.svelte';
  import { chatxIpc } from '../../ipc/chatx';
  import { everyShown } from '../../lib/awake';
  import { formatNumber } from '../../lib/format';
  import { RATE_EVERY_MS, rateBusy, rateText } from './rateText';

  let { login }: { login: string } = $props();

  let perSec = $state(0);
  let peak = $state(0);

  $effect(() => {
    const who = login;
    let dead = false;
    const look = () => {
      if (document.hidden) return;
      chatxIpc
        .rate(who)
        .then((r) => {
          if (dead) return;
          perSec = r.perSec;
          peak = r.peak;
        })
        .catch(() => {});
    };
    look();
    const stop = everyShown(look, RATE_EVERY_MS);
    return () => {
      dead = true;
      stop();
    };
  });
</script>

<span
  class="rate"
  class:busy={rateBusy(perSec)}
  use:tooltip={{ label: 'Messages per second', hint: `Busiest second in the last minute: ${formatNumber(peak)}.` }}
>{rateText(perSec)}</span>

<style>
  .rate {
    flex: none;
    padding: 1px 6px;
    color: var(--muted);
    font-size: 0.72rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .rate.busy {
    color: var(--text);
  }
  /* A narrow chat: the modes and buttons need the room. */
  @container chat (max-width: 260px) {
    .rate {
      display: none;
    }
  }
</style>
