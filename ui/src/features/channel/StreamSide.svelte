<!-- The stream side of a tile: the live stream (1.1) or the channel's pages (1.21), with
     clips over the stream (1.20) and stories. While a page shows, the stream keeps
     playing small in a corner (P4); a VOD or clip from a list stops it. Fullscreen takes
     the whole body, so a clip opened over the stream shows there too (V101-26). V103-05 The
     channel's mod view (V102-26) is a panel of its own next to this one. V107-Mv A VOD keeps
     playing small over About and Clips, paused while a clip plays; Videos brings it back.
     P4 #18 Over a mod view's Stream panel (`bare`) it is the player only: no header, and
     About, Clips and Videos show the stream instead. -->
<script lang="ts">
  import { onDestroy, type Snippet } from 'svelte';

  import type { Tile } from '../../ipc/bindings/Tile';
  import { playerIpc } from '../../ipc/player';
  import { accounts } from '../../lib/accounts.svelte';
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { toast } from '../../lib/toast.svelte';
  import ClipOverlay from '../player/ClipOverlay.svelte';
  import { clipMaking } from '../player/clipMake.svelte';
  import ClipPlayer from '../player/ClipPlayer.svelte';
  import LivePlayer from '../player/LivePlayer.svelte';
  import { type Page, tileState } from '../player/players.svelte';
  import { rewound } from '../player/rewound.svelte';
  import StoryViewer from '../player/StoryViewer.svelte';
  import VodPlayer from '../player/VodPlayer.svelte';
  import { chatToggle, headerHidden, REST_MS } from '../tabs/theatre';
  import { theatre } from '../tabs/theatre.svelte';
  import { tabs } from '../tabs/tabs.svelte';
  import AboutPage from './AboutPage.svelte';
  import ClipsPage from './ClipsPage.svelte';
  import Offline from './Offline.svelte';
  import StreamHeader from './StreamHeader.svelte';
  import VideosPage from './VideosPage.svelte';
  import { errorText } from '../../lib/errors';

  const STREAM: Page = { kind: 'stream' };

  interface Props {
    tile: Tile;
    /** Header buttons, when the chat isn't there to carry them. */
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
    /** 1.58 Pops the chat out; only when the tile shows it. */
    onpopchat?: () => void;
    /** 3.14 The chat is in its own window: this brings it back. */
    onbringchat?: () => void;
    /** P4 #18 Shown in a mod view's Stream panel: the player, nothing around it. */
    bare?: boolean;
  }

  let { tile, actions, onheaderdown, onpopchat, onbringchat, bare = false }: Props = $props();

  const shared = $derived(tileState(tile.id));
  const info = $derived(tabs.live[tile.channel.id]);
  // V1061-03 Q21 a A player on the rewind VOD plays to its end after the stream ends.
  const live = $derived(Boolean(info?.stream) || rewound.has(tile.channel.login));
  /** The page the tile is on; the body shows `page`, which is the stream for a listing when bare. */
  const chosen = $derived(shared.page);
  const page = $derived(bare && (chosen.kind === 'about' || chosen.kind === 'clips' || chosen.kind === 'videos') ? STREAM : chosen);
  const listing = $derived(page.kind === 'about' || page.kind === 'clips' || page.kind === 'videos');
  /** V107-Mv The VOD that stays open, small, while About or Clips shows. */
  let kept = $state<{ id: string; at: number } | null>(null);
  $effect(() => {
    if (chosen.kind === 'vod') kept = { id: chosen.id, at: chosen.at };
    else if (chosen.kind === 'stream') kept = null;
  });
  // Plain values: a new object on every page change would restart the player.
  const vodId = $derived(page.kind === 'vod' ? page.id : page.kind === 'stream' || page.kind === 'videos' ? null : (kept?.id ?? null));
  const vodAt = $derived(page.kind === 'vod' ? page.at : (kept?.at ?? 0));
  /** P4 The small stream was closed by hand. */
  let miniClosed = $state(false);
  /** V103-09 The overlay shows a clip just made without Twitch's editor. */
  let made = $state(false);

  const account = $derived(tile.account && accounts.list.some((a) => a.userId === tile.account) ? tile.account : null);
  const canClip = $derived(live && account !== null && accounts.can('clips', account));
  /** V102-08 Why the clip button is greyed out, if it is: clips are made as the tab's account. */
  const clipOff = $derived(
    account === null ? 'Open this tab signed in to clip.' : canClip ? null : 'This account can’t clip. Check Settings → Accounts.',
  );
  const hushed = $derived(shared.overlay !== null || shared.stories);
  /** V110-B10 Theatre: the button in the player that folds this tile's chat away and back. */
  const chatButton = $derived(
    !bare && page.kind === 'stream' && tile.show.player
      ? chatToggle(theatre.tile, tile.id, tile.show.chat && !tabs.chats.has(tile.id), theatre.chatHidden)
      : null,
  );

  /** V110-B11 The button fades out after the pointer has rested on the player for 3 s; a move brings it back. */
  let restTimer: ReturnType<typeof setTimeout> | undefined;
  let resting = $state(false);
  function stir() {
    if (!chatButton) return;
    resting = false;
    clearTimeout(restTimer);
    restTimer = setTimeout(() => (resting = true), REST_MS);
  }
  function still() {
    clearTimeout(restTimer);
    resting = false;
  }
  onDestroy(() => clearTimeout(restTimer));

  function go(p: Page) {
    // V107-Mv Videos while the VOD plays small: the VOD, where it is.
    // Videos again while it plays big: the list, the VOD closed.
    if (p.kind === 'videos' && kept) {
      if (page.kind === 'vod') kept = null;
      else p = { kind: 'vod', id: kept.id, at: kept.at };
    }
    shared.page = p;
    if (p.kind === 'stream') miniClosed = false;
  }

  function closeVod() {
    kept = null;
    if (page.kind === 'vod') shared.page = { kind: 'videos' };
  }

  /** P9 V109-F73 `at`: the frame on screen in the broadcast (s), where a GQL clip ends. */
  async function makeClip(at: number | null = null) {
    if (!account || !clipMaking.begin()) return;
    toast.info('Making a clip…', { key: 'clip' });
    try {
      const clip = await playerIpc.createClip(account, tile.channel.id, at);
      // V103-09 Twitch's editor is the next step, like on Twitch.
      if (clip.editing) {
        toast.info('Pick the start and end in the Twitch window.', { key: 'clip' });
        return;
      }
      made = true;
      shared.overlay = clip.slug;
      toast.success('Clip made', { key: 'clip' });
    } catch (e) {
      toast.error(errorText(e), { key: 'clip' });
    } finally {
      clipMaking.end();
    }
  }

  function closeOverlay() {
    shared.overlay = null;
    made = false;
  }
</script>

<section class="side">
  {#if !bare && !headerHidden(theatre.tile, tile.id, !actions)}
    <StreamHeader {tile} {info} {page} onpage={go} onstories={() => (shared.stories = true)} {actions} {onheaderdown} />
  {/if}
  {#if onbringchat && !bare}
    <div class="popped">
      Chat is in its own window.
      <button onclick={onbringchat}>Bring it back</button>
    </div>
  {/if}
  {@render body()}
</section>

{#snippet body()}
  <!-- svelte-ignore a11y_no_static_element_interactions (V110-B11: pointer rest only, nothing to operate) -->
  <div class="body" data-fullscreen onpointermove={stir} onpointerleave={still}>
    {#if page.kind === 'about'}
      <AboutPage login={tile.channel.login} {account} live={!!info?.stream} onstories={() => (shared.stories = true)} />
    {:else if page.kind === 'clips'}
      <ClipsPage login={tile.channel.login} onplay={(slug) => go({ kind: 'clip', slug })} />
    {:else if page.kind === 'videos'}
      <VideosPage login={tile.channel.login} onplay={(id) => go({ kind: 'vod', id, at: 0 })} />
    {:else if page.kind === 'clip'}
      {#key page.slug}<ClipPlayer slug={page.slug} {tile} onclose={() => go({ kind: 'clips' })} />{/key}
    {/if}
    {#if vodId}
      {#key vodId}
        <VodPlayer
          id={vodId}
          at={vodAt}
          {tile}
          {account}
          {shared}
          {hushed}
          mini={page.kind !== 'vod'}
          held={page.kind === 'clip'}
          {listing}
          onexpand={() => go({ kind: 'videos' })}
          onclose={closeVod}
        />
      {/key}
    {/if}
    {#if live && (page.kind === 'stream' || (listing && !miniClosed && !vodId))}
      <LivePlayer
        {tile}
        {info}
        {hushed}
        mini={page.kind !== 'stream'}
        {listing}
        onexpand={() => go({ kind: 'stream' })}
        onclose={() => (miniClosed = true)}
        {onpopchat}
        onclip={canClip ? makeClip : undefined}
        {clipOff}
      />
    {:else if page.kind === 'stream'}
      <Offline {tile} {info} onpage={go} onstories={() => (shared.stories = true)} />
    {/if}
    {#if shared.overlay}
      <!-- V110-F83 A second clip link loads its own player. -->
      {#key shared.overlay}
        <ClipOverlay slug={shared.overlay} {tile} {made} onclose={closeOverlay} />
      {/key}
    {/if}
    {#if chatButton}
      <button
        class="chat-toggle"
        class:resting
        onpointerenter={() => { clearTimeout(restTimer); resting = false; }}
        onpointerleave={stir}
        aria-label={chatButton}
        onclick={() => theatre.toggleChat()}
        use:tooltip={{ label: chatButton }}
      >
        <Icon name={chatButton === 'Hide chat' ? 'arrowRightToBar' : 'arrowLeftToBar'} size={16} />
      </button>
    {/if}
    {#if shared.stories && account}
      <StoryViewer
        channelId={tile.channel.id}
        {account}
        name={tile.channel.name}
        avatar={info?.avatar}
        onclose={() => (shared.stories = false)}
        muted={tabs.tileMuted(tile.id)}
        tileId={tile.id}
      />
    {/if}
  </div>
{/snippet}

<style>
  .side {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--page);
    container: side / size;
  }
  /* V110-F90 A slim column leaves the stream a sliver (chat under it): hide the half-cut header and body, not draw them over the chat. */
  @container side (max-height: 59px) {
    .side > :global(*) {
      display: none;
    }
  }
  /* V110-F98 A player squeezed to no width (chat only) must not keep Tab stops nobody can see. */
  @container side (max-width: 40px) {
    .side > :global(*) {
      display: none;
    }
  }
  .body {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  .body:fullscreen {
    background: #000;
  }
  /* V110-B10 Theatre: at the player's right edge, there while the pointer is over it or the button has the keys. */
  .chat-toggle {
    position: absolute;
    top: 50%;
    right: 12px;
    z-index: 4;
    display: grid;
    place-items: center;
    width: 32px;
    height: 40px;
    margin-top: -20px;
    padding: 0;
    /* V110-B11 Square and see-through: the stream stays visible behind it. */
    background: color-mix(in srgb, var(--elevated) 65%, transparent);
    border: 1px solid color-mix(in srgb, var(--border-strong) 70%, transparent);
    border-radius: var(--radius);
    color: var(--text);
    opacity: 0;
    transition: opacity var(--fast) var(--ease);
  }
  .body:hover .chat-toggle:not(.resting),
  .chat-toggle:focus-visible {
    opacity: 1;
  }
  .chat-toggle:hover {
    background: color-mix(in srgb, var(--elevated) 90%, transparent);
  }
  .body:fullscreen .chat-toggle {
    display: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .chat-toggle {
      transition: none;
    }
  }
  .popped {
    display: flex;
    flex-wrap: wrap; /* V110-F101 stacks when narrow */
    align-items: center;
    gap: 4px 8px;
    padding: 3px 12px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.8rem;
  }
  .popped button {
    flex: none;
    white-space: nowrap;
    padding: 1px 8px;
    background: var(--accent-dim);
    border: 0;
    border-radius: var(--radius);
    color: var(--accent-text);
    font-size: 0.75rem;
    font-weight: 600;
  }
</style>
