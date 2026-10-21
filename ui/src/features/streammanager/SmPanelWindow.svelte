<!-- V107-R40 One stream manager panel in a window of its own, route `smpanel/<panel>`.
     Only who runs the manager is read here, not its feed; the panel stays in the manager's
     layout too, so closing this window changes nothing there. -->
<script lang="ts">
  import PanelBox from '../../components/PanelBox.svelte';
  import { smIpc } from '../../ipc/streammanager';
  import ClipsCard from './ClipsCard.svelte';
  import FollowersCard from './FollowersCard.svelte';
  import ModSettingsCard from './ModSettingsCard.svelte';
  import RewardsCard from './RewardsCard.svelte';
  import RolesCard from './RolesCard.svelte';
  import RolesManagerCard from './RolesManagerCard.svelte';
  import { sm } from './streammanager.svelte';
  import VideosCard from './VideosCard.svelte';

  let { route }: { route: string; popout?: boolean } = $props();
  const id = $derived(route.split('/')[1] ?? '');

  let ready = $state(false);
  $effect(() => {
    smIpc
      .manager()
      .then((m) => (sm.manager = m))
      .catch(() => {})
      .finally(() => (ready = true));
  });
</script>

{#if ready && sm.manager}
  <div class="one">
    <PanelBox ctl={{ unlocked: false, bare: true }}>
      {#if id === 'followers'}<FollowersCard />
      {:else if id === 'roles'}<RolesCard />
      {:else if id === 'rolesmgr'}<RolesManagerCard />
      {:else if id === 'rewards'}<RewardsCard />
      {:else if id === 'clips'}<ClipsCard />
      {:else if id === 'videos'}<VideosCard />
      {:else if id === 'modset'}<ModSettingsCard />
      {/if}
    </PanelBox>
  </div>
{:else if ready}
  <p class="gone">Sign in with Twitch for your own channel to use this.</p>
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
