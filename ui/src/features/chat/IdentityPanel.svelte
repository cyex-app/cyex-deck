<!-- 1.40 V107-Mk Chat identity, as on twitch.tv: the badge you wear (everywhere, or with
     V107-Mx's switch one just for this channel: any badge you have, V107-H11a), your name color, and
     with Prime or Turbo any color and a smiley style. Every change is read back from Twitch.
     Shown in the chat box's popover and the account chip's dialog.
     V108-ID A fixed size (the window's height caps it), so nothing grows, shrinks or jumps while
     it loads, switches or picks: the header stays put and one area scrolls under it. The badge
     worn everywhere comes first, the channel switch under it opens its grid right below. Badges,
     colors and smileys share one 8-column grid; arrows move in a grid, Enter or Space picks. -->
<script lang="ts">
  import { tick, untrack } from 'svelte';

  import Button from '../../components/Button.svelte';
  import Loading from '../../components/Loading.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { BadgeInfo } from '../../ipc/bindings/BadgeInfo';
  import type { ChatIdentity } from '../../ipc/bindings/ChatIdentity';
  import type { SmileyChoice } from '../../ipc/bindings/SmileyChoice';
  import { chatIpc } from '../../ipc/chat';
  import { accounts } from '../../lib/accounts.svelte';
  import { density } from '../../lib/density.svelte';
  import { errorText } from '../../lib/errors';
  import { scrollBehavior } from '../../lib/motion';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { DEFAULT_ACCENT } from '../settings/themes';
  import { gridMove, hexColor, NAMED, namedColor, STYLE_NAME } from './identity';
  import { emoteImg } from './urls';

  interface Props {
    account: string;
    login: string;
  }

  type Part = 'badge' | 'color' | 'smiley';

  let { account, login }: Props = $props();
  const uid = $props.id();

  let me = $state<ChatIdentity | null>(null);
  let error = $state('');
  let busy = $state(false);
  let custom = $state('');
  /** V107-Mx Switched on, nothing worn here yet: the list shows, Twitch is told on a pick. */
  let ownWanted = $state(false);
  /** V108-ID A refused change, shown under the part it belongs to. */
  let failed = $state<{ part: Part; text: string } | null>(null);
  let body = $state<HTMLElement>();
  let ownGrid = $state<HTMLElement>();

  const theme = $derived(settings.v.appearance.theme);
  const display = $derived(accounts.list.find((a) => a.userId === account)?.displayName ?? '');
  const shown = $derived(me?.channel.selected ?? me?.global.selected ?? null);
  const styles = $derived(me?.smilies.choices ?? []);
  const typed = $derived(hexColor(custom));
  const same = (a: BadgeInfo | null, b: BadgeInfo | null) => a?.set === b?.set && a?.version === b?.version;
  // A channel pick equal to the global one counts as none: the read may repeat the global badge.
  const ownPick = $derived(me?.channel.selected && !same(me.channel.selected, me.global.selected) ? me.channel.selected : null);
  const own = $derived(!!ownPick || ownWanted);

  // Only the newest read lands: switching account or channel mid-read drops the old one.
  let reads = 0;
  async function load() {
    const n = ++reads;
    try {
      const got = await chatIpc.identity(account, login);
      if (n !== reads) return;
      me = got;
      custom = got.color ?? '';
      error = '';
    } catch (e) {
      if (n === reads) error = errorText(e);
    }
  }

  // Read again for another account or channel, so picks never go to the wrong one.
  $effect(() => {
    void account;
    void login;
    me = null;
    error = '';
    failed = null;
    ownWanted = false;
    if (body) body.scrollTop = 0;
    void untrack(load);
  });

  /** `reload` false: the action already answered with what Twitch has now. */
  async function run(part: Part, action: () => Promise<unknown>, done: string, reload = true) {
    if (busy) return;
    busy = true;
    failed = null;
    try {
      await action();
      if (reload) await load();
      toast.info(done);
    } catch (e) {
      failed = { part, text: errorText(e) };
    } finally {
      busy = false;
    }
  }

  const color = (hex: string) => run('color', () => chatIpc.setColor(account, hex), 'Name color changed.');
  const wear = (channel: boolean, b: BadgeInfo | null) =>
    run(
      'badge',
      () => chatIpc.pickBadge(account, channel ? (me?.channelId ?? null) : null, b?.set ?? null, b?.version ?? null),
      b ? `Wearing ${b.title}.` : channel ? 'Same badge as everywhere.' : 'Badge off.',
    );

  /** V108-ID Brings a part that just opened into view, unless that would push its top out. */
  function reveal(el: HTMLElement | undefined) {
    if (!el || !body) return;
    const box = body.getBoundingClientRect();
    const r = el.getBoundingClientRect();
    const by = Math.min(r.bottom + 12 - box.bottom, r.top - box.top - 12);
    if (by > 0) body.scrollBy({ top: by, behavior: scrollBehavior() });
  }

  /** Off takes the channel badge off, back to the one worn everywhere. */
  async function switchOwn(on: boolean) {
    if (busy) return;
    ownWanted = on;
    if (!on && ownPick) void wear(true, null);
    if (on) {
      await tick();
      reveal(ownGrid);
    }
  }

  const smile = (c: SmileyChoice) => {
    const [who, at] = [account, login];
    return run(
      'smiley',
      async () => {
        const now = await chatIpc.setSmilies(who, c.name);
        if (me && who === account && at === login) me.smilies = now;
      },
      `${STYLE_NAME[c.style]} smileys on.`,
      false,
    );
  };

  /** V108-ID One tab stop per grid (the worn pick, else the first); arrows move, Enter picks. */
  function rove(e: KeyboardEvent) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    const grid = e.currentTarget as HTMLElement;
    const items = [...grid.children] as HTMLElement[];
    const at = items.indexOf(e.target as HTMLElement);
    const cells = items.map((el) => {
      const r = el.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    });
    const to = gridMove(e.key, at, cells);
    if (to === null) return;
    e.preventDefault();
    items[to].focus();
  }

  const worn = $derived(namedColor(me?.color ?? null));
  const colorNote = $derived(
    `${worn ? `${worn}. ` : me?.color ? 'Your color isn’t one of these. ' : ''}Prime or Turbo can pick any color.`,
  );
  const swatchStop =$derived(Math.max(0, NAMED.findIndex(([, hex]) => hex === me?.color?.toUpperCase())));
  const img = (id: string) => emoteImg('twitch', id, theme, { scale: density.scale }).src;
</script>

{#snippet picks(channel: boolean, selected: BadgeInfo | null, available: BadgeInfo[], label: string)}
  {@const stop = available.findIndex((b) => same(selected, b))}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="grid" role="group" aria-label={label} onkeydown={rove}>
    {#if !channel}
      <button
        class="cell badge none"
        class:on={!selected}
        aria-pressed={!selected}
        aria-disabled={busy}
        tabindex={!selected ? 0 : -1}
        onclick={() => wear(false, null)}
      >
        None
      </button>
    {/if}
    {#each available as b, i (`${b.set}/${b.version}`)}
      <button
        class="cell badge"
        class:on={same(selected, b)}
        aria-pressed={same(selected, b)}
        aria-disabled={busy}
        tabindex={i === stop || (channel && stop < 0 && i === 0) ? 0 : -1}
        use:tooltip={b.title}
        aria-label={b.title}
        onclick={() => wear(channel, b)}
      >
        <img class="pale-edge" src={b.image2x} alt="" />
      </button>
    {/each}
  </div>
{/snippet}

{#snippet refused(part: Part)}
  {#if failed?.part === part}<p class="note bad" role="alert">{failed.text}</p>{/if}
{/snippet}

<div class="panel" aria-busy={busy}>
  <header class="head">
    {#if shown}<img class="pale-edge" src={shown.image2x} alt="" />{/if}
    <b class="name" style:color={me?.color ?? 'var(--text)'}>{display}</b>
    <span class="where" title={login}>in {login}</span>
  </header>

  <div class="body" bind:this={body}>
    {#if error}
      <p class="note">{error}</p>
    {:else if !me}
      <Loading minHeight="100%" />
    {:else}
      <section aria-labelledby="{uid}-badge">
        <h3 class="label" id="{uid}-badge">Badge</h3>
        {#if me.channelId}
          <p class="sub">Everywhere</p>
          {@render picks(false, me.global.selected, me.global.available, 'Badge everywhere')}
          <!-- V107-H11a Always there with a GQL sign-in, as on twitch.tv; the grid lists every
               badge you can wear here, the global ones too. -->
          <div class="own">
            <span class="own-text">
              Custom badge for this channel
              <small>Only in <b title={login}>{login}</b>’s chat.</small>
            </span>
            <Toggle checked={own} label="Custom badge for {login}" onchange={switchOwn} />
          </div>
          {#if own}
            <div class="own-picks" bind:this={ownGrid}>
              {#if me.channel.available.length}
                {@render picks(true, ownPick, me.channel.available, `Badge in ${login}`)}
              {:else}
                <p class="note">No badges to wear here.</p>
              {/if}
            </div>
          {/if}
          {@render refused('badge')}
        {:else}
          <p class="note">Picking a badge needs a GQL sign-in.</p>
        {/if}
      </section>

      <section aria-labelledby="{uid}-color">
        <h3 class="label" id="{uid}-color">Name color</h3>
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div class="grid" role="group" aria-label="Name color" onkeydown={rove}>
          {#each NAMED as [name, hex], i (hex)}
            <button
              class="cell swatch"
              class:on={me.color?.toUpperCase() === hex}
              aria-pressed={me.color?.toUpperCase() === hex}
              style:--c={hex}
              use:tooltip={name}
              aria-label={name}
              aria-disabled={busy}
              tabindex={i === swatchStop ? 0 : -1}
              onclick={() => color(hex)}
            ></button>
          {/each}
        </div>
        {#if me.anyColor}
          <div class="custom">
            <input type="color" value={typed ?? me.color ?? DEFAULT_ACCENT} oninput={(e) => (custom = e.currentTarget.value)} aria-label="Pick a color" disabled={busy} />
            <input class="field" bind:value={custom} maxlength="7" spellcheck="false" placeholder="#RRGGBB" aria-label="Any color" disabled={busy} />
            <Button small disabled={busy || !typed || typed === me.color?.toUpperCase()} onclick={() => typed && color(typed)}>Use</Button>
          </div>
        {:else}
          <p class="note">{colorNote}</p>
        {/if}
        {@render refused('color')}
      </section>

      {#if styles.length}
        <section aria-labelledby="{uid}-smiley">
          <h3 class="label" id="{uid}-smiley">Smileys</h3>
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div class="styles" role="radiogroup" aria-labelledby="{uid}-smiley" tabindex="-1" onkeydown={rove}>
            {#each styles as c (c.style)}
              {@const on = me.smilies.current === c.style}
              <button
                class="style"
                class:on
                role="radio"
                aria-checked={on}
                aria-disabled={busy}
                tabindex={on || (!styles.some((s) => s.style === me?.smilies.current) && c === styles[0]) ? 0 : -1}
                onclick={() => !on && smile(c)}
              >
                <span class="faces">
                  {#each c.preview as id (id)}<img src={img(id)} alt="" />{/each}
                </span>
                {STYLE_NAME[c.style]}
              </button>
            {/each}
          </div>
          {@render refused('smiley')}
        </section>
      {/if}
    {/if}
  </div>
</div>

<style>
  /* V108-ID 368 wide: 12 + eight 36 px columns with 8 px gaps + 12 (the scrollbar's gutter is
     the right margin). As tall as the design wants, never taller than the window allows. */
  .panel {
    display: flex;
    flex-direction: column;
    width: 368px;
    max-width: 100%;
    height: min(472px, calc(100vh - 112px));
    min-height: 0;
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 40px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
  }
  .head img {
    flex: none;
    width: 18px;
    height: 18px;
  }
  .name {
    flex: 0 1 auto;
    min-width: 0;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .where {
    flex: 1 1 0;
    min-width: 48px;
    color: var(--muted);
    font-size: var(--text-sub);
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* One scroll area; the gutter is always there, so the grid never shifts when a scrollbar comes. */
  .body {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 16px 1px 16px 12px;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h3 {
    margin: 0;
  }
  .sub {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .own {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  .own-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .own small {
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-hint);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* A very long channel name is cut, not wrapped. */
  .own small b {
    display: inline-block;
    max-width: 20ch;
    overflow: hidden;
    font-weight: 400;
    text-overflow: ellipsis;
    white-space: nowrap;
    vertical-align: bottom;
  }
  .own-picks {
    display: flex;
    flex-direction: column;
  }
  /* Badges and colors on the same columns. */
  .grid {
    display: grid;
    grid-template-columns: repeat(8, 36px);
    grid-auto-rows: 36px;
    gap: 8px;
  }
  .cell {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    padding: 0;
    transition:
      background-color var(--fast),
      border-color var(--fast),
      box-shadow var(--fast);
  }
  .cell:active,
  .style:active {
    opacity: 0.6;
    transition: none;
  }
  .badge {
    background: var(--surface-hover);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .badge img {
    width: 24px;
    height: 24px;
  }
  .badge.none {
    grid-column: span 2;
    width: 100%;
    color: var(--muted);
    font-size: var(--text-hint);
  }
  @media (hover: hover) {
    .badge:not(.on):hover,
    .style:not(.on):hover {
      border-color: var(--dim);
    }
  }
  /* Picked: the accent wash and a 2 px accent edge, the same on badges and smileys. */
  .badge.on,
  .style.on {
    background: var(--accent-dim);
    border-color: var(--accent-border);
    box-shadow: inset 0 0 0 1px var(--accent-border);
    color: var(--text);
  }
  /* A 28 px circle in a 36 px cell. Picked: a 2 px ring in the text colour hugging it (never
     the accent, which vanishes next to the blues); focus is the usual ring outside the cell. */
  .swatch {
    position: relative;
    background: none;
    border: 0;
    border-radius: 50%;
  }
  .swatch::before {
    content: '';
    position: absolute;
    inset: 4px;
    background: var(--c);
    border: 2px solid color-mix(in srgb, var(--text) 18%, transparent);
    border-radius: 50%;
  }
  .swatch.on {
    box-shadow: inset 0 0 0 2px var(--text);
  }
  .swatch.on::before {
    border-color: var(--elevated);
  }
  @media (hover: hover) {
    .swatch:not(.on):hover {
      box-shadow: inset 0 0 0 2px var(--border-strong);
    }
  }
  .badge[aria-disabled='true'],
  .swatch[aria-disabled='true'],
  .style[aria-disabled='true'] {
    opacity: 0.4;
  }
  .custom {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
  .custom .field {
    width: 96px;
    font-family: var(--font-mono);
  }
  .custom input[type='color'] {
    width: 36px;
    height: 30px;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: none;
  }
  /* Three styles over the width of the grids. */
  .styles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .style {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 4px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: var(--text-hint);
    transition:
      background-color var(--fast),
      border-color var(--fast);
  }
  .faces {
    display: flex;
    gap: 2px;
  }
  .faces img {
    width: 20px;
    height: 20px;
  }
  .note {
    margin: 0;
    color: var(--dim);
    font-size: var(--text-sub);
  }
  .note.bad {
    color: var(--red);
  }
  .own-picks .note {
    padding-top: 8px;
  }
  .own-picks .grid {
    margin-top: 8px;
  }
</style>
