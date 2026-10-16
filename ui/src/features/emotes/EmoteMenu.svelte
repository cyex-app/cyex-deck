<!-- 1.40 Emote menu: favorites and most used (C12), Twitch by channel and kind with the
     channel's locked emotes (E5), 7TV, BTTV, FFZ, and emoji (E1). Click puts an emote in
     the message box; right-click keeps it in favorites. -->
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
  import { PROVIDER_NAMES, emoteKey, emoteRatio } from './emotes';
  import { PROVIDER_TABS, type ProviderTab, cellKey, pick, search, usable } from './menu';
  import { emoteMenu } from './menus';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  interface Props {
    login: string;
    account: string | null;
    oninsert: (text: string) => void;
  }

  let { login, account, oninsert }: Props = $props();

  type Tab = 'favorites' | ProviderTab | 'emoji';
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
  const tabs = $derived<Tab[]>([
    'favorites',
    ...PROVIDER_TABS.filter((t) => t === 'twitch' || (menu?.[t].length ?? 0) > 0),
    'emoji',
  ]);
  const found = $derived(menu && query.trim() ? search(menu, query) : []);
  const foundEmoji = $derived(emoji && query.trim() ? findEmoji(emoji, query.trim(), 40) : []);

  onMount(() => {
    // A tip shown for one of the cells shouldn't outlive the menu.
    return hideTip;
  });

  onMount(() => {
    emoteMenu(login, account)
      .then((m) => (menu = m))
      .catch((e) => (error = errorText(e)));
  });

  $effect(() => {
    if ((tab === 'emoji' || query.trim()) && !emoji) void emojiGroups().then((g) => (emoji = g));
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
    return PROVIDER_NAMES[t];
  }

  function toggleFavorite(e: MouseEvent, key: EmoteKey) {
    e.preventDefault();
    const k = emoteKey(key.source, key.id);
    const on = favorite.has(k);
    openMenu(e, [
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
  <button
    class="cell"
    class:locked={!!m.locked}
    data-tip={m.name}
    data-hint={m.locked ?? PROVIDER_NAMES[m.provider]}
    data-tip-img={emoteLarge(m.provider, m.id, theme, { animated: m.animated })}
    onclick={() => !m.locked && oninsert(m.name)}
    oncontextmenu={(e) => toggleFavorite(e, { source: m.provider, id: m.id, name: m.name })}
  >
    <img
      src={img.src}
      alt={m.name}
      style:aspect-ratio={emoteRatio(m.size)}
      loading="lazy"
      decoding="async"
    />
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
    oncontextmenu={(ev) => toggleFavorite(ev, { source: 'emoji', id: char, name })}>{char}</button
  >
{/snippet}

<div class="menu" role="dialog" aria-label="Emotes">
  <div class="top">
    <SearchField bind:value={query} placeholder="Find an emote" small autofocus />
  </div>
  <div class="tabs" role="tablist">
    {#each tabs as t (t)}
      <button role="tab" aria-selected={tab === t && !query} class:on={tab === t && !query} onclick={() => choose(t)}>
        {#if t === 'favorites'}<Icon name="star" size={13} />{:else if t === 'emoji'}<Icon name="smile" size={13} />{/if}
        {label(t)}
      </button>
    {/each}
  </div>

  <!-- svelte-ignore a11y_mouse_events_have_key_events -->
  <div class="body" {onmouseover} {onmouseout} role="presentation">
    {#if error}
      <p class="note">{error}</p>
    {:else if !menu}
      <Loading />
    {:else if query.trim()}
      {#if found.length || foundEmoji.length}
        <div class="grid">
          {#each found as m (cellKey(m))}{@render emote(m)}{/each}
          {#each foundEmoji as e (e.char)}{@render emojiCell(e)}{/each}
        </div>
      {:else}
        <p class="note">Nothing called “{query.trim()}”.</p>
      {/if}
    {:else if tab === 'favorites'}
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
    {:else if tab === 'emoji'}
      {#if emoji}
        {#each emoji as g (g.title)}
          <section>
            <h4>{g.title}</h4>
            <div class="grid">
              {#each g.emoji.slice(0, cap(g.title)) as e (e.char)}{@render emojiCell(e)}{/each}
            </div>
            {@render more(g.title, g.emoji.length)}
          </section>
        {/each}
      {:else}
        <Loading />
      {/if}
    {:else}
      {#if tab === 'twitch' && menu.twitchNote}<p class="note">{menu.twitchNote}</p>{/if}
      {#each menu[tab] as s (s.id)}
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
      {#if !menu[tab].length && !(tab === 'twitch' && menu.twitchNote)}
        <p class="note">No {PROVIDER_NAMES[tab]} emotes here.</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .menu {
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
    overflow-y: auto;
    padding: 4px 8px 8px;
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
  .locked {
    cursor: default;
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
