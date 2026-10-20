<!-- V101-05 Their timeouts, bans or warnings in this channel, from the mod logs, in Twitch's
     own words. Oldest at the top, like the messages. -->
<script lang="ts">
  import type { ModAction } from '../../ipc/bindings/ModAction';
  import { formatDate, formatTime } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { actionText } from './actionText';

  let { actions, empty }: { actions: ModAction[]; empty: string } = $props();

  let box = $state<HTMLElement>();
  const oldest = $derived([...actions].reverse());

  const when = (rfc: string) => {
    const d = new Date(rfc);
    return `${formatDate(d)} ${formatTime(d, settings.v.chat.timestamps)}`;
  };

  $effect(() => {
    void oldest.length;
    if (box) box.scrollTop = box.scrollHeight;
  });
</script>

<div class="list" bind:this={box}>
  {#each oldest as a (a.id)}
    <div class="row">
      <span class="when">{a.at ? when(a.at) : ''}</span>
      <span>{actionText(a.text)}</span>
    </div>
  {:else}
    <p class="empty">{empty}</p>
  {/each}
</div>

<style>
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 0;
  }
  .row {
    padding: 3px 14px;
    color: var(--muted);
    font-size: 0.82rem;
  }
  .when {
    display: block;
    color: var(--dim);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .empty {
    margin: 0;
    padding: 40px 14px 0;
    color: var(--dim);
    font-size: 0.82rem;
    text-align: center;
  }
</style>
