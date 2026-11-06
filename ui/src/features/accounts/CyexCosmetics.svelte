<!-- V109-B02/B03 cyex cosmetics of one account. No sign-in of its own: the app gets a cyex session
     with the account's Twitch sign-in. B03 layout: the chat preview on top with the store link,
     then the badges, a code behind a link. Nothing to set: one line and "Open store". Changes save
     at once and roll back with the error.
     V109-B04 The badges are two slots: slot 1 one one-time badge (from cache), slot 2 Supporter+
     only, opened after a fresh look at the plan.
     V109-B05 Slot 1 opens a picker (the 7TV pop-up's grid, None first, grid or list), slot 2 a
     panel for the uploaded picture. Each slot has its own on/off switch. -->
<script lang="ts">
  import { tick } from 'svelte';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconToggle from '../../components/IconToggle.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { AccountView } from '../../ipc/bindings/AccountView';
  import { cyexIpc, type CyexMe, type CyexPics, type CyexView } from '../../ipc/cyex';
  import { systemIpc } from '../../ipc/system';
  import { density } from '../../lib/density.svelte';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import ItemGrid from '../cosmetics/ItemGrid.svelte';
  import CyexPopup from './CyexPopup.svelte';
  import {
    filterRows,
    nothingToSet,
    plusGate,
    plusPanel,
    plusState,
    readSlot1,
    saveSlot1,
    shownBadges,
    SLOT1_KINDS,
    slot1Choice,
    slot1Current,
    slot1Missing,
    slot1Pick,
    slot1Rows,
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
  /** V109-B05 The open pop-up and the slot button it hangs under. */
  let popup = $state<{ slot: 1 | 2; anchor: HTMLElement } | null>(null);
  let search = $state('');
  /** Grid or list, for this session. */
  let listView = $state(false);
  /** Slot 1's choice kept while its switch is off (per account, on this PC). */
  let remembered = $state<string | null>(null);

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
    remembered = readSlot1(account.userId);
    popup = null;
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

  function remember(id: string | null) {
    remembered = id;
    saveSlot1(account.userId, id);
  }

  /** V109-B05 A card in slot 1: on at once with that badge; None takes it off and forgets it. */
  function pick1(id: string | null) {
    if (!me) return;
    if (id === null) remember(null);
    const next = slot1Pick(me, id);
    if (!next) return;
    remember(next.main);
    void save(next.main, me.equipped.custom);
  }

  /** V109-B05 Slot 1's switch: off keeps the choice here, on puts it back. */
  function switch1(on: boolean) {
    if (!me) return;
    const choice = slot1Choice(me, remembered);
    if (!choice) return;
    remember(choice);
    void save(on ? choice : null, me.equipped.custom);
  }

  function close() {
    popup = null;
    search = '';
  }

  /** The slot's own pop-up is open: its button closes it. */
  const isOpen = (slot: 1 | 2) => popup?.slot === slot;

  function openSlot1(e: MouseEvent) {
    const el = e.currentTarget as HTMLElement;
    if (!me) return;
    if (isOpen(1)) return close();
    search = '';
    popup = { slot: 1, anchor: el };
  }

  async function openSlot2(e: MouseEvent) {
    const el = e.currentTarget as HTMLElement;
    if (!me || checking) return;
    if (isOpen(2)) return close();
    popup = null;
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
    if (!el.isConnected) return;
    popup = { slot: 2, anchor: el };
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
    {@const choice = slot1Choice(me, remembered)}
    {@const on1 = !!slot1Current(me)}
    {@const choiceLabel = choice ? SLOT1_KINDS.find((k) => k.id === choice)!.label(me) : 'None'}
    {@const choicePics = choice ? SLOT1_KINDS.find((k) => k.id === choice)!.pics(view) : null}
    {@const locked = !me.plus.active}
    {@const panel = plusPanel(me)}
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

      <!-- V109-B04/B05 Two slots: the button opens its pop-up, the switch shows or hides it in chat. -->
      <div class="slots">
        <div class="slot" class:off={!on1}>
          <button class="slot-open" aria-haspopup="dialog" aria-expanded={isOpen(1)} aria-label="Badge: {choiceLabel}" onclick={openSlot1}>
            <span class="slot-key">Badge</span>
            <span class="slot-value">
              {#if choice && choicePics}<img class="badge" src={src(choicePics)} alt="" />{/if}
              <span class="slot-text" class:none={!choice}>{choiceLabel}</span>
            </span>
            <Icon name="chevronDown" size={14} />
          </button>
          <Toggle label="Show badge in chat" checked={on1} disabled={!choice} onchange={switch1} />
        </div>
        <div class="slot" class:off={locked || !me.equipped.custom}>
          <button
            class="slot-open"
            aria-haspopup="dialog"
            aria-expanded={isOpen(2)}
            aria-busy={checking}
            aria-label="Supporter+: {locked ? 'locked' : panel.review}"
            onclick={(e) => void openSlot2(e)}
            use:tooltip={locked ? { label: 'Supporter+ only', hint: 'A monthly plan in the store.' } : { label: '' }}
          >
            <span class="slot-key">Supporter+</span>
            <span class="slot-value">
              {#if locked}
                <Icon name="lock" size={12} /><span class="slot-text none">Locked</span>
              {:else}
                {#if me.plus.live}<img class="badge" src={src(me.plus.live)} alt="" />{/if}
                <span class="slot-text" class:none={!me.plus.live}>{me.plus.live ? 'Yours' : panel.review.split('.')[0]}</span>
              {/if}
            </span>
            {#if checking}<span class="spin" aria-hidden="true"></span>{:else}<Icon name="chevronDown" size={14} />{/if}
          </button>
          <Toggle
            label="Show Supporter+ badge in chat"
            checked={!locked && me.equipped.custom}
            disabled={locked}
            onchange={(v) => void save(me.equipped.main, v)}
          />
        </div>
      </div>

      {#if popup?.slot === 1}
        {@const rows = filterRows(slot1Rows(me, view), search)}
        {@const cells = rows.length + 1}
        <CyexPopup title="Badge" anchor={popup.anchor} onclose={close}>
          <div class="tools">
            <SearchField small placeholder="Search badges" value={search} oninput={(t) => (search = t)} />
            <IconToggle icon="list" label="List view" pressed={listView} onchange={(v) => (listView = v)} />
          </div>
          <div class="grid" style:height="{Math.min(240, listView ? cells * 46 : Math.ceil(cells / 3) * 82)}px">
            <ItemGrid kind="badge" {rows} selected={choice} name={account.displayName} label="Badges" list={listView} onpick={(id) => pick1(id)} />
          </div>
          {#if slot1Missing(me)}
            <span class="more"><Button small variant="link" hit onclick={() => open(STORE)}>Get in store</Button></span>
          {/if}
        </CyexPopup>
      {:else if popup?.slot === 2}
        <CyexPopup title="Supporter+" anchor={popup.anchor} onclose={close}>
          <div class="plus-head">
            {#if panel.picture}
              <img class="plus-pic" src={panel.picture['4x'] || panel.picture['2x'] || panel.picture['1x']} alt="Your Supporter+ badge" />
            {:else}
              <span class="plus-pic empty"><Icon name="image" size={24} /></span>
            {/if}
            <div class="plus-lines">
              <strong>{panel.review}</strong>
              {#if panel.plan}<span class="note">{panel.plan}</span>{/if}
            </div>
          </div>
          {#if panel.link}
            {@const link = panel.link}
            <Button small icon="popout" onclick={() => open(link.url)}>{link.label}</Button>
          {/if}
        </CyexPopup>
      {/if}

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
  /* A slot: the button that opens its pop-up, then its switch. */
  .slot {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 44px;
    padding-right: 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    transition: border-color var(--fast);
  }
  .slot:has(.slot-open[aria-expanded='true']) {
    border-color: var(--accent-border);
  }
  .slot-open {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 100%;
    padding: 0 8px 0 10px;
    background: none;
    border: 0;
    border-right: 1px solid var(--border);
    border-radius: var(--radius) 0 0 var(--radius);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition: background var(--fast);
  }
  .slot-open:hover {
    background: var(--surface-hover);
  }
  .slot-open:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .slot-open > :global(svg) {
    flex: none;
    color: var(--dim);
  }
  /* Switched off: the choice stays in sight, quieter. */
  .slot.off .slot-value img {
    opacity: 0.45;
  }
  .slot.off .slot-text {
    color: var(--muted);
  }
  .slot-key {
    flex: none;
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
  .slot-open[aria-busy='true'] {
    cursor: progress;
  }
  .slot-open[aria-busy='true'] .slot-value {
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
  /* V109-B05 The pop-ups' parts. */
  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tools > :global(:first-child) {
    flex: 1;
    min-width: 0;
  }
  .grid {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
  }
  .more {
    align-self: flex-start;
  }
  .plus-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .plus-pic {
    flex: none;
    width: 72px;
    height: 72px;
    object-fit: contain;
    border-radius: var(--radius);
  }
  .plus-pic.empty {
    display: grid;
    place-items: center;
    background: var(--surface);
    border: 1px dashed var(--border-strong);
    color: var(--dim);
  }
  .plus-lines {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
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
