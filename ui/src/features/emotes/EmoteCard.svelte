<!-- 1.41 Emote card: opens beside a clicked emote, never over it, and fully inside the
     window (V101-04). Shows where the emote comes from, who made and added it, what's
     stacked on it, and links to its page. Closes on Escape, a click elsewhere, or scrolling. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import type { EmoteCard } from '../../ipc/bindings/EmoteCard';
  import type { Layer } from '../../ipc/bindings/Layer';
  import { emotesIpc } from '../../ipc/emotes';
  import { systemIpc } from '../../ipc/system';
  import { formatDate } from '../../lib/format';
  import { floating } from '../../lib/place';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { emoteLarge } from '../chat/urls';
  import { type CardTarget, PROVIDER_NAMES } from './emotes';
  import { hidden } from './hidden.svelte';
  import { flipUnlisted, unlistedBlurred } from './unlisted';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';
  import { copy } from '../../lib/clipboard';

  interface Props {
    target: CardTarget;
    /** The chat it was clicked in. */
    login: string;
    onclose: () => void;
  }

  let { target, login, onclose }: Props = $props();

  const WIDTH = 300;

  // The card is rebuilt for each new target, so the first pick is enough.
  // svelte-ignore state_referenced_locally
  let shown = $state<Layer>(target.emote);
  let info = $state<EmoteCard | null>(null);
  let failed = $state(false);
  let box = $state<HTMLElement>();

  const theme = $derived(settings.v.appearance.theme);
  const others = $derived([target.emote, ...target.stack].filter((l) => l.id !== shown.id));

  $effect(() => {
    const e = shown;
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
      if (box && !box.contains(ev.target as Node)) onclose();
    };
    const key = (ev: KeyboardEvent) => ev.key === 'Escape' && onclose();
    addEventListener('mousedown', down, true);
    addEventListener('keydown', key);
    addEventListener('wheel', onclose, { passive: true });
    addEventListener('blur', onclose);
    return () => {
      removeEventListener('mousedown', down, true);
      removeEventListener('keydown', key);
      removeEventListener('wheel', onclose);
      removeEventListener('blur', onclose);
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

  const added = $derived(date(info?.addedAt ?? null));
</script>

<div
  class="card"
  bind:this={box}
  use:floating={{ anchor: target.rect, side: 'right', gap: 8 }}
  style:width="{WIDTH}px"
  role="dialog"
  aria-label={shown.name}
>
  <div class="head">
    <div class="tile">
      <img src={emoteLarge(shown.provider, shown.id, theme, { animated: shown.animated })} alt={shown.name} />
    </div>
    <div class="title">
      <b>{shown.name}</b>
      {#if info?.original}<span class="sub">Originally {info.original}</span>{/if}
      <div class="chips">
        <span class="chip">{PROVIDER_NAMES[shown.provider]}</span>
        {#if info?.kind}<span class="chip">{info.kind}</span>{/if}
        {#if info?.zeroWidth}<span class="chip">Zero-width</span>{/if}
        {#if info?.unlisted}<span class="chip warn">Unlisted</span>{/if}
      </div>
    </div>
  </div>

  {#if info}
    <dl>
      {#if info.channel}<dt>Channel</dt><dd>{info.channel}</dd>{/if}
      {#if info.owner}<dt>{shown.provider === 'twitch' ? 'Artist' : 'By'}</dt><dd>{info.owner}</dd>{/if}
      {#if info.addedBy}<dt>Added by</dt><dd>{info.addedBy}</dd>{/if}
      {#if added}<dt>Added</dt><dd title={info.addedAt ?? undefined}>{added}</dd>{/if}
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

  <div class="foot">
    {#if info?.page}
      <button class="link" onclick={() => open(info!.page!)}><Icon name="popout" size={13} />Emote page</button>
    {/if}
    <span class="grow"></span>
    {#if info?.unlisted && settings.v.chat.unlistedEmotes !== 'hide'}
      <!-- V102-01 The same switch as the emote's menu. -->
      <button class="link" onclick={flipUnlisted} data-tip="All unlisted emotes, every chat">
        <Icon name={unlistedBlurred() ? 'eye' : 'eyeOff'} size={13} />{unlistedBlurred() ? 'Unblur' : 'Blur'}
      </button>
    {/if}
    <button class="link" onclick={() => void copy(shown.name, 'Emote name')}><Icon name="copy" size={13} />Copy</button>
    <button class="link" onclick={hide}><Icon name="eyeOff" size={13} />Hide</button>
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
    box-shadow: 0 8px 24px var(--shadow);
    animation: in 0.18s ease-out;
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
    gap: 10px;
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .grow {
    flex: 1;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text);
    font-size: 0.8rem;
  }
  .link:hover {
    text-decoration: underline;
  }
</style>
