<!-- U7 A tab's chat over games and other apps: see-through, always on top, no message
     box. Its bar shows on hover: drag it to move, the pointer lets clicks through to the
     window below. The bar stays reachable while clicks go through (V105-02); the tray or a
     global hotkey take them back too. Looks are in Settings →
     Chat → Overlay. Route: `overlay/<tile id>`. -->
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { untrack } from 'svelte';

  import IconButton from '../../components/IconButton.svelte';
  import { modesIpc } from '../../ipc/modes';
  import { windowsIpc } from '../../ipc/windows';
  import { settings } from '../../lib/settings.svelte';
  import { streamer } from '../../lib/streamer.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { catalogs } from '../chat/catalog.svelte';
  import Line from '../chat/Line.svelte';
  import { ChatRoom } from '../chat/room.svelte';
  import { tabs, tiles } from '../tabs/tabs.svelte';

  let { route }: { route: string; popout?: boolean } = $props();

  const tileId = $derived(route.split('/')[1]);
  const tile = $derived(tabs.list.flatMap(tiles).find((t) => t.id === tileId));
  const o = $derived(settings.v.chat.overlay);
  /** Reopens the chat only when what it reads changes, not on every tab edit. */
  const key = $derived(tile ? `${tile.channel.login}/${tile.account}/${tile.filters.join()}` : '');

  let room = $state<ChatRoom | null>(null);
  let clickThrough = $state(false);
  /** V105-02 The cursor is on the bar while clicks go through. */
  let barHover = $state(false);

  $effect(() => {
    if (!tile) void getCurrentWindow().close();
  });

  $effect(() => {
    if (tile) void windowsIpc.setTitle(`${tile.channel.name} overlay`);
  });

  $effect(() => {
    if (!key) return;
    const t = untrack(() => tile)!;
    const r = new ChatRoom(t.channel.login);
    void r.open(t.account, t.filters);
    room = r;
    // V105-01 Twitch badges and cheers draw once their catalog is here, like in the chat.
    void catalogs.loadGlobal();
    void catalogs.loadChannel(t.channel.login);
    return () => r.close();
  });

  $effect(() => {
    void modesIpc.clickThrough().then((on) => (clickThrough = on));
    const un = modesIpc.onClickThrough((on) => (clickThrough = on));
    const unBar = modesIpc.onBar((on) => (barHover = on));
    return () => {
      void un.then((f) => f());
      void unBar.then((f) => f());
    };
  });

  const hideMod = $derived(streamer.hides('hideModActions'));
  const hideWhispers = $derived(streamer.hides('hideWhispers'));
  const shown = $derived(
    (room?.messages ?? []).filter(
      (m) =>
        !m.flags.includes('similar') &&
        !(hideMod && (m.notice?.kind === 'moderation' || m.flags.includes('deleted'))) &&
        !(hideWhispers && m.flags.includes('whisper')),
    ),
  );

  /** A line fades once it's `fadeAfter` seconds old; older history is gone right away. */
  function fadeDelay(ts: number): string {
    return `${o.fadeAfter - (Date.now() - ts) / 1000}s`;
  }

  async function toggleClickThrough() {
    try {
      const on = !clickThrough;
      await modesIpc.setClickThrough(on);
      if (on) toast.info('Clicks go through now. Hover the top to take them back.');
    } catch (e) {
      toast.error(String(e));
    }
  }
</script>

<div
  class="overlay"
  class:outline={o.outline}
  class:no-badges={!o.badges}
  class:fading={o.fadeAfter > 0}
  style:--overlay-bg="color-mix(in srgb, var(--bg) {o.background}%, transparent)"
  style:--chat-size="calc({settings.v.general.chatFontSize}px * {o.scale / 100})"
>
  <div class="bar" class:shown={clickThrough && barHover} data-tauri-drag-region>
    <span class="title" data-tauri-drag-region>{tile?.channel.name ?? ''}</span>
    <IconButton
      icon="pointer"
      active={clickThrough}
      label={clickThrough ? 'Take clicks back' : 'Let clicks through'}
      hint={clickThrough
        ? 'The overlay takes the mouse again.'
        : 'The mouse reaches the window below, except on this bar.'}
      size={14}
      onclick={toggleClickThrough}
    />
    <IconButton icon="x" label="Close" size={14} onclick={() => void getCurrentWindow().close()} />
  </div>
  <!-- Newest at the bottom; older lines run off the top. -->
  <div class="list">
    {#each shown as m (m.id)}
      <div class="row" style:animation-delay={o.fadeAfter > 0 ? fadeDelay(m.ts) : undefined}>
        <Line {m} channel={tile?.channel.login ?? ''} />
      </div>
    {/each}
  </div>
</div>

<style>
  .overlay {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--overlay-bg);
    border-radius: var(--radius);
  }
  .bar {
    position: absolute;
    inset: 0 0 auto;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 2px;
    height: 30px;
    padding: 0 4px 0 10px;
    background: var(--elevated);
    border-bottom: 1px solid var(--border);
    border-radius: var(--radius) var(--radius) 0 0;
    opacity: 0;
    transition: opacity var(--fast);
  }
  .overlay:hover .bar,
  .bar.shown {
    opacity: 1;
  }
  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    font-size: 0.8rem;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    padding: 4px 0;
  }
  .row {
    flex: none;
  }
  .row :global(.line:hover) {
    background: none;
  }
  .row :global(.name),
  .row :global(.reply) {
    cursor: default;
  }
  .fading .row {
    animation: fade 0.6s ease-in forwards;
  }
  @keyframes fade {
    to {
      opacity: 0;
    }
  }
  /* V105-01 A painted name keeps its picture: its outline is a drop-shadow (emotes.css). */
  .outline .list {
    text-shadow:
      0 0 2px #000,
      0 0 2px #000,
      1px 1px 1px #000;
    --name-outline: drop-shadow(0 0 1px #000) drop-shadow(1px 1px 1px #000);
  }
  .outline .list :global(.painted) {
    text-shadow: none;
  }
  .no-badges .list :global(.badge),
  .list :global(.ts) {
    display: none;
  }
</style>
