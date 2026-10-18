<!-- 1.41 Emote card: opens beside a clicked emote, never over it, and fully inside the
     window (V101-04). Where it's from, who made and added it, what's stacked on it, what
     it is, and its links. Closes on Escape, a click or scroll elsewhere or leaving the
     window; a typed name keeps it open (c-2). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { hideTip, tip, tooltip } from '../../components/tooltip.svelte';
  import type { EmoteCard } from '../../ipc/bindings/EmoteCard';
  import type { Layer } from '../../ipc/bindings/Layer';
  import { emotesIpc } from '../../ipc/emotes';
  import { notifyEvents } from '../../ipc/notify';
  import { systemIpc } from '../../ipc/system';
  import { layer } from '../../lib/escape';
  import { focusScope } from '../../lib/focusScope';
  import { formatDate } from '../../lib/format';
  import { floating } from '../../lib/place';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { emoteLarge } from '../chat/urls';
  import { cardChips, cardFacts, foldChips, opensTip } from './cardFacts';
  import { type CardTarget, PROVIDER_NAMES } from './emotes';
  import { hidden } from './hidden.svelte';
  import { unlistedPrompt } from './unlisted';
  import UnlistedPrompt from './UnlistedPrompt.svelte';
  import EditSection from './edit/EditSection.svelte';
  import { stayOpen } from './edit/stay';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';
  import { copy } from '../../lib/clipboard';

  interface Props {
    target: CardTarget;
    /** The chat it was clicked in. */
    login: string;
    /** V106-31/32 The signed-in account that may edit 7TV sets. */
    account?: string | null;
    onclose: () => void;
  }

  let { target, login, account = null, onclose }: Props = $props();

  const WIDTH = 300;

  // The card is rebuilt for each new target, so the first pick is enough.
  // svelte-ignore state_referenced_locally
  let shown = $state<Layer>(target.emote);
  let info = $state<EmoteCard | null>(null);
  let failed = $state(false);
  let box = $state<HTMLElement>();
  let heading = $state<HTMLElement>();
  /** R7 The chips behind "+N", shown in the row. */
  let more = $state(false);

  const theme = $derived(settings.v.appearance.theme);
  const layers = $derived([target.emote, ...target.stack]);

  let reloads = $state(0);
  const reload = () => void reloads++;

  /** c-1 After an edit from the card: a rename shows (and looks up) the new name. */
  function edited(e: Layer): void {
    if (e === shown) reload();
    else shown = e;
  }

  $effect(() => {
    const e = shown;
    void reloads;
    info = null;
    failed = false;
    let live = true;
    emotesIpc
      .card(login, e.provider, e.id, e.name)
      .then((c) => live && (info = c))
      .catch(() => live && (failed = true));
    return () => (live = false);
  });

  $effect(() => {
    const down = (ev: MouseEvent) => {
      const t = ev.target as Element;
      if (box && !box.contains(t) && !t.closest?.('[data-shared-menu]')) onclose();
    };
    // c-2 Scrolling the card itself, or a draft in its name form, keeps it.
    const away = (ev: Event) => {
      if (!stayOpen(box, ev)) onclose();
    };
    const off = layer('popover', onclose);
    // ct-2 Focus starts on the name, so a mouse open shows no frame.
    const release = box ? focusScope(box, { trap: true, initial: () => heading }) : undefined;
    addEventListener('mousedown', down, true);
    addEventListener('wheel', away, { passive: true });
    addEventListener('blur', away);
    return () => {
      removeEventListener('mousedown', down, true);
      off();
      release?.();
      removeEventListener('wheel', away);
      removeEventListener('blur', away);
    };
  });

  // ct-10 A chat tip (e.g. the clicked emote's) never stays over the card; its own tips do.
  $effect(() => {
    const a = tip.anchor;
    if (!tip.visible || !a || !box) return;
    const r = box.getBoundingClientRect();
    const x = (a.left + a.right) / 2;
    const y = (a.top + a.bottom) / 2;
    if (x < r.left || x > r.right || y < r.top || y > r.bottom) hideTip();
  });

  function date(iso: string | null): string | null {
    if (!iso) return null;
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? null : formatDate(d);
  }

  function hide() {
    hidden.hide({ source: shown.provider, id: shown.id, name: shown.name });
    toast.info(`${shown.name} shows as text now. Settings → Emotes brings it back.`);
    onclose();
  }

  function open(url: string) {
    systemIpc.openLink(url).catch((e) => toast.error(errorText(e)));
  }

  /** 1.41 The channel's own tab, in the main window (never twitch.tv). */
  function channel(login: string) {
    notifyEvents.open({ kind: 'channel', login }).catch((e) => toast.error(errorText(e)));
    onclose();
  }

  /** R10 The picture fades in once it's there, over the tile's placeholder. */
  function loaded(img: HTMLImageElement) {
    const done = () => img.classList.add('ready');
    if (img.complete && img.naturalWidth) done();
    else img.addEventListener('load', done, { once: true });
  }

  /** R12 A short fade out; no clicks land on it meanwhile. */
  function leave(node: HTMLElement) {
    node.style.pointerEvents = 'none';
    return { duration: 90, css: (t: number) => `opacity: ${t}` };
  }

  const added = $derived(date(info?.addedAt ?? null));
  const facts = $derived(info ? cardFacts(info, shown.provider, added) : []);
  const provider = $derived(PROVIDER_NAMES[shown.provider]);
  // Gigantified is the clicked emote, not one picked from its stack.
  const chips = $derived(
    foldChips(cardChips(info, !!target.giant && shown.id === target.emote.id), provider),
  );
  const page = $derived(info?.page ?? null);
  const pageTip = $derived(opensTip(page));
  const loading = $derived(!info && !failed);
</script>

<!-- R2 A browser link's cue: a small arrow out. In-app links have none. -->
{#snippet out()}<span class="out" aria-hidden="true"><Icon name="popout" size={10} /></span>{/snippet}

<div
  class="card"
  bind:this={box}
  use:floating={{ anchor: target.rect, side: 'right', gap: 8 }}
  out:leave|global
  style:width="min({WIDTH}px, calc(100vw - 16px))"
  role="dialog"
  aria-label={shown.name}
  tabindex="-1"
>
  <div class="head">
    {#key shown.id}
      {#if page && pageTip}
        <button class="tile" use:tooltip={pageTip} aria-label="Emote page" onclick={() => open(page)}>
          <img
            use:loaded
            src={emoteLarge(shown.provider, shown.id, theme, { animated: shown.animated })}
            alt={shown.name}
            decoding="async"
            draggable="false"
          />
          <span class="corner">{@render out()}</span>
        </button>
      {:else}
        <div class="tile">
          <img
            use:loaded
            src={emoteLarge(shown.provider, shown.id, theme, { animated: shown.animated })}
            alt={shown.name}
            decoding="async"
            draggable="false"
          />
        </div>
      {/if}
    {/key}
    <div class="title">
      <h2 class="name" bind:this={heading} tabindex="-1">
        {#if page && pageTip}
          <button class="fact" use:tooltip={{ label: pageTip, side: 'above' }} onclick={() => open(page)}>
            <span class="u">{shown.name}</span>{@render out()}
          </button>
        {:else}
          {shown.name}
        {/if}
      </h2>
      {#if info?.original}<span class="sub">Originally {info.original}</span>{/if}
    </div>
  </div>

  <div class="chips">
    <span class="chip source" use:tooltip={`From ${provider}`}>{provider}</span>
    {#each chips.shown as c (c.text)}<span class="chip" class:warn={c.warn} use:tooltip={c.tip}>{c.text}</span>{/each}
    {#if chips.rest.length}
      {#if more}
        {#each chips.rest as c (c.text)}<span class="chip" class:warn={c.warn} use:tooltip={c.tip}>{c.text}</span>{/each}
      {/if}
      <button
        class="chip more"
        aria-expanded={more}
        use:tooltip={more ? 'Show fewer' : chips.rest.map((c) => c.text).join(', ')}
        onclick={() => (more = !more)}>{more ? 'Less' : `+${chips.rest.length}`}</button
      >
    {/if}
  </div>

  {#if info}
    <dl>
      {#each facts as f (f.label)}
        <dt>{f.label}</dt>
        {#if f.link === 'channel'}
          {@const login = f.login}
          <dd>
            <button
              class="fact"
              class:who={!!f.avatar}
              use:tooltip={{ label: 'Open channel', side: 'right' }}
              onclick={() => channel(login)}
            >
              {#if f.avatar}
                <img
                  class="avatar"
                  src={f.avatar}
                  alt=""
                  onerror={(e) => ((e.currentTarget as HTMLImageElement).hidden = true)}
                />
              {/if}<span class="u">{f.value}</span>
            </button>
          </dd>
        {:else if f.link === 'web'}
          {@const url = f.url}
          <dd>
            <button class="fact" use:tooltip={{ label: f.tip, side: 'right' }} onclick={() => open(url)}>
              <span class="u">{f.value}</span>{@render out()}
            </button>
          </dd>
        {:else if f.tip}
          <dd use:tooltip={f.tip}>{f.value}</dd>
        {:else}
          <dd>{f.value}</dd>
        {/if}
      {/each}
    </dl>
  {:else if failed}
    <div class="pending note" role="status">
      Couldn’t load the details.
      <Button variant="link" small onclick={reload}>Try again</Button>
    </div>
  {:else}
    <div class="pending"><Loading /></div>
  {/if}

  {#if layers.length > 1}
    <div class="stack">
      <span class="label">In this stack</span>
      {#each layers as l (l.id)}
        <button
          class:on={l.id === shown.id}
          aria-pressed={l.id === shown.id}
          aria-label={l.name}
          use:tooltip={l.name}
          onclick={() => (shown = l)}
        >
          <img
            src={emoteLarge(l.provider, l.id, theme, { animated: l.animated })}
            alt=""
            decoding="async"
            draggable="false"
          />
        </button>
      {/each}
    </div>
  {/if}

  {#if info && unlistedPrompt(settings.v.chat, info.unlisted)}
    <UnlistedPrompt onchosen={() => box?.focus()} />
  {/if}

  {#if account && shown.provider === 'seventv'}
    <EditSection {account} {login} emote={shown} {target} onchange={edited} home={() => heading} />
  {/if}

  <!-- R4 One way out first, the copies as a quiet pair, Hide apart. -->
  <div class="foot">
    {#if page && pageTip}
      <span use:tooltip={pageTip}>
        <Button variant="link" small icon="popout" onclick={() => open(page)}>Emote page</Button>
      </span>
    {:else if loading}
      <!-- R11 Holds its place while the details load. -->
      <span class="reserve" aria-hidden="true"><Icon name="popout" size={14} />Emote page</span>
    {/if}
    <span class="copies">
      {#if page}
        <IconButton icon="link" label="Copy link" size={15} onclick={() => void copy(page, 'Link')} />
      {:else if loading}
        <span class="icon-slot"></span>
      {/if}
      <IconButton icon="copy" label="Copy name" size={15} onclick={() => void copy(shown.name, 'Emote name')} />
    </span>
    <span class="grow"></span>
    <button class="hide" onclick={hide}><Icon name="eyeOff" size={14} />Hide</button>
  </div>
</div>

<style>
  .card {
    position: fixed;
    z-index: var(--z-popover);
    padding: 12px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    font-size: 0.85rem;
    /* R12 Grows out of the emote it opened beside. */
    transform-origin: var(--ox, left) var(--oy, center);
    animation: in var(--base) ease-out;
  }
  .card:focus:not(:focus-visible) {
    outline: none;
  }
  .card:global([data-side='right']) {
    --ox: left;
    --oy: center;
    --dx: -4px;
    --dy: 0px;
  }
  .card:global([data-side='left']) {
    --ox: right;
    --oy: center;
    --dx: 4px;
    --dy: 0px;
  }
  .card:global([data-side='below']) {
    --ox: center;
    --oy: top;
    --dx: 0px;
    --dy: -4px;
  }
  .card:global([data-side='above']) {
    --ox: center;
    --oy: bottom;
    --dx: 0px;
    --dy: 4px;
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translate(var(--dx, -4px), var(--dy, 0px)) scale(0.98);
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  /* Reduce motion: no travel, only the fade (the global rule would drop it). */
  @media (prefers-reduced-motion: reduce) {
    .card {
      animation-name: fade;
      animation-duration: var(--base) !important;
    }
  }
  .head {
    display: flex;
    gap: 12px;
  }
  /* R10 A fixed square; the picture fits inside it. */
  .tile {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 88px;
    height: 88px;
    padding: 6px;
    background: var(--elevated-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  button.tile {
    cursor: pointer;
    transition: border-color var(--fast) ease-out;
  }
  button.tile:hover {
    border-color: var(--border-strong);
  }
  button.tile:active {
    transform: scale(0.97);
    transition: none;
  }
  .tile img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    opacity: 0;
    transition: opacity var(--fast) ease-out;
  }
  .tile img:global(.ready) {
    opacity: 1;
  }
  .corner {
    position: absolute;
    top: 3px;
    right: 3px;
    display: flex;
  }
  .title {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  /* R1 */
  .name {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
    line-height: 1.2;
    letter-spacing: -0.01em;
    overflow-wrap: anywhere;
  }
  .name:focus:not(:focus-visible) {
    outline: none;
  }
  .name .fact {
    color: inherit;
  }
  .sub {
    color: var(--dim);
    letter-spacing: 0.01em;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }
  .chip {
    padding: 2px 7px;
    background: var(--elevated-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.75rem;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }
  /* R8 Where it's from reads as a source, not a flag. */
  .source {
    padding-left: 0;
    background: none;
    border-color: transparent;
    color: var(--dim);
  }
  .warn {
    color: var(--yellow);
    border-color: color-mix(in srgb, var(--yellow) 45%, var(--border));
  }
  .more {
    font: inherit;
    font-size: 0.75rem;
    cursor: pointer;
    transition: border-color var(--fast) ease-out;
  }
  .more:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }
  .more:active {
    opacity: 0.6;
    transition: none;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    margin: 12px 0 0;
    line-height: 1.4;
  }
  dt {
    color: var(--dim);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  /* R2 A fact that goes somewhere: a faint underline at rest, full on hover; the underline
     and the arrow say "link", so a dark accent still reads. */
  .fact {
    padding: 0;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--accent-text);
    font: inherit;
    text-align: start;
    overflow-wrap: anywhere;
    cursor: pointer;
  }
  .u {
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, currentColor 35%, transparent);
    text-underline-offset: 2px;
  }
  .fact:hover .u {
    text-decoration-color: currentColor;
  }
  .fact:active {
    opacity: 0.6;
  }
  .fact:focus-visible {
    outline-offset: 2px;
  }
  .out {
    display: inline-block;
    margin-left: 4px;
    color: var(--dim);
    vertical-align: 0.05em;
  }
  .who {
    display: inline-flex;
    align-items: flex-start;
    gap: 8px;
  }
  /* R14 Placeholder until it loads; centered on the first line of a wrapped name. */
  .avatar {
    flex: none;
    width: 16px;
    height: 16px;
    margin-top: calc((1.4em - 16px) / 2);
    background: var(--elevated-2);
    border-radius: 50%;
  }
  /* R11 Holds about three rows while loading or after a failure. */
  .pending {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 64px;
    margin-top: 12px;
  }
  .note {
    flex-direction: column;
    gap: 4px;
    color: var(--dim);
  }
  .stack {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 12px;
  }
  /* R15 Labeled like the facts. */
  .stack .label {
    width: 100%;
    color: var(--dim);
    font-size: 0.75rem;
  }
  .stack button {
    display: grid;
    place-items: center;
    height: 36px;
    min-width: 36px;
    padding: 2px;
    background: var(--elevated-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    transition: border-color var(--fast) ease-out;
  }
  .stack button:hover {
    border-color: var(--border-strong);
  }
  .stack button:active {
    transform: scale(0.97);
    transition: none;
  }
  .stack button.on {
    border-color: var(--accent-border);
  }
  .stack img {
    max-height: 30px;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }
  .reserve {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 0.8rem;
    font-weight: 500;
    visibility: hidden;
  }
  .copies {
    display: flex;
  }
  .icon-slot {
    width: 30px;
    height: 30px;
  }
  .grow {
    flex: 1;
  }
  /* R4 Apart and quiet; red only on the way to it. */
  .hide {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 27px;
    padding: 0 6px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition:
      color var(--fast),
      background var(--fast);
  }
  .hide:hover {
    background: var(--surface-hover);
    color: var(--red);
  }
  .hide:active {
    opacity: 0.6;
    transition: none;
  }
</style>
