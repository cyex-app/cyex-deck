<!-- V109-B02/B03 cyex cosmetics of one account. No sign-in of its own: the app gets a cyex session
     with the account's Twitch sign-in. The chat preview on top with the store link, then the
     badges, a code behind a link. Nothing to set: one line and "Open store". Changes save at once
     and roll back with the error.
     V109-B08 The badges sit inline, like the 7TV badge page: "Badge" (one one-time badge as row
     cards, None first) and "Supporter+" (the uploaded picture, its review, the uploads). Each
     section has its own switch in its header; no dropdowns, no pop-ups. -->
<script lang="ts">
  import { tick, untrack } from 'svelte';
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
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import ItemGrid from '../cosmetics/ItemGrid.svelte';
  import {
    filterRows,
    gridHeight,
    nothingToSet,
    plusGate,
    plusPanel,
    plusState,
    readSlot1,
    saveSlot1,
    SEARCH_FROM,
    shownBadges,
    slot1Choice,
    slot1Current,
    slot1Missing,
    slot1Pick,
    slot1Rows,
    STORE,
    uploadRows,
    type UploadRow,
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
  /** V109-B04 What the plan check said when it couldn't tell. */
  let plusError = $state('');
  let search = $state('');
  /** V109-B08 Only with many badges: tiles instead of rows, for this session. */
  let tiles = $state(false);
  /** The badge grid's width, for its height. */
  let gridW = $state(0);
  /** V109-B06 The upload history, loaded after the plan check says the plan is on. */
  let uploads = $state<{ rows: UploadRow[]; loading: boolean; error: string }>({ rows: [], loading: false, error: '' });
  let showAll = $state(false);
  let restoring = $state<number | null>(null);
  /** V109-B07 Owner previews as object URLs, by their API URL, while the editor is open. */
  let previews = $state<Record<string, string>>({});
  const asking = new Set<string>();
  /** Slot 1's choice kept while its switch is off (per account, on this PC). */
  let remembered = $state<string | null>(null);

  const me = $derived(view?.state === 'ready' ? view.me : null);
  const NO_PICS: CyexPics = { '1x': '', '2x': '', '4x': '' };
  // One gate per account: a look from the last 30 s counts, the editor's own load included.
  const gate = $derived.by(() => {
    const id = account.userId;
    return plusGate(() => cyexIpc.view(id));
  });

  function dropPreviews() {
    for (const u of Object.values(previews)) URL.revokeObjectURL(u);
    previews = {};
    asking.clear();
  }

  $effect(() => dropPreviews);

  /** Placeholder until it comes; a failure keeps the placeholder. */
  async function loadPreview(url: string) {
    if (previews[url] || asking.has(url)) return;
    asking.add(url);
    const id = account.userId;
    try {
      const bytes = await cyexIpc.preview(id, url);
      if (id !== account.userId || !asking.has(url)) return;
      previews = { ...previews, [url]: URL.createObjectURL(new Blob([bytes], { type: 'image/png' })) };
    } catch {
      // The placeholder stays.
    }
  }

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
    void checkPlan();
  }

  $effect(() => {
    void account.userId;
    remembered = readSlot1(account.userId);
    search = '';
    showAll = false;
    uploads = { rows: [], loading: false, error: '' };
    untrack(dropPreviews);
    void load();
  });

  /** V109-B08 The section is shown: the plan check (30 s cache), then the uploads when it's on. */
  async function checkPlan() {
    if (view?.state !== 'ready') return;
    plusError = '';
    const got = await gate.check();
    if (got.kind === 'error') {
      plusError = got.message;
      return;
    }
    if (got.fetched && !saving) view = got.fetched;
    if (got.kind === 'open') void loadUploads();
  }

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

  /** V109-B05 A card: on at once with that badge; None takes it off and forgets it. */
  function pick1(id: string | null) {
    if (!me) return;
    if (id === null) remember(null);
    const next = slot1Pick(me, id);
    if (!next) return;
    remember(next.main);
    void save(next.main, me.equipped.custom);
  }

  /** V109-B05 The Badge switch: off keeps the choice here, on puts it back. */
  function switch1(on: boolean) {
    if (!me) return;
    const choice = slot1Choice(me, remembered);
    if (!choice) return;
    remember(choice);
    void save(on ? choice : null, me.equipped.custom);
  }

  /** V109-B06 After the plan check, never before: a locked section asks for nothing more. */
  async function loadUploads() {
    const id = account.userId;
    uploads = { rows: uploads.rows, loading: true, error: '' };
    try {
      const h = await cyexIpc.history(id);
      if (id !== account.userId) return;
      uploads = { rows: uploadRows(h), loading: false, error: '' };
      for (const r of uploads.rows) if (r.preview) void loadPreview(r.preview);
    } catch (e) {
      if (id === account.userId) uploads = { rows: [], loading: false, error: errorText(e) };
    }
  }

  /** V109-B06 "Use again": that picture goes live; the view and the list follow. */
  async function restore(row: UploadRow) {
    if (restoring !== null) return;
    restoring = row.id;
    try {
      await cyexIpc.restore(account.userId, row.id);
      await load();
    } catch (e) {
      uploads = { ...uploads, error: errorText(e) };
    } finally {
      restoring = null;
    }
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
    {@const all = slot1Rows(me, view)}
    {@const many = all.length + 1 > SEARCH_FROM}
    {@const rows = many ? filterRows(all, search) : all}
    {@const locked = !me.plus.active}
    {@const panel = plusPanel(me)}
    {@const shownUploads = showAll ? uploads.rows : uploads.rows.slice(0, 3)}
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

      <!-- V109-B08 Badge: one one-time badge, picked from row cards; the switch shows or hides it. -->
      <section class="sec" aria-labelledby="cyex-badge-{account.userId}">
        <div class="sec-head">
          <span class="label" id="cyex-badge-{account.userId}">Badge</span>
          <Toggle label="Show badge in chat" checked={on1} disabled={!choice || saving} onchange={switch1} />
        </div>
        {#if many}
          <div class="tools">
            <SearchField small placeholder="Search badges" value={search} oninput={(t) => (search = t)} />
            <IconToggle icon="grid" label="Tiles" pressed={tiles} onchange={(v) => (tiles = v)} />
          </div>
        {/if}
        <div class="grid" class:off={!on1} bind:clientWidth={gridW} style:height="{gridHeight(rows.length + 1, gridW, !tiles)}px">
          <ItemGrid kind="badge" {rows} selected={choice} name={account.displayName} label="Badge" asRows={!tiles} onpick={(id) => pick1(id)} />
        </div>
        {#if slot1Missing(me)}
          <span class="more"><Button small variant="link" hit onclick={() => open(STORE)}>Get more in store</Button></span>
        {/if}
      </section>

      <!-- V109-B08 Supporter+: the uploaded picture, its review and the uploads, after the plan check. -->
      <section class="sec" aria-labelledby="cyex-plus-{account.userId}">
        <div class="sec-head">
          <span class="head-text">
            <span class="label" id="cyex-plus-{account.userId}">Supporter+</span>
            {#if plus.plan}<span class="plan">{plus.plan}</span>{/if}
          </span>
          {#if !locked}
            <Toggle label="Show Supporter+ badge in chat" checked={me.equipped.custom} disabled={saving} onchange={(v) => void save(me.equipped.main, v)} />
          {/if}
        </div>
        {#if plusError}
          <p class="msg bad" role="alert">{plusError}</p>
        {:else if locked}
          <p class="locked">
            <Icon name="lock" size={12} />
            <span class="note">{plus.kind === 'ended' ? 'Your own badge is off while the plan is.' : 'Your own badge in chat, with Supporter+.'}</span>
            {#if plus.kind === 'ended' && plus.link}
              {@const link = plus.link}
              <Button small variant="link" hit onclick={() => open(link.url)}>{link.label}</Button>
            {:else}
              <Button small variant="link" hit onclick={() => open(STORE)}>Get it in store</Button>
            {/if}
          </p>
        {:else}
          <div class="own" class:off={!me.equipped.custom}>
            {#if panel.picture}
              <img class="own-pic" src={panel.picture['4x'] || panel.picture['2x'] || panel.picture['1x']} alt="Your Supporter+ badge" />
            {:else}
              <span class="own-pic empty"><Icon name="image" size={20} /></span>
            {/if}
            <span class="own-text">
              <strong>{panel.review}</strong>
              {#if panel.reason}<span class="reason">{panel.reason}</span>{/if}
              {#if panel.link}
                {@const link = panel.link}
                <span><Button small icon="popout" onclick={() => open(link.url)}>{link.label}</Button></span>
              {/if}
            </span>
          </div>
          <div class="uploads" aria-busy={uploads.loading}>
            <span class="label">Uploads</span>
            {#if uploads.error}
              <p class="msg bad" role="alert">{uploads.error}</p>
              <span><Button small variant="link" hit onclick={() => void loadUploads()}>Try again</Button></span>
            {:else if uploads.loading && !uploads.rows.length}
              <span class="note">Loading…</span>
            {:else if !uploads.rows.length}
              <span class="note">Nothing uploaded yet</span>
            {:else}
              <ul>
                {#each shownUploads as r (r.id)}
                  <li>
                    {#if r.pics}
                      <img class="thumb" src={src(r.pics)} alt="" />
                    {:else if r.preview && previews[r.preview]}
                      <img class="thumb" src={previews[r.preview]} alt="" />
                    {:else}
                      <span class="thumb empty"><Icon name="image" size={12} /></span>
                    {/if}
                    <span class="up-text">
                      <span class="up-line"><span class:live={r.live}>{r.status}</span><span class="date">{r.date}</span></span>
                      {#if r.reason}<span class="reason">{r.reason}</span>{/if}
                    </span>
                    {#if r.restorable}
                      <Button small variant="link" hit disabled={restoring !== null} onclick={() => void restore(r)}>
                        {restoring === r.id ? 'Using…' : 'Use again'}
                      </Button>
                    {/if}
                  </li>
                {/each}
              </ul>
              {#if uploads.rows.length > 3}
                <span>
                  <Button small variant="link" hit onclick={() => (showAll = !showAll)}>
                    {showAll ? 'Show less' : `Show all ${formatNumber(uploads.rows.length)}`}
                  </Button>
                </span>
              {/if}
            {/if}
          </div>
        {/if}
      </section>

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
  /* V109-B08 A section: the Settings row's header (label left, switch right), then its body. */
  .sec {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px 12px;
    border-bottom: 1px solid var(--border);
  }
  .sec-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 24px;
  }
  .head-text {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 10px;
    min-width: 0;
  }
  .label {
    color: var(--dim);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .plan {
    color: var(--dim);
    font-size: var(--text-hint);
    font-variant-numeric: tabular-nums;
  }
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
  /* Switched off: the choice stays in sight and can be picked, quieter. */
  .grid.off :global(.row img),
  .own.off .own-pic {
    opacity: 0.45;
  }
  .grid.off :global(.row .rn) {
    color: var(--muted);
  }
  .more {
    align-self: flex-start;
  }
  .locked {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    margin: 0;
    color: var(--dim);
  }
  .own {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .own-pic {
    flex: none;
    width: 72px;
    height: 72px;
    object-fit: contain;
    border-radius: var(--radius);
  }
  .own-pic.empty {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface);
    border: 1px dashed var(--border-strong);
    color: var(--dim);
  }
  .own-text {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    min-width: 0;
  }
  .reason {
    color: var(--muted);
    font-size: var(--text-hint);
    overflow-wrap: anywhere;
  }
  .uploads {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 4px;
  }
  .uploads ul {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .uploads li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 4px 0;
  }
  .uploads li + li {
    border-top: 1px solid var(--border);
  }
  .thumb {
    flex: none;
    width: 24px;
    height: 24px;
    object-fit: contain;
    border-radius: var(--radius);
  }
  .thumb.empty {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface);
    border: 1px dashed var(--border-strong);
    color: var(--dim);
  }
  .up-text {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }
  .up-line {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .date {
    color: var(--dim);
    font-size: var(--text-hint);
    font-variant-numeric: tabular-nums;
  }
  .live {
    font-weight: 600;
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
  /* Narrow: the empty state's button goes under its text. */
  @container (max-width: 420px) {
    .empty {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
