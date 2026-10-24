<!-- One account in the account manager (1.23): its two sign-ins, joined chats, default,
     remove, its Twitch profile (2.2) and its sign-in on Twitch's pages in the app (3.8).
     A missing sign-in is added to this same account (V102-19). -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import GateFlag from '../../components/GateFlag.svelte';
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { accountsIpc } from '../../ipc/accounts';
  import { cosmeticsIpc } from '../../ipc/cosmetics';
  import { formatNumber } from '../../lib/format';
  import { subsIpc } from '../../ipc/subs';
  import type { AccountView } from '../../ipc/bindings/AccountView';
  import type { TokenKind } from '../../ipc/bindings/TokenKind';
  import type { TokenView } from '../../ipc/bindings/TokenView';
  import { accounts } from '../../lib/accounts.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { alsoSevenTv } from './alsoSevenTv.svelte';
  import GqlUnlock from './GqlUnlock.svelte';
  import SevenTvPaste from './SevenTvPaste.svelte';
  import ProfileEditor from './ProfileEditor.svelte';
  import ProfileImages from './ProfileImages.svelte';
  import { profileImagesIpc } from '../../ipc/profileImages';
  import { signIn } from './signIn.svelte';
  import { twitchPages } from './twitchPages.svelte';
  import { twitchLeft } from '../unlock/choice';
  import { unlock } from '../unlock/unlock.svelte';
  import { accountName, missingScopesText, needsSignIn, tokenStatus } from './status';
  import { errorText } from '../../lib/errors';
  import { untrack } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { slide } from 'svelte/transition';
  import { BASE, reduced } from '../../lib/motion';
  import { settingsPlace } from '../settings/place.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';

  let { account }: { account: AccountView } = $props();
  twitchPages.watch();

  const TOKENS: { kind: TokenKind; title: string; none: string; tip?: string }[] = [
    { kind: 'helix', title: 'Twitch', none: 'Needed for payment methods, subscriptions and mod tools.' },
    { kind: 'gql', title: 'Extra Twitch features', tip: 'The miner, following, channel points, GIFs, power-ups, votes, raids, badge picking and profile edits.', none: 'Not added. Unlocks the miner, following, channel points, GIFs and power-ups.' },
  ];

  const conn = $derived(accounts.connectionsOf(account.userId));
  const bothTokens = $derived(account.helix !== null && account.gql !== null);
  /** V102-19 The GQL options: asked for, or offered right after Sign in with Twitch. */
  let unlocking = $state(false);
  /** V108-7K The "Paste a 7TV token" steps. */
  let pasting = $state(false);
  const showUnlock = $derived(
    (unlocking || signIn.offer === account.userId) && (account.gql === null || needsSignIn(account.gql)),
  );

  // V107-Mo The profile editor stays open when you come back to Settings.
  let editing = $state(untrack(() => settingsPlace.open('accounts', `profile:${account.userId}`)));
  $effect(() => settingsPlace.setOpen('accounts', `profile:${account.userId}`, editing));
  /** V107-R19 Mocks test runs: the images work without a GQL sign-in. */
  let imagesMock = $state(false);
  profileImagesIpc.mocked().then((m) => (imagesMock = m)).catch(() => {});
  const canEdit = $derived(accounts.can('displayName', account.userId) || imagesMock);
  /** V106-31/32 7TV rejected the session or it ran out. */
  const ended = $derived(account.seventv.consent && !account.seventv.signedIn && account.seventv.expired);

  function tokenOf(kind: TokenKind): TokenView | null {
    return kind === 'helix' ? account.helix : account.gql;
  }

  async function attempt(action: () => Promise<void>) {
    try {
      await action();
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  function removeAccount() {
    void attempt(() => accountsIpc.remove(account.userId));
  }

  // V1051-02 The same two clicks as removing the account.
  function withdraw() {
    void attempt(async () => {
      await cosmeticsIpc.withdraw(account.userId);
      toast.success(`7TV removed from ${account.displayName}.`);
    });
  }

  /** First what to unlock (7TV, subscriptions and wallet, or both). Twitch alone: its sign-in
   * runs right away; GQL alone shows its two options (code or token). */
  function add(kind: TokenKind) {
    const left = twitchLeft(account);
    const alone =
      kind === 'helix' ? () => void signIn.start('helix', account.userId)
      : left.length === 1 && left[0] === 'gql' ? () => (unlocking = true)
      : undefined;
    unlock.ask('settings', account.userId, { twitchAlone: alone });
  }

  /** V108-7P The prompt's Connect: the same 7TV sign-in (and consent) as the 7TV line. */
  function connectSevenTv() {
    const id = alsoSevenTv.take();
    if (id) unlock.ask('seventv', id);
  }

  function closeUnlock() {
    unlocking = false;
    if (signIn.offer === account.userId) signIn.offer = null;
  }
</script>

<section class="card">
  <header>
    <Avatar src={account.avatar} name={account.displayName} size={36} />
    <div class="who">
      <strong>{accountName(account)}</strong>
      {#if account.isDefault}<span class="default">Default</span>{/if}
    </div>
    {#if !account.isDefault}
      <Button small variant="ghost" onclick={() => attempt(() => accountsIpc.setDefault(account.userId))}>
        Make default
      </Button>
    {/if}
  </header>

  {#each TOKENS as t (t.kind)}
    {@const token = tokenOf(t.kind)}
    <div class="line">
      <span class="name" use:tooltip={{ label: t.title, hint: t.tip }}>{t.title}</span>
      {#if token}
        {@const status = tokenStatus(token)}
        <span class="status {status.tone}">{status.text}</span>
        {#if needsSignIn(token)}
          <Button small disabled={signIn.kind !== null} onclick={() => add(t.kind)}>
            Sign in again
          </Button>
        {/if}
        {#if bothTokens}
          <Button small variant="ghost" onclick={() => attempt(() => accountsIpc.removeToken(account.userId, t.kind))}>
            Remove
          </Button>
        {/if}
      {:else}
        <span class="status muted">{t.none}</span>
        <Button small disabled={signIn.kind !== null || (t.kind === 'gql' && showUnlock)} onclick={() => add(t.kind)}>
          {t.kind === 'helix' ? 'Sign in' : 'Add'}
        </Button>
      {/if}
    </div>
    {#if t.kind === 'gql' && showUnlock}<GqlUnlock {account} onclose={closeUnlock} />{/if}
  {/each}

  <!-- V1051-02 -->
  <div class="line">
    <span class="name">7TV</span>
    <!-- V106-31/32 A rejected or run-out session reads "Sign-in ran out", not "Not signed in". -->
    <span class="status" class:muted={!account.seventv.consent} class:bad={ended}>
      {#if !account.seventv.consent}Not used{:else if account.seventv.signedIn}{account.seventv.until ? `Signed in until ${account.seventv.until}` : 'Signed in'}{:else if ended}Sign-in ran out{:else}Not signed in{/if}
    </span>
    <!-- V1051-02 A session only counts with consent; Sign out never shows beside "Not used". -->
    {#if account.seventv.consent && account.seventv.signedIn}
      <Button small variant="ghost" onclick={() => attempt(() => cosmeticsIpc.signOut(account.userId))}>Sign out</Button>
    {:else}
      <Button small disabled={!!unlock.asked} onclick={() => unlock.ask('settings', account.userId)}>
        {ended ? 'Sign in again' : 'Sign in'}
      </Button>
      <Button small variant="ghost" disabled={pasting} onclick={() => (pasting = true)}>Paste a token</Button>
    {/if}
    {#if account.seventv.consent}
      <ConfirmButton label="Withdraw" confirm="Click again to withdraw" variant="ghost" onconfirm={withdraw} />
    {/if}
  </div>

  {#if pasting && !(account.seventv.consent && account.seventv.signedIn)}<SevenTvPaste {account} onclose={() => (pasting = false)} />{/if}

  <!-- V108-7P -->
  {#if alsoSevenTv.asking === account.userId}
    <div class="also" role="group" aria-label="Also connect 7TV?" transition:slide={{ duration: reduced() ? 0 : BASE, easing: cubicOut }}>
      <span><strong>Also connect 7TV?</strong> Your paint and badge in chat.</span>
      <Button small variant="primary" disabled={!!unlock.asked} onclick={connectSevenTv}>Connect</Button>
      <Button small variant="ghost" onclick={() => alsoSevenTv.later()}>Not now</Button>
    </div>
  {/if}

  {#if account.helix && !needsSignIn(account.helix) && account.missingScopes.length}
    <p class="warn" use:tooltip={{ label: account.missingScopes.join(', ') }}>
      <Icon name="alert" size={14} />
      {missingScopesText(account.missingScopes.length)}
    </p>
  {/if}

  <!-- V108-04 Twitch's own pages in the app keep their own login. -->
  <div class="line">
    <span class="name">Twitch pages</span>
    {#if twitchPages.has(account.userId)}
      <span class="status ok">Connected</span>
      <Button
        small
        variant="ghost"
        onclick={() =>
          attempt(async () => {
            await subsIpc.signOut(account.userId);
            toast.success('Twitch pages signed out.');
          })}
      >
        Sign out
      </Button>
    {:else}
      <span class="status muted">Not connected. Wallet, subscriptions, Bits and the clip editor ask you to log in.</span>
      <Button small disabled={twitchPages.connecting !== null} onclick={() => twitchPages.connect(account.userId)}>
        {twitchPages.connecting === account.userId ? 'Waiting for login…' : 'Connect'}
      </Button>
    {/if}
  </div>

  <div class="line">
    <span class="name">Joined chats</span>
    <span class="status">
      {#if conn?.joined}{formatNumber(conn.joined)}{:else}<span class="muted">None open</span>{/if}
      {#if conn?.refused.length}
        <span class="warn">· Twitch refused {conn.refused.map((l) => `#${l}`).join(', ')}</span>
      {/if}
    </span>
  </div>

  <!-- V107-H15 Opens and closes by height and opacity; reversing mid-way picks up where it is. -->
  {#if editing}
    <div class="editors" transition:slide={{ duration: reduced() ? 0 : BASE, easing: cubicOut }}>
      <ProfileEditor account={account.userId} /><ProfileImages account={account.userId} />
    </div>
  {/if}

  <footer>
    {#if canEdit}
      <Button small variant="ghost" icon="pencil" onclick={() => (editing = !editing)}>
        {editing ? 'Close profile' : 'Twitch profile'}
      </Button>
    {:else}
      <!-- V102-20 -->
      <span class="locked"><Button small variant="ghost" icon="pencil" disabled>Twitch profile</Button><GateFlag kind="gql" /></span>
    {/if}
    <ConfirmButton label="Remove account" confirm="Click again to remove" icon="trash" variant="ghost" onconfirm={removeAccount} />
  </footer>
</section>

<style>
  .card {
    margin-bottom: 12px;
    padding: 14px 16px 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }
  .who {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .who strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .default {
    padding: 1px 6px;
    border-radius: var(--radius);
    background: var(--accent-dim);
    color: var(--accent-text);
    font-size: 0.72rem;
    font-weight: 700;
  }
  .also {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 2px 0 8px;
    padding: 8px 12px;
    background: var(--bg);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius);
  }
  .also span {
    flex: 1;
    min-width: 0;
    color: var(--muted);
    font-size: 0.82rem;
  }
  .also strong {
    color: var(--text);
    font-size: 0.875rem;
    margin-right: 4px;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    border-top: 1px solid var(--border);
  }
  .name {
    flex: none;
    width: 140px;
    font-weight: 600;
  }
  .status {
    flex: 1;
    font-size: 0.85rem;
  }
  .ok {
    color: var(--text);
  }
  .muted {
    color: var(--dim);
  }
  .warn {
    color: var(--yellow);
  }
  .bad {
    color: var(--red);
  }
  p.warn {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 8px 0;
    border-top: 1px solid var(--border);
    font-size: 0.82rem;
  }
  .locked {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: flex-end;
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }
</style>
