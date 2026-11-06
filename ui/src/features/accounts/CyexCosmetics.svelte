<!-- V109-B02/B03 cyex cosmetics of one account. No sign-in of its own: the app gets a cyex session
     with the account's Twitch sign-in. B03 layout: the chat preview on top with the store link,
     then the badges, a code behind a link. Nothing to set: one line and "Open store". Changes save
     at once and roll back with the error.
     V109-B04 The badges are two slots that open the shared menu: slot 1 one one-time badge (from
     cache), slot 2 Supporter+ only, opened after a fresh look at the plan. -->
<script lang="ts">
  import { tick } from 'svelte';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import { closeMenu, openMenu } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { AccountView } from '../../ipc/bindings/AccountView';
  import { cyexIpc, type CyexMe, type CyexPics, type CyexView } from '../../ipc/cyex';
  import { systemIpc } from '../../ipc/system';
  import { density } from '../../lib/density.svelte';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import {
    type Main,
    nothingToSet,
    plusGate,
    plusState,
    shownBadges,
    SLOT1_KINDS,
    slot1Current,
    slot1Menu,
    slot2Menu,
    STORE,
  } from './cyexBadges';

  let { account }: { account: AccountView } = $props();

  let view = $state.raw<CyexView | null>(null);
  let loading = $state(false);
  let saving = $state(false);
  let error = $state('');
  let coding = $state(false);
  let code = $state('');
  let codeField = $state<HTMLInputElement>();
  let redeeming = $state(false);
  let redeemed = $state('');
  let redeemError = $state('');
  /** V109-B04 Slot 2 is checking the plan; what it found when that stopped it opening. */
  let checking = $state(false);
  let plusError = $state('');
  let plusTried = $state(false);

  const me = $derived(view?.state === 'ready' ? view.me : null);
  const NO_PICS: CyexPics = { '1x': '', '2x': '', '4x': '' };
  // One gate per account: a look from the last 30 s opens slot 2 without asking again.
  const gate = $derived.by(() => {
    const id = account.userId;
    return plusGate(() => cyexIpc.view(id));
  });

  async function load() {
    loading = true;
    try {
      view = await cyexIpc.view(account.userId);
      gate.seen(view);
    } catch {
      view = { state: 'offline' };
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void account.userId;
    void load();
  });

  const src = (p: CyexPics) => (density.scale === 1 ? p['1x'] : density.scale === 2 ? p['2x'] || p['1x'] : p['4x'] || p['2x'] || p['1x']);

  function open(url: string) {
    systemIpc.openLink(url).catch((e) => toast.error(errorText(e)));
  }

  function withMe(next: CyexMe) {
    if (view?.state === 'ready') view = { ...view, me: next };
  }

  /** Saves at once; a refusal puts the old choice back with the reason. */
  async function save(main: string | null, custom: boolean) {
    if (!me || saving) return;
    const before = me;
    // The preview follows at once; the server's answer settles `shown`.
    const guess = [...(main ? [main] : []), ...(custom && me.plus.active && me.plus.live ? ['custom'] : [])];
    withMe({ ...me, equipped: { main, custom, shown: guess } });
    saving = true;
    error = '';
    try {
      const equipped = await cyexIpc.equip(account.userId, main, custom);
      withMe({ ...before, equipped });
    } catch (e) {
      withMe(before);
      error = errorText(e);
    } finally {
      saving = false;
    }
  }

  /** The slot's own menu is open: its button closes it (V109-F61). */
  const isOpen = (el: HTMLElement) => el.getAttribute('aria-expanded') === 'true';

  function openSlot1(e: MouseEvent) {
    const el = e.currentTarget as HTMLElement;
    if (!me) return;
    if (isOpen(el)) return closeMenu();
    const m = me;
    openMenu(el, slot1Menu(m, { equip: (main: Main | null) => void save(main, m.equipped.custom), store: () => open(STORE) }), 'start');
  }

  async function openSlot2(e: MouseEvent) {
    const el = e.currentTarget as HTMLElement;
    if (!me || checking) return;
    if (isOpen(el)) return closeMenu();
    plusError = '';
    // Busy only when it really asks the server: a cached look opens at once.
    checking = !gate.fresh();
    const got = await gate.check();
    checking = false;
    if (got.kind === 'error') {
      plusError = got.message;
      return;
    }
    // The fresh look also refreshes slot 1 and the preview.
    if (got.fetched && !saving) view = got.fetched;
    if (got.kind === 'locked') {
      plusTried = true;
      return;
    }
    const m = view?.state === 'ready' ? view.me : null;
    if (!m || !el.isConnected) return;
    openMenu(el, slot2Menu(m, { show: (on) => void save(m.equipped.main, on), open }), 'start');
  }

  async function openCode() {
    coding = true;
    redeemed = '';
    await tick();
    codeField?.focus();
  }

  function closeCode() {
    coding = false;
    code = '';
    redeemError = '';
  }

  async function redeem(e: SubmitEvent) {
    e.preventDefault();
    const c = code.trim();
    if (!c || redeeming) return;
    redeeming = true;
    redeemed = '';
    redeemError = '';
    try {
      redeemed = await cyexIpc.redeem(account.userId, c);
      code = '';
      coding = false;
      await load();
    } catch (err) {
      redeemError = errorText(err);
    } finally {
      redeeming = false;
    }
  }
</script>

{#snippet codeArea()}
  <div class="code">
    {#if coding}
      <form class="code-form" onsubmit={redeem}>
        <input
          class="field"
          placeholder="Code"
          aria-label="cyex code"
          autocomplete="off"
          spellcheck="false"
          maxlength="64"
          bind:this={codeField}
          bind:value={code}
          onkeydown={(e) => e.key === 'Escape' && closeCode()}
        />
        <Button small type="submit" disabled={!code.trim() || redeeming}>{redeeming ? 'Redeeming…' : 'Redeem'}</Button>
        <Button small variant="ghost" onclick={closeCode}>Cancel</Button>
      </form>
      {#if redeemError}<p class="msg bad" role="alert">{redeemError}</p>{/if}
    {:else}
      <Button small variant="link" hit onclick={() => void openCode()}>Have a code?</Button>
      {#if redeemed}<span class="msg ok" role="status">{redeemed}</span>{/if}
    {/if}
  </div>
{/snippet}

<div class="cyex" aria-busy={loading}>
  {#if !view}
    <div class="box state"><span class="note">Loading…</span></div>
  {:else if view.state === 'offline'}
    <div class="box state">
      <span class="note">Can’t reach cyex right now.</span>
      <Button small disabled={loading} onclick={() => void load()}>{loading ? 'Trying…' : 'Retry'}</Button>
    </div>
  {:else if view.state === 'noToken'}
    <div class="box state"><span class="note">cyex needs this account’s Twitch sign-in. Sign in again above.</span></div>
  {:else if me && nothingToSet(me)}
    <div class="box">
      <div class="empty">
        <div class="lead">
          <strong>No cyex badges yet</strong>
          <span class="note">A founder or supporter badge, or your own with Supporter+.</span>
        </div>
        <Button small variant="primary" icon="popout" onclick={() => open(STORE)}>Open store</Button>
      </div>
      {@render codeArea()}
    </div>
  {:else if me && view.state === 'ready'}
    {@const plus = plusState(me)}
    {@const worn = slot1Current(me)}
    {@const wornPics = worn ? SLOT1_KINDS.find((k) => k.id === worn.id)?.pics(view) : null}
    {@const locked = !me.plus.active}
    <div class="box">
      <!-- What chat shows now, with the store beside it. -->
      <div class="preview">
        <span class="line" aria-label="In chat">
          {#each shownBadges(me, view.founder ?? NO_PICS, view.supporter ?? NO_PICS) as b (b.key)}
            <img class="badge" src={src(b.pics)} alt={b.title} use:tooltip={{ label: b.title }} />
          {/each}
          <strong class="who">{account.displayName}</strong><span class="said">: hi chat</span>
        </span>
        <Button small variant="link" icon="popout" hit onclick={() => open(STORE)}>Store</Button>
      </div>

      <!-- V109-B04 Two slots, each a button that opens the shared menu under it. -->
      <div class="slots">
        <button class="slot" aria-haspopup="menu" aria-label="Badge: {worn?.label ?? 'None'}" onclick={openSlot1}>
          <span class="slot-key">Badge</span>
          <span class="slot-value">
            {#if worn && wornPics}<img class="badge" src={src(wornPics)} alt="" />{/if}
            <span class="slot-text" class:none={!worn}>{worn?.label ?? 'None'}</span>
          </span>
          <Icon name="chevronDown" size={14} />
        </button>
        <button
          class="slot"
          class:locked
          aria-haspopup="menu"
          aria-busy={checking}
          aria-label="Supporter+: {locked ? 'locked' : me.equipped.custom ? 'on' : 'off'}"
          onclick={(e) => void openSlot2(e)}
          use:tooltip={locked ? { label: 'Supporter+ only', hint: 'A monthly plan in the store.' } : { label: '' }}
        >
          <span class="slot-key">Supporter+</span>
          <span class="slot-value">
            {#if locked}
              <Icon name="lock" size={12} /><span class="slot-text none">Locked</span>
            {:else}
              {#if me.equipped.custom && me.plus.live}<img class="badge" src={src(me.plus.live)} alt="" />{/if}
              <span class="slot-text" class:none={!me.equipped.custom}>{me.equipped.custom ? 'On' : 'Off'}</span>
            {/if}
          </span>
          {#if checking}<span class="spin" aria-hidden="true"></span>{:else}<Icon name="chevronDown" size={14} />{/if}
        </button>
      </div>

      <!-- The plan in one line: renews, review, ended (with Renew), or why slot 2 stayed shut. -->
      {#if plusError}
        <p class="status msg bad" role="alert">{plusError}</p>
      {:else if plus.kind !== 'never' || plusTried}
        <p class="status" role="status">
          {#if plus.kind === 'never'}
            <span class="note">Only with Supporter+</span>
            <Button small variant="link" hit onclick={() => open(STORE)}>Get it in store</Button>
          {:else}
            {#if plus.plan}<span class="note">{plus.plan}</span>{/if}
            {#if plus.note && plus.kind !== 'ended'}<span class="note">· {plus.note}</span>{/if}
            {#if plus.kind === 'ended' && plus.link}
              {@const link = plus.link}
              <Button small variant="link" hit onclick={() => open(link.url)}>{link.label}</Button>
            {/if}
          {/if}
        </p>
      {/if}

      {#if error}<p class="msg bad err" role="alert">{error}</p>{/if}
      {@render codeArea()}
    </div>
  {/if}
</div>

<style>
  .cyex {
    padding: 2px 0 10px;
  }
  /* One inset box under the "cyex badges" line: the card's surface steps down to the page. */
  .box {
    container-type: inline-size;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .box.state {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 6px 12px;
  }
  .note {
    color: var(--dim);
    font-size: var(--text-sub);
  }
  .empty {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
  }
  .lead {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .preview {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
  }
  .line {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 3px;
    min-width: 0;
    font-family: var(--chat-font);
    font-size: var(--chat-size);
  }
  .who {
    margin-left: 1px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .said {
    flex: none;
    /* Sits on the name like in chat, past the row's gap. */
    margin-left: -3px;
    color: var(--muted);
  }
  .badge {
    width: 18px;
    height: 18px;
    flex: none;
  }
  /* V109-B04 Two equal slots in a row, each a compact field-like button. */
  .slots {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }
  .slot {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 44px;
    padding: 0 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition:
      background var(--fast),
      border-color var(--fast);
  }
  .slot:hover {
    background: var(--surface-hover);
  }
  .slot:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .slot:global([aria-expanded='true']) {
    border-color: var(--accent-border);
  }
  .slot > :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .slot-key {
    flex: none;
    width: 74px;
    color: var(--dim);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .slot-value {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .slot-value > :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .slot-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .slot-text.none {
    color: var(--muted);
    font-weight: 400;
  }
  /* Checking the plan: a 14 px ring where the chevron was, so nothing moves. */
  .spin {
    flex: none;
    width: 14px;
    height: 14px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--muted);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }
  .slot[aria-busy='true'] {
    cursor: progress;
  }
  .slot[aria-busy='true'] .slot-value {
    opacity: 0.6;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
  @media (prefers-contrast: more) {
    .slot {
      border-color: var(--text);
    }
  }
  .status {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    margin: 0;
    min-height: 32px;
    padding: 4px 12px;
    border-bottom: 1px solid var(--border);
  }
  .code {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    min-height: 40px;
    padding: 6px 12px;
  }
  .empty + .code {
    border-top: 1px solid var(--border);
  }
  .code-form {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }
  .code-form input {
    flex: 1;
    min-width: 0;
    max-width: 240px;
  }
  .msg {
    margin: 0;
    font-size: var(--text-hint);
  }
  p.msg {
    flex-basis: 100%;
  }
  .err {
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
  }
  .ok {
    color: var(--text);
  }
  .bad {
    color: var(--red);
  }
  /* Narrow: the slots stack; the empty state's button goes under its text. */
  @container (max-width: 420px) {
    .slots {
      grid-template-columns: 1fr;
    }
    .empty {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
