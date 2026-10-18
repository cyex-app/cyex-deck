<!-- 1.41 Emote card: opens beside a clicked emote, never over it, and fully inside the
     window (V101-04). Shows where the emote comes from, who made and added it, what's
     stacked on it, and links to its page. Closes on Escape, a click elsewhere, scrolling
     elsewhere or leaving the window; a typed name keeps it open (c-2). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
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
  import { cardChips, cardFacts } from './cardFacts';
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

  const theme = $derived(settings.v.appearance.theme);
  const others = $derived([target.emote, ...target.stack].filter((l) => l.id !== shown.id));

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
    const release = box ? focusScope(box, { trap: true }) : undefined;
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

  const added = $derived(date(info?.addedAt ?? null));
  const facts = $derived(info ? cardFacts(info, shown.provider, added) : []);
  // Gigantified is the clicked emote, not one picked from its stack.
  const chips = $derived(cardChips(info, !!target.giant && shown.id === target.emote.id));
</script>

<div
  class="card"
  bind:this={box}
  use:floating={{ anchor: target.rect, side: 'right', gap: 8 }}
  style:width="min({WIDTH}px, calc(100vw - 16px))"
  role="dialog"
  aria-label={shown.name}
  tabindex="-1"
>
  <div class="head">
    <div class="tile">
      <img src={emoteLarge(shown.provider, shown.id, theme, { animated: shown.animated })} alt={shown.name} />
    </div>
    <div class="title">
      <b bind:this={heading} tabindex="-1">{shown.name}</b>
      {#if info?.original}<span class="sub">Originally {info.original}</span>{/if}
      <div class="chips">
        <span class="chip">{PROVIDER_NAMES[shown.provider]}</span>
        {#each chips as c (c.text)}<span class="chip" class:warn={c.warn}>{c.text}</span>{/each}
      </div>
    </div>
  </div>

  {#if info}
    <dl>
      {#each facts as f (f.label)}
        <dt>{f.label}</dt>
        {#if f.link === 'channel'}
          {@const login = f.login}
          <dd><button class="fact" use:tooltip={'Open channel'} onclick={() => channel(login)}>{f.value}</button></dd>
        {:else if f.link === 'web'}
          {@const url = f.url}
          <dd><button class="fact" use:tooltip={f.tip} onclick={() => open(url)}>{f.value}</button></dd>
        {:else if f.tip}
          <dd use:tooltip={f.tip}>{f.value}</dd>
        {:else}
          <dd>{f.value}</dd>
        {/if}
      {/each}
    </dl>
  {:else if failed}
    <p class="note">Couldn’t load the details.</p>
  {:else}
    <Loading />
  {/if}

  {#if others.length}
    <div class="stack">
      <span class="label">{shown.id === target.emote.id ? 'Overlaid with' : 'Also here'}</span>
      {#each others as l (l.id)}
        <button onclick={() => (shown = l)} data-tip={l.name}>
          <img src={emoteLarge(l.provider, l.id, theme, { animated: l.animated })} alt={l.name} />
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

  <div class="foot">
    {#if info?.page}
      <Button variant="link" small icon="popout" onclick={() => open(info!.page!)}>Emote page</Button>
    {/if}
    <span class="grow"></span>
    {#if info?.page}
      {@const page = info.page}
      <Button variant="link" small icon="link" onclick={() => void copy(page, 'Link')}>Copy link</Button>
    {/if}
    <Button variant="link" small icon="copy" onclick={() => void copy(shown.name, 'Emote name')}>Copy</Button>
    <Button variant="link" small icon="eyeOff" onclick={hide}>Hide</Button>
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
    animation: in var(--base) ease-out;
    font-size: 0.85rem;
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }
  .head {
    display: flex;
    gap: 12px;
  }
  .tile {
    display: grid;
    place-items: center;
    flex: none;
    min-width: 88px;
    max-width: 148px;
    height: 88px;
    padding: 6px;
    background: var(--elevated-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .tile img {
    max-width: 136px;
    max-height: 76px;
    object-fit: contain;
  }
  .title {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .title b {
    font-size: 0.92rem;
    overflow-wrap: anywhere;
  }
  .title b:focus:not(:focus-visible) {
    outline: none;
  }
  .sub {
    color: var(--dim);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    padding: 1px 6px;
    background: var(--elevated-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.75rem;
  }
  .warn {
    color: var(--yellow);
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 3px 12px;
    margin: 12px 0 0;
  }
  dt {
    color: var(--dim);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  /* A fact that goes somewhere: accent text, pressed at once (1.41). */
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
    transition: opacity var(--fast) ease-out;
  }
  .fact:hover {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .fact:active {
    opacity: 0.6;
    transition: none;
  }
  .note {
    margin: 12px 0 0;
    color: var(--dim);
  }
  .stack {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 12px;
  }
  .stack .label {
    width: 100%;
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
  }
  .stack img {
    max-height: 30px;
  }
  .foot {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 10px;
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .grow {
    flex: 1;
  }
</style>
