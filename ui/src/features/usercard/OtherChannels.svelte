<!-- V108-N16 Their bans and timeouts in the other channels you moderate (read only). Hidden
     when you mod nothing else, there is nothing on them, or Twitch won't say (no scope). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { usercardIpc } from '../../ipc/rules';
  import type { OtherModCount } from '../../ipc/bindings/OtherModCount';
  import { countText, sortCounts } from './otherCounts';

  let { channel, userId, account }: { channel: string; userId: string; account: string } = $props();

  let list = $state.raw<OtherModCount[]>([]);

  // A card shows one user in one channel for its whole life.
  // svelte-ignore state_referenced_locally
  usercardIpc
    .otherModCounts(account, channel, userId)
    .then((l) => (list = sortCounts(l)))
    .catch(() => {});
</script>

{#if list.length}
  <ul>
    {#each list as c (c.login)}
      <li><Icon name="shield" size={14} /><span><b>{c.name}</b> · {countText(c)}</span></li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    display: grid;
    gap: 3px;
    margin: 0;
    padding: 0 14px 8px;
    list-style: none;
    font-size: 0.82rem;
  }
  li {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }
  li :global(svg) {
    flex: none;
    color: var(--dim);
  }
  li > span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
