<!-- 1.46 The moderator view for a channel the account mods, laid out by the user (1.53):
     V103-06 the shared panel layout (`PanelLayout`), unlocked to drag panels onto a side of
     another and to resize them, locked for neither; closed panels wait in the corner. One
     layout for every channel, in the `modview` settings. Opened from the shield (1.46),
     poppable out (1.47). The chat panel (V102-22) is a view on the room the tab already has
     open, as the moderating account; with no tab on that channel it joins while it shows.
     V102-26 Inside a tab it has a close button. V103-07 The stream panel, closed at first,
     shows the tab's own player when the view sits in that tab, else a player of its own.
     V103-15 Mod actions have their own panel next to the activity. V103-22 One chat per
     channel: its chat follows the channel's regular chat, out while that shows (in the
     tab or another window), in as a whole column while it doesn't; brought back or closed
     by hand, it stays so until "Reset layout". V107-R29 Closed panels sit as icons under the
     layout, each opening a quick menu; any panel pops out into its own window and docks back
     when that closes. V107-R43 Reward requests, closed at first. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import IconButton from '../../components/IconButton.svelte';
  import { grabFrom, type PanelCtl } from '../../components/panel';
  import PanelLayout from '../../components/PanelLayout.svelte';
  import Button from '../../components/Button.svelte';
  import {
    asPatch,
    type Box,
    chatShows,
    type Edge,
    extend,
    fitTree,
    has,
    leaf,
    minOf,
    move,
    type Node,
    prune,
    remove,
    restore,
    setSizes,
    split,
    stacked,
    withChat,
  } from '../../lib/panels';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { chatShown } from '../chat/shown.svelte';
  import { ModBoard } from './board.svelte';
  import ModPanelBody from './ModPanelBody.svelte';
  import { INFO, isModPanel, type ModPanelId as Id } from './panelInfo';
  import { modPopouts } from './popouts.svelte';
  import { closedOut, dockBack, poppedKey, shownTree } from './popped';

  interface Props {
    /** The channel login; `account` mods it. */
    login: string;
    account: string;
    /** In the main window (V102-16) and in a tab (V102-26). */
    onpopout?: () => void;
    /** In a tab (V102-26). */
    onclose?: () => void;
    /** V103-07 In a tab whose tile plays the stream: that tile, to show its player. */
    host?: string | null;
    /** V103-05 In an unlocked tab: drags the mod view by its header. */
    ongrab?: (e: PointerEvent) => void;
    /** V103-22 In a tab: the tab's own chat shows (in the tile or its own window). */
    chatNear?: boolean;
  }

  let { login, account, onpopout, onclose, host = null, ongrab, chatNear = false }: Props = $props();

  /** Page order; never changes, so the chat and the stream never restart. */
  const ORDER: Id[] = ['stream', 'chat', 'activity', 'actions', 'automod', 'unban', 'chatters', 'mass', 'terms', 'rewards'];
  /** Where every panel sits by default; a closed one comes back here. */
  const FULL: Node = split(
    'row',
    [
      split('column', [leaf('stream'), leaf('chat')], [0.4, 0.6]),
      split('column', [leaf('activity'), leaf('actions'), leaf('terms')], [0.35, 0.4, 0.25]),
      split('column', [leaf('automod'), leaf('unban'), leaf('rewards'), leaf('chatters'), leaf('mass')], [0.2, 0.14, 0.2, 0.28, 0.18]),
    ],
    [0.34, 0.33, 0.33],
  );
  /** V103-07 Closed until asked for. */
  const START_CLOSED: string[] = ['stream', 'terms', 'rewards'];
  /** V103-22 The chat's column in `FULL`, kept when it opens as the only chat. */
  const CHAT_SHARE = 0.34;
  const GAP = 8;
  const PAD = 8;
  const min = (id: string) => (isModPanel(id) ? INFO[id].min : { w: 200, h: 110 });

  // svelte-ignore state_referenced_locally
  const data = new ModBoard(() => login, () => account);

  const mv = $derived(settings.v.modview);
  const locked = $derived(mv.locked);
  /** The arrangement; before V103-06 only which widgets were removed was kept. */
  const stored = $derived<Node | null>(
    mv.layout ?? prune(FULL, (id) => !START_CLOSED.includes(id) && !mv.hidden.includes(id)),
  );

  // V103-22 One chat per channel: the chat follows the channel's regular chat, the tab's own
  // right away, other windows' as Rust tells (`chatShown`). Kept by hand, it stays so.
  $effect(() => chatShown.start());
  const kept = $derived(mv.chatKept ?? null);
  const regular = $derived(chatNear ? true : chatShown.elsewhere(login));
  const tree = $derived(withChat(stored, 'chat', chatShows(kept, regular), 'left', CHAT_SHARE));

  // V107-R29 Panels in a window of their own don't show here; closing it puts them back
  // where they came from: their exact place in the layout (`mv.popped`), else the closed
  // row. `going`: asked to pop out, its window not heard of yet.
  $effect(() => modPopouts.start());
  let going = $state.raw<Set<string>>(new Set());
  const out = $derived(new Set([...modPopouts.of(login), ...going]));
  $effect(() => {
    const now = modPopouts.of(login);
    if ([...going].some((id) => now.has(id))) going = new Set([...going].filter((id) => !now.has(id)));
  });
  const view = $derived(shownTree(tree, out));
  $effect(() => {
    if (!modPopouts.ready) return;
    const popped = mv.popped ?? {};
    const gone = closedOut(popped, modPopouts.routes).filter((k) => !going.has(k.slice(k.indexOf('/') + 1)) || !k.startsWith(`${login}/`));
    if (!gone.length) return;
    untrack(() => {
      const still = new Set(Object.keys(popped).filter((k) => !gone.includes(k)).map((k) => k.slice(k.indexOf('/') + 1)));
      let t = tree;
      for (const k of gone) {
        const id = k.slice(k.indexOf('/') + 1);
        if (!has(t, id)) t = dockBack(t, id, popped[k], still).tree;
      }
      const drop = Object.fromEntries(gone.map((k) => [k, null]));
      void settings.patch('modview', { layout: asPatch(t) as never, popped: drop as never });
    });
  });
  const closed = $derived(
    ORDER.filter((id) => !has(tree, id) && !out.has(id)).map((id) => ({ id, ...INFO[id] })),
  );

  // Fitted to the room: the panels' least sizes kept; too narrow for the layout, one
  // column that scrolls (as the old grid did), which can't be moved or resized.
  let box = $state<Box>({ w: 0, h: 0 });
  const narrow = $derived(
    view !== null && box.w > 0 && minOf(view, (id) => ({ w: min(id).w + GAP, h: 0 })).w - GAP + 2 * PAD > box.w,
  );
  const fitted = $derived.by(() => {
    if (!view || box.w <= 0 || box.h <= 0) return null;
    if (narrow) return stacked(view, box.w, (id) => (isModPanel(id) ? INFO[id].tall : 240), GAP, PAD);
    return fitTree(view, box, { min, gap: GAP, pad: PAD });
  });

  /** What shows is saved. `chat`: the chat brought back or closed by hand (V103-22). */
  function save(next: Node | null, chat?: boolean) {
    if (!next) return;
    void settings.patch('modview', { layout: asPatch(next) as never, ...(chat === undefined ? {} : { chatKept: chat }) });
  }

  function drop(panel: string, target: string | null, edge: Edge) {
    // From the corner it's brought back by hand.
    save(move(view, panel, target, edge), panel === 'chat' && !has(tree, 'chat') ? true : undefined);
  }

  /** V103-22 The chat comes back as a whole column, as it opens by itself. */
  function bring(id: string) {
    if (id === 'chat') save(extend(view, 'chat', 'left', CHAT_SHARE), true);
    else save(restore(view, id, FULL));
  }

  /** V107-R29 Into its own window. Out of the layout, it leaves it, and the layout as it
   *  was is kept to put it back exactly; from the closed row it just stays closed. The
   *  chat stays in the layout, hidden, as it follows its own rule (V103-22). */
  function popOut(id: string) {
    if (!isModPanel(id)) return;
    const from = id !== 'chat' && has(tree, id) ? tree : null;
    going = new Set([...going, id]);
    void modPopouts.open(login, id, INFO[id].title).then(async (ok) => {
      if (ok && from) {
        // Its window is known first, so no view puts it straight back.
        await modPopouts.refresh();
        await settings.patch('modview', {
          layout: asPatch(remove(from, id)) as never,
          popped: { [poppedKey(login, id)]: from } as never,
        });
      }
      going = new Set([...going].filter((g) => g !== id));
    });
  }

  function resize(path: number[], sizes: number[]) {
    if (view && !narrow) save(setSizes(view, path, sizes));
  }

  function even(path: number[]) {
    if (!view || narrow) return;
    const s = path.reduce<Node | null>((n, i) => (n && 'dir' in n ? n.parts[i] : null), view);
    if (s && 'dir' in s) save(setSizes(view, path, s.parts.map(() => 1)));
  }

  function resetLayout() {
    void settings.patch('modview', { order: [], hidden: [], layout: null as never, chatKept: null as never, popped: Object.fromEntries(Object.keys(mv.popped ?? {}).map((k) => [k, null])) as never });
    toast.success('Mod view layout reset.');
  }
</script>

{#snippet panel(id: string, _ctl: PanelCtl)}
  <ModPanelBody {id} {login} {account} {data} {host} />
{/snippet}

{#snippet quick(id: string)}
  <ModPanelBody {id} {login} {account} {data} />
{/snippet}

<div class="modview">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header onpointerdown={grabFrom(ongrab)} class:grab={!!ongrab}>
    <h2>Mod view · {login}</h2>
    {#if !locked}<span class="hint">Drag panels by their title.</span>{/if}
    {#if data.problem}
      <span class="hint bad">{data.problem}</span>
      <Button small variant="link" onclick={() => data.retry++}>Retry</Button>
    {/if}
    <div class="spacer"></div>
    {#if mv.layout || mv.hidden.length || mv.chatKept != null}
      <Button small variant="link" onclick={resetLayout}>Reset layout</Button>
    {/if}
    <IconButton
      icon={locked ? 'lock' : 'lockOpen'}
      label={locked ? 'Unlock layout' : 'Lock layout'}
      hint={locked ? 'Move, resize and close panels.' : 'Stops panels from moving when you drag by accident.'}
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

  <PanelLayout
    key="modview-{login}-{host ?? 'own'}"
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
    label={(id) => (isModPanel(id) ? INFO[id].title : id)}
    tray={closed}
    empty="Every panel is closed."
    onresize={resize}
    onreset={(path) => even(path)}
    ondrop={drop}
    onclose={(id) => save(remove(view, id), id === 'chat' ? false : undefined)}
    onrestore={bring}
    onpopout={popOut}
    {quick}
  />
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
  header.grab {
    cursor: grab;
  }
  h2 {
    margin: 0;
    font-size: 0.95rem;
    white-space: nowrap;
  }
  .hint {
    overflow: hidden;
    color: var(--muted);
    font-size: 0.75rem;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .bad {
    color: var(--red);
  }
  .spacer {
    flex: 1;
  }
</style>
