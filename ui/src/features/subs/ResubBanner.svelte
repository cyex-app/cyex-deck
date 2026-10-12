<!-- 1.28 Above the message box when Twitch lets you announce a resub here, like twitch.tv. -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { ResubShare as Resub } from '../../ipc/bindings/ResubShare';
  import { subsIpc } from '../../ipc/subs';
  import { accounts } from '../../lib/accounts.svelte';
  import ResubShare from './ResubShare.svelte';

  let { account, login }: { account: string | null; login: string } = $props();

  let resub = $state<Resub | null>(null);
  let open = $state(false);
  let dismissed = $state(false);

  $effect(() => {
    const [a, l] = [account, login];
    resub = null;
    if (!a || !accounts.can('subs', a)) return;
    subsIpc
      .channel(a, l)
      .then((c) => {
        if (a === account && l === login) resub = c?.resub ?? null;
      })
      .catch(() => {});
  });
</script>

{#if resub && account && !dismissed}
  <div class="banner">
    {#if open}
      <ResubShare {account} {login} {resub} onshared={() => (resub = null)} />
    {:else}
      <button class="ask" onclick={() => (open = true)}>
        <Icon name="star" size={14} />Share your resub
      </button>
    {/if}
    <IconButton icon="x" label="Not now" size={14} onclick={() => (dismissed = true)} />
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 0 10px 6px;
    padding: 8px 8px 8px 10px;
    border-left: 3px solid var(--twitch);
    background: color-mix(in srgb, var(--twitch) 8%, var(--surface));
    border-radius: var(--radius);
  }
  .banner > :global(.share) {
    flex: 1;
  }
  .ask {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0;
    background: none;
    border: 0;
    color: var(--text);
    font-weight: 600;
    text-align: left;
  }
</style>
