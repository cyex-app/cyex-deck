<!-- V109-B02 cyex cosmetics of one account: owned badges, which one shows, the Supporter+ badge, a
     preview as chat shows it and codes. No sign-in of its own: the app gets a cyex session with
     the account's Twitch sign-in. Changes save at once and roll back with the error. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { AccountView } from '../../ipc/bindings/AccountView';
  import { cyexIpc, type CyexMe, type CyexPics, type CyexView } from '../../ipc/cyex';
  import { systemIpc } from '../../ipc/system';
  import { density } from '../../lib/density.svelte';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import {
    choiceOf,
    founderNumber,
    nothingOwned,
    owns,
    plusNote,
    SHOP,
    shownBadges,
    UPLOAD,
    type Choice,
  } from './cyexBadges';

  let { account }: { account: AccountView } = $props();

  let view = $state.raw<CyexView | null>(null);
  let loading = $state(false);
  let saving = $state(false);
  let error = $state('');
  let code = $state('');
  let redeeming = $state(false);
  let redeemed = $state('');
  let redeemError = $state('');

  const me = $derived(view?.state === 'ready' ? view.me : null);

  async function load() {
    loading = true;
    try {
      view = await cyexIpc.view(account.userId);
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
    const guess = [...(main ? [main] : []), ...(custom && me.plus.live ? ['custom'] : [])];
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

  function pick(c: Choice) {
    if (!me || choiceOf(me) === c) return;
    void save(c === 'none' ? null : c, me.equipped.custom);
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
      await load();
    } catch (err) {
      redeemError = errorText(err);
    } finally {
      redeeming = false;
    }
  }

  const CHOICES: { value: Choice; label: string }[] = [
    { value: 'founder', label: 'Founder' },
    { value: 'supporter', label: 'Supporter' },
    { value: 'none', label: 'None' },
  ];
</script>

<div class="cyex" aria-busy={loading}>
  {#if !view}
    <p class="note">Loading…</p>
  {:else if view.state === 'offline'}
    <div class="row">
      <span class="note">Can’t reach cyex right now.</span>
      <Button small disabled={loading} onclick={() => void load()}>{loading ? 'Trying…' : 'Retry'}</Button>
    </div>
  {:else if view.state === 'noToken'}
    <p class="note">cyex needs this account’s Twitch sign-in. Sign in again above, then come back.</p>
  {:else if me}
    {@const founder = view.state === 'ready' ? view.founder : null}
    {@const supporter = view.state === 'ready' ? view.supporter : null}
    {#if nothingOwned(me)}
      <div class="row">
        <span class="note">No cyex badges on this account yet.</span>
        <Button small variant="primary" onclick={() => open(SHOP)}>Get a badge</Button>
      </div>
    {:else}
      <div class="row">
        <span class="label">Owned</span>
        <span class="owned">
          {#if owns(me, 'founder') && founder}
            <span class="chip"><img class="badge" src={src(founder)} alt="" />Founder #{founderNumber(me)}</span>
          {/if}
          {#if owns(me, 'supporter') && supporter}
            <span class="chip"><img class="badge" src={src(supporter)} alt="" />Supporter</span>
          {/if}
          {#if owns(me, 'custom')}
            <span class="chip">
              {#if me.plus.live}<img class="badge" src={src(me.plus.live)} alt="" />{/if}Supporter+{#if me.plus.until}<span class="dim"> until {me.plus.until.slice(0, 10).replaceAll('-', '/')}</span>{/if}
            </span>
          {/if}
        </span>
      </div>

      <div class="row">
        <span class="label">Badge</span>
        <span class="choices" role="radiogroup" aria-label="Badge">
          {#each CHOICES as c (c.value)}
            {@const have = c.value === 'none' || owns(me, c.value)}
            <button
              role="radio"
              class="choice"
              class:on={choiceOf(me) === c.value}
              aria-checked={choiceOf(me) === c.value}
              disabled={!have || saving}
              onclick={() => pick(c.value)}
            >
              {c.label}
            </button>
            {#if !have}<Button small variant="link" onclick={() => open(SHOP)}>Get it</Button>{/if}
          {/each}
        </span>
      </div>

      {#if owns(me, 'custom')}
        <div class="row">
          <span class="label">Supporter+ badge</span>
          <Toggle
            label="Supporter+ badge"
            checked={me.equipped.custom}
            disabled={saving}
            onchange={(v) => void save(me.equipped.main, v)}
          />
          {#if me.plus.live}
            <img class="badge big" src={me.plus.live['4x'] || me.plus.live['2x']} alt="Your Supporter+ badge" />
          {:else}
            <span class="note">{plusNote(me)}</span>
          {/if}
          <Button small variant="link" onclick={() => open(UPLOAD)}>Upload or change it</Button>
        </div>
      {/if}

      <div class="row">
        <span class="label">In chat</span>
        <span class="preview">
          {#each shownBadges(me, founder ?? { '1x': '', '2x': '', '4x': '' }, supporter ?? { '1x': '', '2x': '', '4x': '' }) as b (b.key)}
            <img class="badge" src={src(b.pics)} alt={b.title} use:tooltip={{ label: b.title }} />
          {/each}
          <strong class="who">{account.displayName}</strong><span class="dim">: hi chat</span>
        </span>
      </div>
      {#if error}<p class="bad" role="alert">{error}</p>{/if}
    {/if}

    <form class="row" onsubmit={redeem}>
      <span class="label">Have a code?</span>
      <input
        class="field"
        placeholder="Code"
        aria-label="cyex code"
        autocomplete="off"
        spellcheck="false"
        maxlength="64"
        bind:value={code}
      />
      <Button small type="submit" disabled={!code.trim() || redeeming}>{redeeming ? 'Redeeming…' : 'Redeem'}</Button>
    </form>
    {#if redeemed}<p class="ok" role="status">{redeemed}</p>{/if}
    {#if redeemError}<p class="bad" role="alert">{redeemError}</p>{/if}
  {/if}
</div>

<style>
  .cyex {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px 0 8px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    min-height: 32px;
  }
  .label {
    flex: none;
    width: 130px;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .note,
  .dim {
    color: var(--dim);
    font-size: 0.85rem;
  }
  p.note {
    margin: 4px 0;
  }
  .owned,
  .preview {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    min-width: 0;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 0.85rem;
  }
  .badge {
    width: 18px;
    height: 18px;
    flex: none;
  }
  .badge.big {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
  }
  .preview {
    gap: 3px;
    font-size: 0.875rem;
  }
  .who {
    margin-left: 1px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 260px;
  }
  .choices {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
  }
  .choice {
    padding: 3px 10px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.82rem;
  }
  .choice.on {
    background: var(--accent-dim);
    border-color: var(--accent-border);
    color: var(--accent-text);
  }
  .choice:disabled {
    color: var(--dim);
    cursor: default;
  }
  .choice.on:disabled {
    color: var(--accent-text);
  }
  form input {
    flex: 1;
    min-width: 120px;
    max-width: 240px;
  }
  .ok,
  .bad {
    margin: 2px 0;
    font-size: 0.82rem;
  }
  .ok {
    color: var(--text);
  }
  .bad {
    color: var(--red);
  }
</style>
