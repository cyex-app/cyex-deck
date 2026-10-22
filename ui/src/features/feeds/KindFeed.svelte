<!-- V106-07 A feed tab of its own kind: live notifications, AutoMod or whispers. Rust keeps
     the list for the session; this shows it and applies the events.
     Rust sends the list oldest first. -->
<script lang="ts">
  import { onDestroy, untrack, type Snippet } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';

  import Button from '../../components/Button.svelte';
  import FeedList from '../../components/FeedList.svelte';
  import type { FeedEntry } from '../../ipc/bindings/FeedEntry';
  import type { FeedKind } from '../../ipc/bindings/FeedKind';
  import type { KindEvent } from '../../ipc/bindings/KindEvent';
  import { feedsIpc } from '../../ipc/feeds';
  import { streamer } from '../../lib/streamer.svelte';
  import { whispers } from '../whispers/whispers.svelte';
  import { ClearUndo } from './clearUndo.svelte';
  import { apply, keyOf, tsOf } from './entries';
  import FeedHeader from './FeedHeader.svelte';
  import HeldRow from './HeldRow.svelte';
  import { KIND } from './kinds';
  import LiveRow from './LiveRow.svelte';
  import WhisperRow from './WhisperRow.svelte';

  interface Props {
    kind: Exclude<FeedKind, 'rules'>;
    name: string;
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
  }

  let { kind, name, actions, onheaderdown }: Props = $props();

  const info = $derived(KIND[kind]);

  let entries = $state.raw<FeedEntry[]>([]);
  let failed = $state(false);
  let view = 0;
  let closed = false;
  let pending: KindEvent[] | null = [];
  const revealed = new SvelteSet<string>();

  // A5 A revealed whisper hides again when streamer mode turns on.
  $effect(() => {
    if (streamer.active) untrack(() => revealed.clear());
  });

  function open() {
    failed = false;
    pending = [];
    const cap = info.cap;
    feedsIpc
      .open(kind, (e) => {
        if (pending) pending.push(...e);
        else entries = apply(entries, e, cap);
      })
      .then((o) => {
        if (closed) return void feedsIpc.close(o.view);
        view = o.view;
        entries = apply([...o.entries].reverse().slice(0, cap), pending ?? [], cap);
        pending = null;
      })
      .catch(() => !closed && (failed = true));
  }
  // svelte-ignore state_referenced_locally
  open();

  /** s3-3 Clear hides the rows at once and offers Undo (V107-MF, shared with the rules feeds). */
  const undo = new ClearUndo(
    () => entries.map(keyOf),
    (keys) => feedsIpc.clear(kind, keys),
    (gone) => (entries = entries.filter((e) => !gone.has(keyOf(e)))),
  );
  const shown = $derived(undo.hidden ? entries.filter((e) => undo.shows(keyOf(e))) : entries);

  onDestroy(() => {
    undo.flush();
    closed = true;
    if (view) void feedsIpc.close(view);
  });
</script>

<section class="feed">
  <FeedHeader icon={info.icon} {name} {actions} {onheaderdown}>
    {#snippet extra()}
      {#if shown.length > 0}
        <Button small variant="ghost" onclick={undo.clear}>Clear</Button>
      {/if}
    {/snippet}
  </FeedHeader>
  <div class="body">
    {#if shown.length === 0}
      <p class="empty">
        {#if failed}
          Couldn’t load this feed. <Button variant="link" small onclick={open}>Retry</Button>
        {:else}
          {info.empty}
          {#if kind === 'whispers'}<Button variant="link" small onclick={() => whispers.show()}>Open whispers</Button>{/if}
        {/if}
      </p>
    {:else}
      <FeedList rows={shown.map((e) => ({ key: keyOf(e), ts: tsOf(e), kind: e.type, e }))} empty="">
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
    font-size: var(--text-sub);
  }
</style>
