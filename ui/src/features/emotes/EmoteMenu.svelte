<!-- 1.40 Emote menu: favorites and most used (C12), Twitch by channel and kind with the
     channel's locked emotes (E5), 7TV, BTTV, FFZ, and emoji (E1). Click puts an emote in
     the message box; right-click keeps it in favorites. Sub emotes above tier 1 carry a
     tier mark; a locked one opens its card instead (1.41). GIFs (1.43) get a tab where
     this account can send them. Arrow keys move between cells. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import { hideTip, showTip } from '../../components/tooltip.svelte';
  import type { EmoteKey } from '../../ipc/bindings/EmoteKey';
  import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
  import type { MenuEmote } from '../../ipc/bindings/MenuEmote';
  import { density } from '../../lib/density.svelte';
  import { focus } from '../../lib/focus.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { emoteImg, emoteLarge } from '../chat/urls';
  import { emojiGroups, emojiLabel, findEmoji, type Emoji, type EmojiGroup } from './emoji';
  import { type CardTarget, PROVIDER_NAMES, emoteKey, emoteRatio } from './emotes';
  import { PROVIDER_TABS, type ProviderTab, cellHint, cellKey, pick, search, tierMark, usable } from './menu';
  import EmoteCard from './EmoteCard.svelte';
  import { emoteMenu, forgetEmoteMenu } from './menus';
  import EditBar from './edit/EditBar.svelte';
  import { emoteEditMenu } from './edit/menu';
  import { type EditForm, removeHere } from './edit/run';
  import { errorText } from '../../lib/errors';
  import { uniqueKeys } from '../../lib/keys';
  import Loading from '../../components/Loading.svelte';
  import type { GifInfo } from '../../ipc/bindings/GifInfo';
  import { chatxIpc } from '../../ipc/chatx';
  import { ARROWS, moveFocus } from '../../lib/gridNav';
  import type { ChatRoom } from '../chat/room.svelte';
  import GifPicker from './GifPicker.svelte';
  import { showGifTab } from './gifs';

  interface Props {
    login: string;
    account: string | null;
    oninsert: (text: string) => void;
    /** 1.43 The chat this menu sends GIFs to; none in whispers. */
    room?: ChatRoom;
    onclose?: () => void;
  }

  let { login, account, oninsert, room, onclose }: Props = $props();

  type Tab = 'favorites' | ProviderTab | 'emoji' | 'gifs';
  /** V101-12 Twitch the first time, then the tab used last. A new key starts everyone
   * on Twitch once. */
  const TAB_KEY = 'cyex.emoteTab';

  let menu = $state<EmoteMenu | null>(null);
  let error = $state('');
  let emoji = $state<EmojiGroup[] | null>(null);
  let query = $state('');
  let tab = $state<Tab>(readTab());
  /** Big sections draw this many cells, then a "Show more" (nothing else is windowed here). */
  const CAP = 240;
  let extra = $state<Record<string, number>>({});
  const cap = (id: string) => CAP + (extra[id] ?? 0);

  const theme = $derived(settings.v.appearance.theme);
  const saved = $derived(settings.v.emotes);
  const have = $derived(menu ? usable(menu) : new Map<string, MenuEmote>());
  const favEmotes = $derived(pick(saved.favorites, have));
  const favEmoji = $derived(saved.favorites.filter((k) => k.source === 'emoji'));
  const used = $derived(pick(saved.usage.map((u) => u.emote), have).slice(0, 30));
  const favorite = $derived(new Set(saved.favorites.map((k) => emoteKey(k.source, k.id))));
  /** 1.43 `undefined` while asking. */
  let gifInfo = $state<GifInfo | null | undefined>(undefined);
  const gifsOk = $derived(showGifTab(gifInfo, room?.state.emoteOnly ?? false));
  const tabs = $derived<Tab[]>([
    'favorites',
    ...PROVIDER_TABS.filter((t) => t === 'twitch' || (menu?.[t].length ?? 0) > 0),
    'emoji',
    ...(gifsOk ? (['gifs'] as const) : []),
  ]);
  /** The last tab, unless it isn't here (GIFs in another channel). GIFs wait for their answer. */
  const current = $derived<Tab>(tabs.includes(tab) || (tab === 'gifs' && gifInfo === undefined) ? tab : 'twitch');
  const searchingEmotes = $derived(!!query.trim() && current !== 'gifs');
  const found = $derived(menu && query.trim() ? search(menu, query) : []);
  const foundEmoji = $derived(emoji && query.trim() ? findEmoji(emoji, query.trim(), 40) : []);
  const foundEmojiKeys = $derived(uniqueKeys(foundEmoji, (e) => e.char));

  onMount(() => {
    // A tip current for one of the cells shouldn't outlive the menu.
    return hideTip;
  });

  function load() {
    emoteMenu(login, account)
      .then((m) => (menu = m))
      .catch((e) => (error = errorText(e)));
  }

  onMount(load);

  onMount(() => {
    if (!room?.view) return void (gifInfo = null);
    chatxIpc
      .gifInfo(room.view)
      .then((i) => (gifInfo = i))
      .catch(() => (gifInfo = null));
  });

  /** Down from the search box into the cells; arrows between them; up from the top row
   * back to the search box. Cells come from the one section and its neighbours. */
  function onkeydown(e: KeyboardEvent) {
    const dir = ARROWS[e.key];
    if (!dir || e.altKey || e.ctrlKey || e.metaKey) return;
    const t = e.target as HTMLElement;
    const root = e.currentTarget as HTMLElement;
    if (t instanceof HTMLInputElement && t.closest('.top')) {
      if (dir !== 'down') return;
      const first = root.querySelector<HTMLElement>('.body .cell, .body .gif');
      if (!first) return;
      e.preventDefault();
      first.focus();
      return;
    }
    const grid = t.closest('.grid');
    if (!grid || !t.matches('.cell, .gif')) return;
    e.preventDefault();
    const near = [grid.closest('section')?.previousElementSibling, grid, grid.closest('section')?.nextElementSibling];
    const cells = near.flatMap((n) => (n ? [...n.querySelectorAll<HTMLElement>('.cell, .gif')] : []));
    if (!moveFocus(cells, t, dir) && dir === 'up') root.querySelector<HTMLInputElement>('.top input')?.focus();
  }

  /** V106-31/32 The add/rename strip; the emote it is about. */
  type Editing = {
    emote: { id: string; name: string };
    mode: EditForm;
  };
  let editing = $state<Editing | null>(null);

  /** E5 A locked emote's card: it can't go in the message box. */
  let card = $state<CardTarget | null>(null);

  function pickCell(m: MenuEmote, el: HTMLElement) {
    if (!m.locked) return oninsert(m.name);
    hideTip();
    card = {
      emote: { provider: m.provider, id: m.id, name: m.name, animated: m.animated, size: m.size ?? undefined },
      stack: [],
      rect: el.getBoundingClientRect(),
    };
  }

  function edited() {
    editing = null;
    forgetEmoteMenu(login);
    load();
  }

  $effect(() => {
    if ((current === 'emoji' || searchingEmotes) && !emoji) void emojiGroups().then((g) => (emoji = g));
  });

  function readTab(): Tab {
    try {
      return (localStorage.getItem(TAB_KEY) as Tab) || 'twitch';
    } catch {
      return 'twitch';
    }
  }

  function choose(t: Tab) {
    tab = t;
    query = '';
    try {
      localStorage.setItem(TAB_KEY, t);
    } catch {
      // Only a convenience.
    }
  }

  function label(t: Tab): string {
    if (t === 'favorites') return 'Favorites';
    if (t === 'emoji') return 'Emoji';
    if (t === 'gifs') return 'GIFs';
    return PROVIDER_NAMES[t];
  }

  /** Right-click: favorite, plus the 7TV edits when there are any (V106-31/32). */
  async function openEmoteMenu(e: MouseEvent, key: EmoteKey) {
    e.preventDefault();
    const at = { clientX: e.clientX, clientY: e.clientY };
    const extra = await emoteEditMenu(account, login, { id: key.id, name: key.name, provider: key.source }, {
      add: (t, ownName) => (editing = { emote: key, mode: { kind: 'add', target: t, ownName } }),
      rename: (h) => (editing = { emote: key, mode: { kind: 'rename', here: h } }),
      remove: (h) =>
        void removeHere(account!, login, key.id, h).then((f) => {
          if (!f) edited();
        }),
    });
    if (!extra) return;
    const k = emoteKey(key.source, key.id);
    const on = favorite.has(k);
    openMenu(at, [
      {
        label: on ? 'Unfavorite' : 'Favorite',
        icon: on ? 'starFilled' : 'star',
        run: () =>
          void settings.patch('emotes', {
            favorites: on
              ? saved.favorites.filter((f) => emoteKey(f.source, f.id) !== k)
              : [...saved.favorites, key],
          }),
      },
      ...extra,
    ]);
  }

  // Tooltips are delegated, like in chat.
  function onmouseover(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[data-tip]');
    if (el) showTip(el, { label: el.dataset.tip ?? '', hint: el.dataset.hint, image: el.dataset.tipImg });
  }

  function onmouseout(e: MouseEvent) {
    const from = (e.target as HTMLElement).closest('[data-tip]');
    if (from && !from.contains(e.relatedTarget as Node)) hideTip();
  }
</script>

{#snippet more(id: string, total: number)}
  {#if total > cap(id)}
    <button class="more" onclick={() => (extra[id] = (extra[id] ?? 0) + CAP)}>Show more ({total - cap(id)})</button>
  {/if}
{/snippet}

{#snippet emote(m: MenuEmote)}
  {@const img = emoteImg(m.provider, m.id, theme, { animated: m.animated, still: focus.still, scale: density.scale })}
  {@const mark = tierMark(m)}
  <button
    class="cell"
    class:locked={!!m.locked}
    aria-label={m.locked ? `${m.name}, ${m.locked}` : undefined}
    data-tip={m.name}
    data-hint={cellHint(m)}
    data-tip-img={emoteLarge(m.provider, m.id, theme, { animated: m.animated })}
    onclick={(e) => pickCell(m, e.currentTarget)}
    oncontextmenu={(e) => openEmoteMenu(e, { source: m.provider, id: m.id, name: m.name })}
  >
    <img
      src={img.src}
      alt={m.name}
      style:aspect-ratio={emoteRatio(m.size)}
      loading="lazy"
      decoding="async"
    />
    {#if mark}<span class="tier">{mark}</span>{/if}
    {#if m.locked}<span class="lock"><Icon name="lock" size={10} /></span>{/if}
  </button>
{/snippet}

{#snippet emojiCell(e: Emoji | EmoteKey)}
  {@const char = 'char' in e ? e.char : e.id}
  {@const name = 'char' in e ? e.names[0] : e.name}
  <button
    class="cell emoji"
    data-tip={'char' in e ? emojiLabel(e) : name}
    onclick={() => oninsert(char)}
    oncontextmenu={(ev) => openEmoteMenu(ev, { source: 'emoji', id: char, name })}>{char}</button
  >
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="menu" role="dialog" aria-label="Emotes" tabindex="-1" {onkeydown}>
  <div class="top">
    <SearchField bind:value={query} placeholder={current === 'gifs' ? 'Search GIPHY' : 'Find an emote'} small autofocus />
  </div>
  <div class="tabs" role="tablist">
    {#each tabs as t (t)}
      {@const on = current === t && (t === 'gifs' || !query)}
      <button role="tab" aria-selected={on} class:on onclick={() => choose(t)}>
        {#if t === 'favorites'}<Icon name="star" size={13} />{:else if t === 'emoji'}<Icon name="smile" size={13} />{/if}
        {label(t)}
      </button>
    {/each}
  </div>

  <!-- svelte-ignore a11y_mouse_events_have_key_events -->
  <div class="body" class:gifs={current === 'gifs'} {onmouseover} {onmouseout} role="presentation">
    {#if current === 'gifs'}
      {#if gifInfo && room}
        <GifPicker view={room.view} chat="{account}:{login}" {query} onsent={() => onclose?.()} />
      {:else}
        <Loading />
      {/if}
    {:else if error}
      <p class="note">{error}</p>
    {:else if !menu}
      <Loading />
    {:else if searchingEmotes}
      {#if found.length || foundEmoji.length}
        <div class="grid">
          {#each found as m (cellKey(m))}{@render emote(m)}{/each}
          {#each foundEmoji as e, i (foundEmojiKeys[i])}{@render emojiCell(e)}{/each}
        </div>
      {:else}
        <p class="note">Nothing called “{query.trim()}”.</p>
      {/if}
    {:else if current === 'favorites'}
      {#if favEmotes.length || favEmoji.length}
        <section>
          <h4>Favorites</h4>
          <div class="grid">
            {#each favEmotes as m (cellKey(m))}{@render emote(m)}{/each}
            {#each favEmoji as e (e.id)}{@render emojiCell(e)}{/each}
          </div>
        </section>
      {/if}
      {#if used.length}
        <section>
          <h4>Most used</h4>
          <div class="grid">
            {#each used as m (cellKey(m))}{@render emote(m)}{/each}
          </div>
        </section>
      {/if}
      {#if !favEmotes.length && !favEmoji.length && !used.length}
        <p class="note">Right-click an emote to keep it here. The ones you use most show up too.</p>
      {/if}
    {:else if current === 'emoji'}
      {#if emoji}
        {#each emoji as g (g.title)}
          {@const shown = g.emoji.slice(0, cap(g.title))}
          {@const keys = uniqueKeys(shown, (e) => e.char)}
          <section>
            <h4>{g.title}</h4>
            <div class="grid">
              {#each shown as e, i (keys[i])}{@render emojiCell(e)}{/each}
            </div>
            {@render more(g.title, g.emoji.length)}
          </section>
        {/each}
      {:else}
        <Loading />
      {/if}
    {:else}
      {#if current === 'twitch' && menu.twitchNote}<p class="note">{menu.twitchNote}</p>{/if}
      {#each menu[current] as s (s.id)}
        <section>
          <h4>
            {#if s.avatar}<img class="avatar" src={s.avatar} alt="" />{/if}
            {s.title}
          </h4>
          <div class="grid">
            {#each s.emotes.slice(0, cap(s.id)) as m (cellKey(m))}{@render emote(m)}{/each}
          </div>
          {@render more(s.id, s.emotes.length)}
        </section>
      {/each}
      {#if !menu[current].length && !(current === 'twitch' && menu.twitchNote)}
        <p class="note">No {PROVIDER_NAMES[current]} emotes here.</p>
      {/if}
    {/if}
  </div>
  {#if editing && account}
    <EditBar {account} {login} emote={editing.emote} mode={editing.mode} ondone={edited} oncancel={() => (editing = null)} />
  {/if}
  {#if card}
    {#key card}<EmoteCard target={card} {login} {account} onclose={() => (card = null)} />{/key}
  {/if}
</div>

<style>
  .menu {
    position: relative;
    display: flex;
    flex-direction: column;
    width: min(380px, calc(100vw - 24px));
    height: 380px;
    /* V102-03 Shorter when the window is. */
    flex: 0 1 auto;
    min-height: 0;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
  }
  .top {
    padding: 8px 8px 6px;
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 0 8px 6px;
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
  }
  .tabs button {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 600;
    white-space: nowrap;
  }
  .tabs button:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .tabs .on,
  .tabs .on:hover {
    color: var(--accent-text);
    background: var(--accent-dim);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 8px 8px;
  }
  /* The GIF grid scrolls itself, for its next pages. */
  .body.gifs {
    display: flex;
    flex-direction: column;
    padding: 0;
    overflow: hidden;
  }
  .menu:focus {
    outline: none;
  }
  .cell:focus-visible {
    background: var(--surface-hover);
  }
  section {
    content-visibility: auto;
    contain-intrinsic-size: auto 120px;
  }
  h4 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 8px 0 4px;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .avatar {
    width: 16px;
    height: 16px;
    border-radius: var(--radius);
  }
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .cell {
    position: relative;
    display: grid;
    place-items: center;
    min-width: 36px;
    height: 36px;
    padding: 2px;
    background: none;
    border: 0;
    border-radius: var(--radius);
  }
  .cell:hover {
    background: var(--surface-hover);
  }
  .cell img {
    height: 28px;
    max-width: 84px;
  }
  .emoji {
    font-size: 22px;
    line-height: 1;
    font-family: 'Segoe UI Emoji', 'Apple Color Emoji', 'Noto Color Emoji', sans-serif;
  }
  .locked img {
    filter: grayscale(1);
    opacity: 0.45;
  }
  .tier {
    position: absolute;
    top: 1px;
    right: 1px;
    padding: 0 3px;
    background: var(--elevated-2);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.6rem;
    font-weight: 700;
    line-height: 12px;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }
  .lock {
    position: absolute;
    right: 1px;
    bottom: 1px;
    display: grid;
    place-items: center;
    width: 14px;
    height: 14px;
    background: var(--elevated-2);
    border-radius: 50%;
    color: var(--muted);
    pointer-events: none;
  }
  .more {
    margin: 4px 0 0;
    padding: 4px 8px;
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 0.8rem;
  }
  .more:hover {
    color: var(--text);
  }
  .note {
    margin: 16px 4px;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
