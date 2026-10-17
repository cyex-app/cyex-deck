<!-- V106-07 A feed tab of its own kind: live notifications, AutoMod or whispers. Rust keeps
     the list for the session; this shows it and applies the events. -->
<script lang="ts">
  import { onDestroy, untrack, type Snippet } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';

  import Button from '../../components/Button.svelte';
  import FeedList from '../../components/FeedList.svelte';
  import type { IconName } from '../../components/icons';
  import type { FeedEntry } from '../../ipc/bindings/FeedEntry';
  import type { FeedKind } from '../../ipc/bindings/FeedKind';
  import { feedsIpc } from '../../ipc/feeds';
  import { streamer } from '../../lib/streamer.svelte';
  import { whispers } from '../whispers/whispers.svelte';
  import { apply, keyOf, tsOf } from './entries';
  import FeedHeader from './FeedHeader.svelte';
  import HeldRow from './HeldRow.svelte';
  import LiveRow from './LiveRow.svelte';
  import WhisperRow from './WhisperRow.svelte';

  interface Props {
    kind: FeedKind;
    name: string;
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
  }

  let { kind, name, actions, onheaderdown }: Props = $props();

  const CAP: Record<string, number> = { live: 300, automod: 200, whispers: 200 };
  const ICON: Record<string, IconName> = { live: 'radio', automod: 'shield', whispers: 'whisper' };
  const EMPTY: Record<string, string> = {
    live: 'Nothing yet. Channels in your tabs, and the ones you follow while live alerts are on, show up here when they go live or offline.',
    automod: 'Nothing held right now. Held messages from chats you moderate show up here.',
    whispers: 'No whispers yet this session.',
  };

  let entries = $state.raw<FeedEntry[]>([]);
  let failed = $state(false);
  let view = 0;
  let closed = false;
  const revealed = new SvelteSet<string>();

  // A5 A revealed whisper hides again when streamer mode turns on.
  $effect(() => {
    if (streamer.active) untrack(() => revealed.clear());
  });

  function open() {
    failed = false;
    feedsIpc
      .open(kind, (e) => (entries = apply(entries, e, CAP[kind] ?? 200)))
      .then((o) => {
        if (closed) return void feedsIpc.close(o.view);
        view = o.view;
        entries = o.entries.slice(0, CAP[kind] ?? 200);
      })
      .catch(() => !closed && (failed = true));
  }
  // svelte-ignore state_referenced_locally
  open();

  onDestroy(() => {
    closed = true;
    if (view) void feedsIpc.close(view);
  });
</script>

<section class="feed">
  <FeedHeader icon={ICON[kind] ?? 'list'} {name} {actions} {onheaderdown}>
    {#snippet extra()}
      {#if entries.length > 0}
        <Button small variant="ghost" onclick={() => void feedsIpc.clear(kind)}>Clear</Button>
      {/if}
    {/snippet}
  </FeedHeader>
  <div class="body">
    {#if entries.length === 0}
      <p class="empty">
        {#if failed}
          Couldn’t load this feed. <Button variant="link" small onclick={open}>Retry</Button>
        {:else}
          {EMPTY[kind] ?? ''}
          {#if kind === 'whispers'}<Button variant="link" small onclick={() => whispers.show()}>Open whispers</Button>{/if}
        {/if}
      </p>
    {:else}
      <FeedList rows={entries.map((e) => ({ key: keyOf(e), ts: tsOf(e), kind: e.type, e }))} empty="">
        {#snippet row(r)}
          {#if r.e.type === 'live'}
            <LiveRow entry={r.e} />
          {:else if r.e.type === 'held'}
            <HeldRow
              account={r.e.account}
              login={r.e.channel}
              item={r.e.item}
              status={r.e.status}
              by={r.e.by}
              channel={r.e.channel}
            />
          {:else}
            {@const w = r.e}
            <WhisperRow
              entry={w}
              revealed={revealed.has(w.key)}
              onreveal={(on) => (on ? revealed.add(w.key) : revealed.delete(w.key))}
            />
          {/if}
        {/snippet}
      </FeedList>
    {/if}
  </div>
</section>

<style>
  .feed {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--page);
  }
  .body {
    flex: 1;
    overflow-y: auto;
  }
  .empty {
    margin: 0;
    padding: 10px 12px;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
