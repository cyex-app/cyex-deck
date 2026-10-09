<!-- Top of a tile's stream side: avatar (with the stories ring), what's on, and the
     switch between the stream and the channel's pages (1.21). Carries the tile's buttons
     and drag handle when the chat isn't next to it. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import type { IconName } from '../../components/icons';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { formatShort } from '../../lib/format';
  import type { Page } from '../player/players.svelte';
  import StoryRing from '../player/StoryRing.svelte';
  import { uptime } from '../player/time';

  interface Props {
    tile: Tile;
    info: ChannelInfo | undefined;
    page: Page;
    onpage: (p: Page) => void;
    onstories: () => void;
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
  }

  let { tile, info, page, onpage, onstories, actions, onheaderdown }: Props = $props();

  const stream = $derived(info?.stream);
  const PAGES: { kind: 'stream' | 'about' | 'clips' | 'videos'; label: string; icon: IconName }[] = [
    { kind: 'stream', label: 'Stream', icon: 'radio' },
    { kind: 'about', label: 'About', icon: 'info' },
    { kind: 'clips', label: 'Clips', icon: 'clapper' },
    { kind: 'videos', label: 'Videos', icon: 'film' },
  ];
  /** A clip or VOD from a list counts as that list. */
  const current = $derived(page.kind === 'vod' ? 'videos' : page.kind === 'clip' ? 'clips' : page.kind);

  let minute = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (minute = Date.now()), 30_000);
    return () => clearInterval(timer);
  });
</script>

<!-- The header drags a tile in a tiled tab (U3); its buttons stay clickable. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<header onpointerdown={onheaderdown}>
  <StoryRing
    channelId={tile.channel.id}
    account={tile.account}
    avatar={info?.avatar}
    name={tile.channel.name}
    onopen={onstories}
    onplain={() => onpage({ kind: 'about' })}
  />
  <div class="what">
    {#if stream}
      <span class="title" use:tooltip={{ label: stream.title, hint: stream.game ?? undefined }}>
        <span class="dot" class:rerun={stream.rerun}></span>{stream.title || tile.channel.name}
      </span>
      <span class="meta">
        {[stream.game, `${formatShort(stream.viewers)} watching`, uptime(stream.startedAt, minute)].filter(Boolean).join(' · ')}
      </span>
    {:else}
      <span class="title">{tile.channel.name}</span>
      <span class="meta">Offline</span>
    {/if}
  </div>
  <nav aria-label="Channel pages">
    {#each PAGES as p (p.kind)}
      <button class:on={current === p.kind} aria-label={p.label} use:tooltip={p.label} onclick={() => onpage({ kind: p.kind })}>
        <Icon name={p.icon} size={14} /><span class="name">{p.label}</span>
      </button>
    {/each}
  </nav>
  {@render actions?.()}
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 6px 0 8px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    container-type: inline-size;
  }
  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .title,
  .meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 700;
    font-size: 0.85rem;
  }
  .meta {
    color: var(--dim);
    font-size: 0.72rem;
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #e91916;
    box-shadow: 0 0 6px rgba(233, 25, 22, 0.8);
  }
  .dot.rerun {
    background: var(--yellow);
    box-shadow: none;
  }
  nav {
    display: flex;
    gap: 2px;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 7px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 600;
  }
  nav button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  nav button.on {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  @container (max-width: 560px) {
    .name {
      display: none;
    }
  }
</style>
