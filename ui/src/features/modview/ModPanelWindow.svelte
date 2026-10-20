<!-- V107-R29 One mod view panel in a window of its own, route `modpanel/<login>/<panel>`.
     Closing it docks it back into the mod view. The account comes from the roles, live,
     as in the whole view (V102-21). -->
<script lang="ts">
  import PanelBox from '../../components/PanelBox.svelte';
  import ModPanelBody from './ModPanelBody.svelte';
  import { mods } from './moderation.svelte';
  import { parseRoute } from './popped';

  let { route }: { route: string; popout?: boolean } = $props();
  const target = $derived(parseRoute(route));
  const login = $derived(target?.login ?? '');
  const account = $derived(login ? mods.moderating(login) : null);
</script>

{#if target && account}
  {#key account}
    <div class="one"><PanelBox ctl={{ unlocked: false, bare: true }}><ModPanelBody id={target.id} {login} {account} /></PanelBox></div>
  {/key}
{:else}
  <p class="gone">You’re not moderating {login} right now. Open its chat with your mod account first.</p>
{/if}

<style>
  .one {
    flex: 1;
    min-width: 0;
    display: flex;
    padding: 8px;
    background: var(--page);
  }
  .gone {
    padding: 24px;
    color: var(--muted);
    font-size: 0.9rem;
  }
</style>
