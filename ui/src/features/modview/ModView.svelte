<!-- 1.46 The moderator view for a channel the account mods: a grid of widgets, dragged to
     rearrange and lockable (1.53). Opened from the shield on a chat (1.46) and poppable
     out (1.47). The layout is one for every channel, in the `modview` settings. The chat
     widget (V102-22) is a view on the room the tab already has open, as the moderating
     account, with its own scroll and pause; with no tab on that channel it joins for as
     long as the widget shows. V102-26 Inside a tab it has a close button. -->
<script lang="ts">
  import { onDestroy } from 'svelte';

  import IconButton from '../../components/IconButton.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Board0 } from '../../ipc/bindings/Board0';
  import { modIpc } from '../../ipc/moderation';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import ActivityFeed from './ActivityFeed.svelte';
  import AutomodQueue from './AutomodQueue.svelte';
  import ChatView from '../chat/ChatView.svelte';
  import ViewerList from '../chat/ViewerList.svelte';
  import MassActions from './MassActions.svelte';
  import ModActions from './ModActions.svelte';
  import { mods } from './moderation.svelte';
  import UnbanQueue from './UnbanQueue.svelte';

  /** The channel login; `account` mods it. `onpopout` shows in the main window (V102-16)
   * and in a tab, `onclose` in a tab (V102-26). */
  let { login, account, onpopout, onclose }: { login: string; account: string; onpopout?: () => void; onclose?: () => void } =
    $props();

  type Widget = 'chat' | 'activity' | 'actions' | 'automod' | 'unban' | 'chatters' | 'mass';
  const TITLES: Record<Widget, string> = {
    chat: 'Chat',
    activity: 'Activity',
    actions: 'Mod actions',
    automod: 'AutoMod queue',
    unban: 'Unban requests',
    chatters: 'Viewers',
    mass: 'Mass actions',
  };
  const DEFAULT: Widget[] = ['chat', 'activity', 'actions', 'automod', 'unban', 'chatters', 'mass'];

  let channelId = $state<string | null>(null);
  let board = $state<Board0>({ activity: [], actions: [], held: [], suspicious: [] });

  const mv = $derived(settings.v.modview);
  const locked = $derived(mv.locked);
  const order = $derived.by(() => {
    const seen = new Set(mv.order);
    const full = [...mv.order.filter((w): w is Widget => (DEFAULT as string[]).includes(w)), ...DEFAULT.filter((w) => !seen.has(w))];
    return full.filter((w) => !mv.hidden.includes(w)) as Widget[];
  });

  // The id comes from an open room. V102-26 From a tab with only the stream, that's this
  // view's chat widget joining: asked again whenever the roles change until it's known.
  $effect(() => {
    if (channelId) return;
    void mods.role(account, login);
    void modIpc.channelId(login).then((id) => {
      if (!id || channelId) return;
      channelId = id;
      modIpc.view(id).then((b) => (board = b)).catch(() => {});
    });
  });

  $effect(() => {
    if (!channelId) return;
    let stop: (() => void) | undefined;
    let gone = false;
    void modIpc.onBoard(channelId, (b) => (board = b)).then((u) => (gone ? u() : (stop = u)));
    return () => {
      gone = true;
      stop?.();
    };
  });

  let dragging = $state<Widget | null>(null);

  function drop(target: Widget) {
    if (!dragging || locked || dragging === target) return;
    const list = order.slice();
    const from = list.indexOf(dragging);
    const to = list.indexOf(target);
    list.splice(to, 0, list.splice(from, 1)[0]);
    void settings.patch('modview', { order: list });
    dragging = null;
  }

  function hide(w: Widget) {
    void settings.patch('modview', { hidden: [...mv.hidden, w] });
  }

  function resetLayout() {
    void settings.patch('modview', { order: [], hidden: [], locked: false });
    toast.info('Mod view layout reset.');
  }

  onDestroy(() => {});
</script>

<div class="modview">
  <header>
    <h2>Mod view · {login}</h2>
    <div class="spacer"></div>
    {#if mv.hidden.length}
      <button class="link" onclick={resetLayout}>Reset layout</button>
    {/if}
    <IconButton
      icon={locked ? 'lock' : 'grid'}
      label={locked ? 'Unlock layout' : 'Lock layout'}
      hint={locked ? undefined : 'Stops widgets from moving when you drag by accident.'}
      size={15}
      onclick={() => void settings.patch('modview', { locked: !locked })}
    />
    {#if onpopout}
      <IconButton icon="popout" label="Pop out" hint="Open the mod view in its own window." size={15} onclick={onpopout} />
    {/if}
    {#if onclose}
      <IconButton icon="x" label="Close mod view" size={15} onclick={onclose} />
    {/if}
  </header>

  <div class="grid">
    {#each order as w (w)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <section
        class="widget"
        class:tall={w === 'chat'}
        class:dragging={dragging === w}
        draggable={!locked}
        ondragstart={() => (dragging = w)}
        ondragend={() => (dragging = null)}
        ondragover={(e) => e.preventDefault()}
        ondrop={() => drop(w)}
      >
        <div class="whead" class:grab={!locked}>
          <span class="title">{TITLES[w]}</span>
          {#if !locked}
            <button class="x" use:tooltip={'Remove'} onclick={() => hide(w)} aria-label="Remove">×</button>
          {/if}
        </div>
        <div class="wbody">
          {#if w === 'chat'}
            <ChatView {login} {account} onmodview={null} />
          {:else if w === 'activity'}
            <ActivityFeed items={board.activity} suspicious={board.suspicious} />
          {:else if w === 'actions'}
            <ModActions items={board.actions} />
          {:else if w === 'automod'}
            <AutomodQueue {account} {login} held={board.held} />
          {:else if w === 'unban'}
            <UnbanQueue {account} {login} />
          {:else if w === 'chatters'}
            <ViewerList {account} {login} />
          {:else if w === 'mass'}
            <MassActions {account} {login} channelId={channelId} />
          {/if}
        </div>
      </section>
    {/each}
  </div>
</div>

<style>
  .modview {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--page);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 0.95rem;
  }
  .spacer {
    flex: 1;
  }
  .link {
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 0.8rem;
    cursor: pointer;
  }
  .link:hover {
    color: var(--text);
  }
  .grid {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 10px;
    padding: 10px;
    align-content: start;
  }
  .widget {
    display: flex;
    flex-direction: column;
    min-height: 180px;
    max-height: 420px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  /* V102-22 Room for a chat: two rows high. */
  .widget.tall {
    grid-row: span 2;
    height: min(72vh, 640px);
    max-height: none;
  }
  .widget.dragging {
    opacity: 0.5;
  }
  .whead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    font-weight: 700;
    font-size: 0.8rem;
  }
  .whead.grab {
    cursor: grab;
  }
  .x {
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 1rem;
    line-height: 1;
    cursor: pointer;
  }
  .x:hover {
    color: var(--text);
  }
  .wbody {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .tall .wbody {
    display: flex;
    overflow: hidden;
  }
</style>
