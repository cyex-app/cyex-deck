<!-- V112-16 The cosmetics page's cyex.tv side: the badges and paints the account owns on cyex.tv,
     None first. A pick saves at once (selectCosmetics) and rolls back with the error. Signed out:
     one line and the sign-in. Owns nothing: one line and a link to cyex.tv. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { AccountView } from '../../ipc/bindings/AccountView';
  import { cyextvIpc, type TvCosmetics, type TvKind, type TvOwned } from '../../ipc/cyextv';
  import { systemIpc } from '../../ipc/system';
  import { errorText } from '../../lib/errors';
  import { reduced } from '../../lib/motion';
  import { toast } from '../../lib/toast.svelte';
  import { menuSheet } from '../cosmetics/sheet';
  import { tvSignIn } from './cyextvSignIn.svelte';

  let { account }: { account: AccountView } = $props();

  const SITE = 'https://cyex.tv';

  let view = $state.raw<TvCosmetics | null>(null);
  let loading = $state(false);
  let saving = $state<TvKind | null>(null);

  const owned = $derived(view?.state === 'ready' ? view.owned : null);
  const waiting = $derived(tvSignIn.waiting === account.userId);

  // V112-19 Paints are drawn the way chat draws them: their classes in one sheet while this is open.
  const paints = $derived(owned?.paints ?? []);
  $effect(() => {
    const styles = paints.flatMap((p) => (p.style ? [p.style] : []));
    if (!styles.length) return;
    const sheet = menuSheet(styles);
    sheet.adopt();
    return () => sheet.drop();
  });

  async function load() {
    loading = true;
    const id = account.userId;
    try {
      const got = await cyextvIpc.cosmetics(id);
      if (id === account.userId) view = got;
    } catch (e) {
      view = { state: 'offline', message: errorText(e) };
    } finally {
      loading = false;
    }
  }

  // Again after a sign-in or sign-out (Accounts or here).
  $effect(() => {
    void account.userId;
    void account.cyextv.signedIn;
    void load();
  });

  async function pick(kind: TvKind, id: string | null) {
    if (!owned || saving) return;
    const before = owned;
    if ((kind === 'badge' ? before.badgeId : before.paintId) === id) return;
    const guess: TvOwned = kind === 'badge' ? { ...before, badgeId: id } : { ...before, paintId: id };
    view = { state: 'ready', owned: guess };
    saving = kind;
    try {
      view = { state: 'ready', owned: await cyextvIpc.select(account.userId, kind, id) };
    } catch (e) {
      view = { state: 'ready', owned: before };
      toast.error(errorText(e));
    } finally {
      saving = null;
    }
  }

  function open(url: string) {
    systemIpc.openLink(url).catch((e) => toast.error(errorText(e)));
  }
</script>

<div class="tv">
  {#if !view}
    <div class="box state"><span class="note">Loading…</span></div>
  {:else if view.state === 'signedOut' || view.state === 'ended'}
    <div class="box state">
      <span class="note">
        {#if waiting}Finish in your browser.{:else if view.state === 'ended'}Your cyex.tv sign-in ran out.{:else}Sign in to wear your cyex.tv badge and paint.{/if}
      </span>
      {#if waiting}
        <Button small onclick={() => tvSignIn.cancel()}>Cancel</Button>
      {:else}
        <Button small variant="primary" disabled={tvSignIn.waiting !== null} onclick={() => void tvSignIn.start(account.userId)}>Sign in to cyex.tv</Button>
      {/if}
    </div>
  {:else if view.state === 'offline'}
    <div class="box state">
      <span class="note">{view.message}</span>
      <Button small disabled={loading} onclick={() => void load()}>{loading ? 'Trying…' : 'Try again'}</Button>
    </div>
  {:else if owned && !owned.badges.length && !owned.paints.length}
    <div class="box state">
      <span class="note">Nothing here yet.</span>
      <Button small variant="link" icon="popout" hit onclick={() => open(SITE)}>cyex.tv</Button>
    </div>
  {:else if owned}
    <div class="box">
      <section class="sec" aria-labelledby="tv-badge-{account.userId}">
        <span class="label" id="tv-badge-{account.userId}">Badge</span>
        <div class="grid" role="radiogroup" aria-labelledby="tv-badge-{account.userId}">
          <button class="cell" role="radio" aria-checked={owned.badgeId === null} class:on={owned.badgeId === null} disabled={saving !== null} onclick={() => void pick('badge', null)}>
            <span class="pic none"><Icon name="ban" size={16} /></span><span class="nm">None</span>
          </button>
          {#each owned.badges as b (b.id)}
            {@const on = owned.badgeId === b.id}
            <button class="cell" role="radio" aria-checked={on} class:on disabled={saving !== null} use:tooltip={{ label: b.tooltip || b.name }} onclick={() => void pick('badge', b.id)}>
              {#if b.picture}
                <img class="pic" src={reduced() && b.picture.still ? b.picture.still : b.picture.src} srcset={reduced() && b.picture.still ? undefined : b.picture.srcset} width="18" height="18" alt="" decoding="async" />
              {:else}
                <span class="pic none"></span>
              {/if}
              <span class="nm">{b.name}</span>
            </button>
          {/each}
        </div>
      </section>
      <section class="sec" aria-labelledby="tv-paint-{account.userId}">
        <span class="label" id="tv-paint-{account.userId}">Paint</span>
        <div class="grid" role="radiogroup" aria-labelledby="tv-paint-{account.userId}">
          <button class="cell" role="radio" aria-checked={owned.paintId === null} class:on={owned.paintId === null} disabled={saving !== null} onclick={() => void pick('paint', null)}>
            <span class="who">{account.displayName}</span><span class="nm">None</span>
          </button>
          {#each owned.paints as p (p.id)}
            {@const on = owned.paintId === p.id}
            <button class="cell" role="radio" aria-checked={on} class:on disabled={saving !== null} use:tooltip={{ label: p.name }} onclick={() => void pick('paint', p.id)}>
              <span class="who {p.style ? `painted paint-${p.style.id}` : ''}">{account.displayName}</span><span class="nm">{p.name}</span>
            </button>
          {/each}
        </div>
      </section>
      <p class="foot">
        <span class="note">Signed in as {owned.account.displayName}</span>
        <Button small variant="link" icon="popout" hit onclick={() => open(SITE)}>cyex.tv</Button>
      </p>
    </div>
  {/if}
</div>

<style>
  .tv {
    padding: 2px 0 10px;
  }
  .box {
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
    flex: 1;
    min-width: 0;
    color: var(--dim);
    font-size: var(--text-sub);
  }
  .sec {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px 12px;
    border-bottom: 1px solid var(--border);
  }
  .label {
    color: var(--dim);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 6px;
  }
  .cell {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    min-height: 36px;
    padding: 4px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
    transition: border-color var(--fast);
  }
  .cell:hover:not(:disabled) {
    border-color: var(--border-strong);
  }
  .cell.on {
    background: var(--accent-dim);
    border-color: var(--accent-border);
  }
  .cell:disabled {
    cursor: default;
  }
  .pic {
    flex: none;
    width: 18px;
    height: 18px;
  }
  .pic.none {
    display: grid;
    place-items: center;
    color: var(--dim);
  }
  .who {
    flex: none;
    max-width: 55%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--chat-font);
    font-weight: 700;
  }
  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .cell.on .nm {
    color: var(--accent-text);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    padding: 6px 12px;
  }
</style>
