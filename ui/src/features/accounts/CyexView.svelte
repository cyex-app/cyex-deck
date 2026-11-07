<!-- V109-B14 The cyex cosmetics page: a pop-up from the account menu, like the 7TV one. It acts
     for the default account, or the one picked in the header. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Select from '../../components/Select.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import { accountChoices } from '../cosmetics/accountChoices';
  import CyexCosmetics from './CyexCosmetics.svelte';

  let chosen = $state<string | null>(null);
  const picked = $derived(
    accounts.list.find((a) => a.userId === chosen) ?? accounts.current ?? accounts.list[0] ?? null,
  );
  const choices = $derived(accountChoices(accounts.list));
</script>

<div class="view">
  <header>
    <h1>cyex cosmetics</h1>
    {#if accounts.list.length > 1 && picked}
      <Select label="Account" width="220px" value={picked.userId} options={choices} onchange={(id) => (chosen = id)} />
    {:else if accounts.list.length === 1}
      <span class="one"><Avatar src={accounts.list[0].avatar} name={accounts.list[0].displayName} size={20} />{accounts.list[0].displayName}</span>
    {/if}
  </header>
  <div class="body">
    {#if picked}
      {#key picked.userId}
        <CyexCosmetics account={picked} />
      {/key}
    {/if}
  </div>
</div>

<style>
  .view {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 48px 12px 20px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    margin: 0 4px 0 0;
    font-size: 1.05rem;
  }
  .one {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .body {
    flex: 1;
    width: 100%;
    max-width: 720px;
    min-height: 0;
    margin: 0 auto;
    padding: 16px 24px;
    box-sizing: border-box;
    overflow-y: auto;
  }
</style>
