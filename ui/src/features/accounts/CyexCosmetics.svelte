<!-- V109-B02/B03 cyex cosmetics of one account. No sign-in of its own: the app gets a cyex session
     with the account's Twitch sign-in. B03 layout: the chat preview on top with the store link,
     then the two choices (one main badge, the Supporter+ switch), a code behind a link. Nothing to
     set: one line and "Open store". Changes save at once and roll back with the error. -->
<script lang="ts">
  import { tick } from 'svelte';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { radioKey } from '../../components/select';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { AccountView } from '../../ipc/bindings/AccountView';
  import { cyexIpc, type CyexMe, type CyexPics, type CyexView } from '../../ipc/cyex';
  import { systemIpc } from '../../ipc/system';
  import { density } from '../../lib/density.svelte';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { badgeOptions, choiceOf, nothingToSet, plusState, shownBadges, STORE, type Choice } from './cyexBadges';

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
  let group = $state<HTMLElement>();

  const me = $derived(view?.state === 'ready' ? view.me : null);
  const NO_PICS: CyexPics = { '1x': '', '2x': '', '4x': '' };

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

  function pick(c: Choice) {
    if (!me || choiceOf(me) === c) return;
    void save(c === 'none' ? null : c, me.equipped.custom);
  }

  /** Arrows move between the badges this account owns and pick them; the rest are skipped. */
  function onkeydown(e: KeyboardEvent) {
    if (!me) return;
    const owned = badgeOptions(me).filter((o) => o.owned);
    const at = Math.max(owned.findIndex((o) => o.value === choiceOf(me)), 0);
    const to = radioKey(e, at, owned.length);
    if (to === null) return;
    e.preventDefault();
    pick(owned[to].value);
    (group?.querySelector(`[data-value="${owned[to].value}"]`) as HTMLElement | null)?.focus();
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
    {@const chosen = choiceOf(me)}
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

      <div class="row">
        <span class="key">Badge</span>
        <!-- svelte-ignore a11y_interactive_supports_focus (the radios inside hold the tab stop) -->
        <div class="seg" role="radiogroup" aria-label="Badge" bind:this={group} {onkeydown}>
          {#each badgeOptions(me) as o (o.value)}
            <button
              role="radio"
              data-value={o.value}
              aria-checked={chosen === o.value}
              aria-disabled={!o.owned}
              tabindex={chosen === o.value ? 0 : -1}
              class:on={chosen === o.value}
              class:locked={!o.owned}
              onclick={() => o.owned && pick(o.value)}
              use:tooltip={o.owned ? { label: '' } : { label: 'Not yours yet', hint: 'In the store.' }}
            >
              {#if !o.owned}<Icon name="lock" size={12} />{/if}{o.label}
            </button>
          {/each}
        </div>
      </div>

      {#if plus.kind !== 'never'}
        <div class="row">
          <span class="key">
            Supporter+
            {#if plus.plan}<span class="plan">{plus.plan}</span>{/if}
          </span>
          <div class="plus">
            {#if plus.kind !== 'ended'}
              <Toggle label="Show Supporter+ badge" checked={me.equipped.custom} onchange={(v) => void save(me.equipped.main, v)} />
            {/if}
            {#if me.plus.live && plus.kind === 'live'}
              <img class="badge" src={src(me.plus.live)} alt="Your Supporter+ badge" />
            {/if}
            {#if plus.note}<span class="note">{plus.note}</span>{/if}
            {#if plus.link}
              {@const link = plus.link}
              <Button small variant="link" hit onclick={() => open(link.url)}>{link.label}</Button>
            {/if}
          </div>
        </div>
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
  .row {
    display: flex;
    align-items: center;
    gap: 6px 12px;
    min-height: 44px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
  }
  .key {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 1px;
    /* Lines up the controls with the status column of the card's lines above (140 + 10). */
    width: 125px;
    font-weight: 600;
  }
  .plan {
    color: var(--dim);
    font-size: var(--text-hint);
    font-weight: 400;
    white-space: nowrap;
  }
  .plus {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    min-width: 0;
  }
  /* The picker: the app's segmented control, with badges not owned shown but off. */
  .seg {
    display: inline-flex;
    flex-wrap: wrap;
    max-width: 100%;
    padding: 2px;
    gap: 2px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .seg button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 10px;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
    white-space: nowrap;
    transition:
      background var(--fast),
      box-shadow var(--fast),
      color var(--fast);
  }
  .seg button:hover:not(.locked) {
    color: var(--text);
  }
  .seg button.on {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .seg button.locked {
    color: var(--dim);
    font-weight: 500;
    cursor: default;
  }
  :global(:root[data-theme='light']) .seg {
    background: var(--neutral-chip);
  }
  :global(:root[data-theme='light']) .seg button.on {
    background: var(--elevated);
    box-shadow:
      0 0 0 1px var(--accent-text),
      0 1px 2px var(--shadow);
    color: var(--accent-text);
  }
  @media (prefers-contrast: more) {
    .seg {
      border-color: var(--text);
    }
    .seg button.on,
    :global(:root[data-theme='light']) .seg button.on {
      box-shadow: 0 0 0 1px var(--text);
    }
  }
  @media (forced-colors: active) {
    .seg button.on,
    :global(:root[data-theme='light']) .seg button.on {
      background: Highlight;
      box-shadow: none;
      color: HighlightText;
      forced-color-adjust: none;
    }
    .seg button.locked {
      color: GrayText;
    }
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
  /* Narrow: labels sit above their control; the empty state's button goes under its text. */
  @container (max-width: 420px) {
    .row {
      flex-direction: column;
      align-items: flex-start;
      gap: 6px;
      padding: 8px 12px;
    }
    .key {
      width: auto;
      flex-direction: row;
      align-items: baseline;
      gap: 8px;
    }
    .empty {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
