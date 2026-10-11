<!-- The stream side of a tile: the live stream (1.1) or the channel's pages (1.21), with
     clips over the stream (1.20) and stories. While a page shows, the stream keeps
     playing small in a corner (P4); a VOD or clip from a list stops it. Fullscreen takes
     the whole body, so a clip opened over the stream shows there too (V101-26). -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import type { Tile } from '../../ipc/bindings/Tile';
  import { playerIpc } from '../../ipc/player';
  import { accounts } from '../../lib/accounts.svelte';
  import { toast } from '../../lib/toast.svelte';
  import ClipOverlay from '../player/ClipOverlay.svelte';
  import ClipPlayer from '../player/ClipPlayer.svelte';
  import LivePlayer from '../player/LivePlayer.svelte';
  import { type Page, tileState } from '../player/players.svelte';
  import StoryViewer from '../player/StoryViewer.svelte';
  import VodPlayer from '../player/VodPlayer.svelte';
  import { tabs } from '../tabs/tabs.svelte';
  import AboutPage from './AboutPage.svelte';
  import ClipsPage from './ClipsPage.svelte';
  import Offline from './Offline.svelte';
  import StreamHeader from './StreamHeader.svelte';
  import VideosPage from './VideosPage.svelte';

  interface Props {
    tile: Tile;
    /** Header buttons, when the chat isn't there to carry them. */
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
    /** 1.58 Pops the chat out; only when the tile shows it. */
    onpopchat?: () => void;
    /** 3.14 The chat is in its own window: this brings it back. */
    onbringchat?: () => void;
  }

  let { tile, actions, onheaderdown, onpopchat, onbringchat }: Props = $props();

  const shared = $derived(tileState(tile.id));
  const info = $derived(tabs.live[tile.channel.id]);
  const live = $derived(Boolean(info?.stream));
  const page = $derived(shared.page);
  const listing = $derived(page.kind === 'about' || page.kind === 'clips' || page.kind === 'videos');
  /** P4 The small stream was closed by hand. */
  let miniClosed = $state(false);
  /** P9 A clip just made can be edited on Twitch. */
  let editUrl = $state<string | null>(null);

  const account = $derived(tile.account && accounts.list.some((a) => a.userId === tile.account) ? tile.account : null);
  const canClip = $derived(live && account !== null && accounts.can('clips', account));
  const hushed = $derived(shared.overlay !== null || shared.stories);

  function go(p: Page) {
    shared.page = p;
    if (p.kind === 'stream') miniClosed = false;
  }

  /** P9 */
  async function makeClip() {
    if (!account) return;
    toast.info('Making a clip…');
    try {
      const made = await playerIpc.createClip(account, tile.channel.id);
      editUrl = made.edit_url;
      shared.overlay = made.id;
    } catch (e) {
      toast.error(String(e));
    }
  }

  function closeOverlay() {
    shared.overlay = null;
    editUrl = null;
  }
</script>

<section class="side">
  <StreamHeader {tile} {info} {page} onpage={go} onstories={() => (shared.stories = true)} {actions} {onheaderdown} />
  {#if onbringchat}
    <div class="popped">
      Chat is in its own window.
      <button onclick={onbringchat}>Bring it back</button>
    </div>
  {/if}
  <div class="body" data-fullscreen>
    {#if page.kind === 'about'}
      <AboutPage login={tile.channel.login} {account} onstories={() => (shared.stories = true)} />
    {:else if page.kind === 'clips'}
      <ClipsPage login={tile.channel.login} onplay={(slug) => go({ kind: 'clip', slug })} />
    {:else if page.kind === 'videos'}
      <VideosPage login={tile.channel.login} onplay={(id) => go({ kind: 'vod', id, at: 0 })} />
    {:else if page.kind === 'vod'}
      {#key page.id}
        <VodPlayer id={page.id} at={page.at} {tile} {account} {shared} {hushed} onclose={() => go({ kind: 'videos' })} />
      {/key}
    {:else if page.kind === 'clip'}
      {#key page.slug}<ClipPlayer slug={page.slug} {tile} />{/key}
    {/if}
    {#if live && (page.kind === 'stream' || (listing && !miniClosed))}
      <LivePlayer
        {tile}
        {info}
        {hushed}
        mini={page.kind !== 'stream'}
        onexpand={() => go({ kind: 'stream' })}
        onclose={() => (miniClosed = true)}
        {onpopchat}
        onclip={canClip ? makeClip : undefined}
      />
    {:else if page.kind === 'stream'}
      <Offline {tile} {info} onpage={go} onstories={() => (shared.stories = true)} />
    {/if}
    {#if shared.overlay}
      <ClipOverlay slug={shared.overlay} {tile} {editUrl} onclose={closeOverlay} />
    {/if}
    {#if shared.stories && account}
      <StoryViewer
        channelId={tile.channel.id}
        {account}
        name={tile.channel.name}
        avatar={info?.avatar}
        onclose={() => (shared.stories = false)}
      />
    {/if}
  </div>
</section>

<style>
  .side {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--page);
  }
  .body {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .body:fullscreen {
    background: #000;
  }
  .popped {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 12px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.8rem;
  }
  .popped button {
    padding: 1px 8px;
    background: var(--accent-dim);
    border: 0;
    border-radius: var(--radius);
    color: var(--accent-text);
    font-size: 0.75rem;
    font-weight: 600;
  }
</style>
