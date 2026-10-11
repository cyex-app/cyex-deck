<!-- 1.35 The stream manager: your own channel at a glance and the tools to run it. A fixed
     view in the main window that pops out (route `streammanager`), e.g. onto a second
     screen. Needs a Twitch sign-in for your channel (D30). Your chat is a panel (V102-22),
     a view on the room a tab may already have open, with its own scroll. V103-06 The
     panels are the shared panel layout (`PanelLayout`): unlocked, they drag onto a side of
     another and resize; locked, neither. Closed ones wait in the corner. Too narrow, they
     go one under the other (V102-24). V103-15 Mod actions have their own panel. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import type { PanelCtl } from '../../components/panel';
  import Card from '../../components/PanelCard.svelte';
  import PanelLayout from '../../components/PanelLayout.svelte';
  import type { Board0 } from '../../ipc/bindings/Board0';
  import type { ModActionFilters } from '../../ipc/bindings/ModActionFilters';
  import { modIpc } from '../../ipc/moderation';
  import { asPatch, type Box, type Edge, fitTree, has, leaf, minOf, move, type Node, prune, remove, restore, setSizes, split, stacked } from '../../lib/panels';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { uptimeShort } from '../browse/lists';
  import ChatView from '../chat/ChatView.svelte';
  import ModActions from '../modview/ModActions.svelte';
  import ActivityCard from './ActivityCard.svelte';
  import AdsCard from './AdsCard.svelte';
  import InfoCard from './InfoCard.svelte';
  import MarkersCard from './MarkersCard.svelte';
  import ModesCard from './ModesCard.svelte';
  import PollCard from './PollCard.svelte';
  import PredictionCard from './PredictionCard.svelte';
  import RaidCard from './RaidCard.svelte';
  import StreamCard from './StreamCard.svelte';
  import { sm } from './streammanager.svelte';
  import TogetherCard from './TogetherCard.svelte';

  interface Props {
    /** Shown in its own window. */
    popout?: boolean;
    route?: string;
    onpopout?: () => void;
    onsignin?: () => void;
  }

  let { popout = false, onpopout, onsignin }: Props = $props();

  $effect(() => {
    void sm.open();
    return () => sm.close();
  });

  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(t);
  });

  const m = $derived(sm.manager);
  const s = $derived(sm.stats);
  const cfg = $derived(settings.v.streammanager);
  const locked = $derived(cfg.locked);

  type Id = 'stream' | 'info' | 'activity' | 'actions' | 'modes' | 'poll' | 'prediction' | 'raid' | 'ads' | 'markers' | 'together' | 'chat';
  const INFO: Record<Id, { title: string; hint: string; min: Box; tall: number }> = {
    stream: { title: 'Stream', hint: 'Preview, viewers, followers, subs.', min: { w: 220, h: 200 }, tall: 320 },
    info: { title: 'Stream info', hint: 'Title, category, tags.', min: { w: 220, h: 160 }, tall: 280 },
    activity: { title: 'Activity', hint: 'Follows, subs, cheers, raids, rewards.', min: { w: 240, h: 150 }, tall: 320 },
    actions: { title: 'Mod actions', hint: 'Bans, timeouts, deletes and more.', min: { w: 240, h: 150 }, tall: 300 },
    modes: { title: 'Chat settings', hint: 'Slow, followers-only, sub-only, emote-only.', min: { w: 200, h: 110 }, tall: 220 },
    poll: { title: 'Poll', hint: 'Start or end a poll.', min: { w: 200, h: 110 }, tall: 280 },
    prediction: { title: 'Prediction', hint: 'Start, lock or pay out.', min: { w: 200, h: 110 }, tall: 300 },
    raid: { title: 'Raid', hint: 'Raid another channel.', min: { w: 200, h: 110 }, tall: 240 },
    ads: { title: 'Ads', hint: 'Run an ad, snooze, schedule.', min: { w: 200, h: 110 }, tall: 260 },
    markers: { title: 'Markers', hint: 'Mark moments for your VOD.', min: { w: 200, h: 110 }, tall: 200 },
    together: { title: 'Stream Together', hint: 'Guests and join requests.', min: { w: 200, h: 110 }, tall: 260 },
    chat: { title: 'Chat', hint: 'Your chat, as you.', min: { w: 240, h: 200 }, tall: 460 },
  };
  /** Page order; never changes. The chat is last, so stacked it's at the bottom. */
  const ORDER: Id[] = ['stream', 'info', 'modes', 'activity', 'actions', 'poll', 'prediction', 'raid', 'ads', 'markers', 'together', 'chat'];
  /** Where every panel sits by default; a closed one comes back here. */
  const full = (chatShare: number): Node =>
    split(
      'row',
      [
        split(
          'row',
          [
            split('column', [leaf('stream'), leaf('info'), leaf('modes')], [0.4, 0.35, 0.25]),
            split('column', [leaf('activity'), leaf('actions')], [0.6, 0.4]),
            split('column', [leaf('poll'), leaf('prediction'), leaf('raid'), leaf('ads'), leaf('markers'), leaf('together')]),
          ],
          [0.36, 0.37, 0.27],
        ),
        leaf('chat'),
      ],
      [1 - chatShare, chatShare],
    );
  /** Open the rest from the corner (assumption, DECISIONS.md). */
  const START_CLOSED: string[] = ['markers', 'together'];
  const GAP = 8;
  const PAD = 8;
  const min = (id: string) => INFO[id as Id]?.min ?? { w: 200, h: 110 };

  const FULL = $derived(full(cfg.chat.share));
  /** The arrangement; before V103-06 only the chat's place and width were kept. */
  const tree = $derived<Node | null>(
    cfg.layout ?? prune(FULL, (id) => !START_CLOSED.includes(id) && (id !== 'chat' || cfg.chat.shown)),
  );
  const closed = $derived(ORDER.filter((id) => !has(tree, id)).map((id) => ({ id, title: INFO[id].title, hint: INFO[id].hint })));
  const chatShown = $derived(has(tree, 'chat'));

  let box = $state<Box>({ w: 0, h: 0 });
  const narrow = $derived(
    tree !== null && box.w > 0 && minOf(tree, (id) => ({ w: min(id).w + GAP, h: 0 })).w - GAP + 2 * PAD > box.w,
  );
  const fitted = $derived.by(() => {
    if (!tree || box.w <= 0 || box.h <= 0) return null;
    if (narrow) return stacked(tree, box.w, (id) => INFO[id as Id]?.tall ?? 240, GAP, PAD);
    return fitTree(tree, box, { min, gap: GAP, pad: PAD });
  });

  function save(next: Node | null) {
    if (next) void settings.patch('streammanager', { layout: asPatch(next) as never });
  }

  function drop(panel: string, target: string | null, edge: Edge) {
    save(move(tree, panel, target, edge));
  }

  function even(path: number[]) {
    if (!tree || narrow) return;
    const n = path.reduce<Node | null>((x, i) => (x && 'dir' in x ? x.parts[i] : null), tree);
    if (n && 'dir' in n) save(setSizes(tree, path, n.parts.map(() => 1)));
  }

  function toggleChat() {
    save(chatShown ? remove(tree, 'chat') : restore(tree, 'chat', FULL));
  }

  function resetLayout() {
    void settings.patch('streammanager', { layout: null as never, chat: { shown: true } });
    toast.info('Stream manager layout reset.');
  }

  // V103-15 The mod actions on your channel: the same board as its mod view.
  let board = $state<Board0>({ activity: [], actions: [], held: [], suspicious: [], older: false });
  $effect(() => {
    const id = m?.accountId;
    if (!id) return;
    let stop: (() => void) | undefined;
    let gone = false;
    // Listening first, then the snapshot, so nothing in between is missed.
    void modIpc.onBoard(id, (b) => (board = b)).then((u) => {
      if (gone) return u();
      stop = u;
      modIpc.view(id).then((b) => !gone && (board = b)).catch(() => {});
    });
    return () => {
      gone = true;
      stop?.();
    };
  });

  function flip(key: keyof ModActionFilters) {
    void settings.patch('streammanager', { modActions: { [key]: !cfg.modActions[key] } });
  }
</script>

{#snippet panel(id: string, _ctl: PanelCtl)}
  {#if m}
    {#if id === 'stream'}<StreamCard />
    {:else if id === 'info'}<InfoCard />
    {:else if id === 'activity'}<ActivityCard />
    {:else if id === 'actions'}
      <ModActions items={board.actions} older={board.older} filters={cfg.modActions} onflip={flip} account={m.accountId} login={m.login} />
    {:else if id === 'poll'}<PollCard login={m.login} />
    {:else if id === 'prediction'}<PredictionCard login={m.login} />
    {:else if id === 'modes'}<ModesCard />
    {:else if id === 'ads'}<AdsCard />
    {:else if id === 'raid'}<RaidCard />
    {:else if id === 'markers'}<MarkersCard />
    {:else if id === 'together'}<TogetherCard />
    {:else if id === 'chat'}
      <Card title="Chat" flush>
        <div class="chat">
          {#key m.accountId}<ChatView login={m.login} name={m.display} account={m.accountId} />{/key}
        </div>
      </Card>
    {/if}
  {/if}
{/snippet}

<div class="sm">
  {#if !sm.ready}
    <!-- Reading what Rust has. -->
  {:else if !m}
    <div class="empty">
      <EmptyState icon="radio" text="Sign in with Twitch for your own channel to run your stream from here." />
      {#if onsignin}<Button small onclick={onsignin}>Sign in</Button>{/if}
    </div>
  {:else}
    <header>
      <Avatar src={m.avatar} name={m.display} size={28} />
      <h2>{m.display}</h2>
      {#if s?.live}
        <span class="live">Live</span>
        <span class="up">{uptimeShort(s.startedAt, now)}</span>
      {:else if s}
        <span class="off">Offline</span>
      {/if}
      {#if !locked}<span class="hint">Drag panels by their title.</span>{/if}
      <span class="spacer"></span>
      {#if cfg.layout}
        <button class="link" onclick={resetLayout}>Reset layout</button>
      {/if}
      <IconButton
        icon="chat"
        label={chatShown ? 'Hide chat' : 'Show chat'}
        hint="Your chat next to the cards."
        size={15}
        active={chatShown}
        onclick={toggleChat}
      />
      <IconButton
        icon={locked ? 'lock' : 'grid'}
        label={locked ? 'Unlock layout' : 'Lock layout'}
        hint={locked ? 'Move, resize and close panels.' : 'Stops panels from moving when you drag by accident.'}
        size={15}
        onclick={() => void settings.patch('streammanager', { locked: !locked })}
      />
      {#if !popout && onpopout}
        <IconButton icon="popout" label="Pop out" hint="Open the stream manager in its own window." size={15} onclick={onpopout} />
      {/if}
    </header>
    <PanelLayout
      key="streammanager"
      shown={fitted?.tree ?? null}
      size={fitted?.box ?? null}
      order={ORDER}
      {panel}
      unlocked={!locked}
      resizable={!locked && !narrow}
      gap={GAP}
      pad={PAD}
      bind:box
      {min}
      label={(id) => INFO[id as Id]?.title ?? id}
      tray={closed}
      empty="Every panel is closed."
      onresize={(path, sizes) => tree && !narrow && save(setSizes(tree, path, sizes))}
      onreset={(path) => even(path)}
      ondrop={drop}
      onclose={(id) => save(remove(tree, id))}
      onrestore={(id) => save(restore(tree, id, FULL))}
    />
  {/if}
</div>

<style>
  .sm {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    height: 100%;
    background: var(--page);
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
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
  .live {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--red);
    font-size: 0.75rem;
    font-weight: 700;
  }
  .live::before {
    content: '';
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--red);
  }
  .up,
  .off {
    color: var(--muted);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    overflow: hidden;
    color: var(--muted);
    font-size: 0.75rem;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .spacer {
    flex: 1;
  }
  .link {
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 0.8rem;
    white-space: nowrap;
    cursor: pointer;
  }
  .link:hover {
    color: var(--text);
  }
  .chat {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
</style>
