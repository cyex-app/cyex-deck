<!-- 1.46 One mod view panel by its id: in the layout, in its quick menu and in a window of
     its own (V107-R29). V103-07 The stream shows the tab's own player when `host` is that
     tile, else a player of its own. -->
<script lang="ts">
  import Card from '../../components/PanelCard.svelte';
  import type { ModActionFilters } from '../../ipc/bindings/ModActionFilters';
  import { settings } from '../../lib/settings.svelte';
  import ChatView from '../chat/ChatView.svelte';
  import ViewerList from '../chat/ViewerList.svelte';
  import { layout } from '../tabs/layout.svelte';
  import ActivityFeed from './ActivityFeed.svelte';
  import AutomodQueue from './AutomodQueue.svelte';
  import BlockedTerms from './BlockedTerms.svelte';
  import { ModBoard } from './board.svelte';
  import MassActions from './MassActions.svelte';
  import ModActions from './ModActions.svelte';
  import ModRewards from './ModRewards.svelte';
  import ModStream from './ModStream.svelte';
  import UnbanQueue from './UnbanQueue.svelte';

  interface Props {
    id: string;
    login: string;
    account: string;
    /** The view's board; a panel on its own (V107-R29) loads its own. */
    data?: ModBoard;
    host?: string | null;
  }

  let { id, login, account, data: shared, host = null }: Props = $props();

  // svelte-ignore state_referenced_locally
  const data = shared ?? new ModBoard(() => login, () => account);

  const filters = $derived(settings.v.modview.modActions);
  const board = $derived(data.board);

  function flip(key: keyof ModActionFilters) {
    void settings.patch('modview', { modActions: { [key]: !filters[key] } });
  }

  /** V103-07 Offers the tile's player this place while the panel shows. */
  function slot(node: HTMLElement, tile: string) {
    let current = tile;
    layout.offer(current, node);
    return {
      update(next: string) {
        layout.withdraw(current, node);
        current = next;
        layout.offer(current, node);
      },
      destroy() {
        layout.withdraw(current, node);
      },
    };
  }
</script>

{#if id === 'chat'}
  <Card title="Chat" flush>
    <div class="fill"><ChatView {login} {account} onmodview={null} follows /></div>
  </Card>
{:else if id === 'activity'}
  <Card title="Activity" flush><ActivityFeed items={board.activity} suspicious={board.suspicious} /></Card>
{:else if id === 'actions'}
  <ModActions items={board.actions} older={board.older} {filters} onflip={flip} {account} {login} />
{:else if id === 'automod'}
  <Card title="AutoMod queue" flush><AutomodQueue {account} {login} held={board.held} /></Card>
{:else if id === 'unban'}
  <UnbanQueue {account} {login} />
{:else if id === 'chatters'}
  <Card title="Viewers" flush><ViewerList {account} {login} /></Card>
{:else if id === 'terms'}
  <BlockedTerms {account} {login} actions={board.actions} />
{:else if id === 'rewards'}
  <ModRewards {account} {login} />
{:else if id === 'mass'}
  <Card title="Mass actions" flush><MassActions {account} {login} channelId={data.channelId} /></Card>
{:else if id === 'stream'}
  <Card title="Stream" flush>
    {#if host}
      <div class="slot" use:slot={host}></div>
    {:else if data.channelId}
      <ModStream {login} {account} channelId={data.channelId} />
    {/if}
  </Card>
{/if}

<style>
  .fill {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .slot {
    flex: 1;
    min-height: 0;
    background: #000;
  }
</style>
