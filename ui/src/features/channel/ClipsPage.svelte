<!-- A channel's top clips (1.21). Signed out, Twitch lists at most 100 per filter. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import Icon from '../../components/Icon.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import { CLIP_DEF } from '../../components/menus/defs/clip';
  import { menuEntries } from '../../components/menus/use';
  import { tooltip } from '../../components/tooltip.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { Clip } from '../../ipc/bindings/Clip';
  import type { ClipPeriod } from '../../ipc/bindings/ClipPeriod';
  import type { ClipSort } from '../../ipc/bindings/ClipSort';
  import { channelIpc } from '../../ipc/player';
  import { formatCount, formatDate } from '../../lib/format';
  import { clipSaves } from '../player/clipSave.svelte';
  import { clock } from '../player/time';
  import { uniqueKeys } from '../../lib/keys';
  import Card from './Card.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';
  import { copy } from '../../lib/clipboard';

  let { login, onplay }: { login: string; onplay: (slug: string) => void } = $props();

  const PERIODS = [
    { value: 'day', label: '24h' },
    { value: 'week', label: '7d' },
    { value: 'month', label: '30d' },
    { value: 'all', label: 'All' },
  ] as const;
  const SORTS = [
    { value: 'views', label: 'Top' },
    { value: 'trending', label: 'Trending' },
  ] as const;

  let period = $state<ClipPeriod>('week');
  let sort = $state<ClipSort>('views');
  let clips = $state<Clip[] | null>(null);
  const clipKeys = $derived(uniqueKeys(clips ?? [], (c) => c.slug));
  let problem = $state<string | null>(null);
  /** V108-F45 Try again: a new number loads the list once more. */
  let attempt = $state(0);

  $effect(() => {
    const [l, p, s] = [login, period, sort];
    void attempt;
    let alive = true;
    clips = null;
    problem = null;
    channelIpc
      .clips(l, p, s)
      .then((list) => alive && (clips = list))
      .catch((e) => alive && (problem = errorText(e)));
    return () => (alive = false);
  });

  /** V108-N02 Save the clip as MP4 (the best file), or stop that. */
  const toggleSave = (slug: string) => (clipSaves.has(slug) ? clipSaves.cancel(slug) : void clipSaves.save(slug));

  function menu(e: MouseEvent, c: Clip) {
    e.preventDefault();
    const saving = clipSaves.has(c.slug);
    openMenu(
      e,
      menuEntries(
        CLIP_DEF,
        {
          saving: saving ? clipSaves.text(c.slug) : null,
          rename: false,
          editOnTwitch: false,
          remove: false,
          act: {
            play: () => onplay(c.slug),
            save: () => void clipSaves.save(c.slug),
            stop: () => clipSaves.cancel(c.slug),
            copyLink: () => void copy(`https://clips.twitch.tv/${c.slug}`, 'Link'),
            rename: () => {},
            edit: () => {},
            remove: () => {},
          },
        },
        { values: { nick: c.clippedBy ?? '', chan: login } },
      ),
    );
  }
</script>

<div class="page">
  <div class="filters">
    <Segmented label="Time" options={PERIODS} value={period} onchange={(v) => (period = v)} />
    <Segmented label="Order" options={SORTS} value={sort} onchange={(v) => (sort = v)} />
  </div>
  {#if problem}
    <EmptyState icon="alert" text={problem}>
      <Button small onclick={() => attempt++}>Try again</Button>
    </EmptyState>
  {:else if !clips}
    <Loading cards={12} kind="media" label="Loading" />
  {:else if !clips.length}
    <EmptyState icon="clapper" text="No clips in this time." />
  {:else}
    <div class="grid">
      {#each clips as c, i (clipKeys[i])}
        <!-- The right-click menu is the same as the button's; the card itself stays one button. -->
        <div class="cell" role="presentation" oncontextmenu={(e) => menu(e, c)}>
          <Card
            image={c.thumbnail}
            title={c.title}
            sub={[c.clippedBy && `by ${c.clippedBy}`, formatDate(new Date(c.createdAt))].filter(Boolean).join(' · ')}
            corner={clock(c.duration)}
            views={formatCount(c.views, 'view')}
            onclick={() => onplay(c.slug)}
          />
          <button
            class="save"
            class:busy={clipSaves.has(c.slug)}
            aria-label={clipSaves.has(c.slug) ? 'Stop saving' : 'Save as MP4'}
            use:tooltip={clipSaves.has(c.slug) ? { label: 'Stop saving', hint: clipSaves.text(c.slug) } : { label: 'Save as MP4' }}
            onclick={() => toggleSave(c.slug)}
          >
            <Icon name={clipSaves.has(c.slug) ? 'x' : 'download'} size={14} />
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px;
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
  }
  .cell {
    position: relative;
    min-width: 0;
  }
  /* V108-N02 Top right of the picture; shown on hover and focus, and while it saves. */
  .save {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    background: var(--scrim-strong);
    border: 0;
    border-radius: var(--radius);
    color: var(--on-video);
    opacity: 0;
    transition: opacity var(--fast) var(--ease);
  }
  .cell:hover .save,
  .cell:focus-within .save,
  .save.busy {
    opacity: 1;
  }
  .save:active {
    opacity: 0.6;
    transition: none;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(220px, 100%), 1fr));
    gap: 14px 12px;
  }
</style>
