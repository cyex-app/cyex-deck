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
  import { formatNumber } from '../../lib/format';
  import { subsIpc } from '../../ipc/subs';
  import type { AccountView } from '../../ipc/bindings/AccountView';
  import type { TokenKind } from '../../ipc/bindings/TokenKind';
  import type { TokenView } from '../../ipc/bindings/TokenView';
  import { accounts } from '../../lib/accounts.svelte';
  import { toast } from '../../lib/toast.svelte';
  import GqlUnlock from './GqlUnlock.svelte';
  import ProfileEditor from './ProfileEditor.svelte';
  import { signIn } from './signIn.svelte';
  import { accountName, missingScopesText, needsSignIn, tokenStatus } from './status';

  let { account }: { account: AccountView } = $props();

  const TOKENS: { kind: TokenKind; title: string; none: string }[] = [
    { kind: 'helix', title: 'Twitch', none: 'Needed for payment methods, subscriptions and mod tools.' },
    { kind: 'gql', title: 'GQL tools', none: 'Not added. Needed for the miner, following, channel points, GIFs and power-ups.' },
  ];

  const conn = $derived(accounts.connectionsOf(account.userId));
  const bothTokens = $derived(account.helix !== null && account.gql !== null);
  /** V102-19 The GQL options: asked for, or offered right after Sign in with Twitch. */
  let unlocking = $state(false);
  const showUnlock = $derived(
    (unlocking || signIn.offer === account.userId) && (account.gql === null || needsSignIn(account.gql)),
  );

  let armed = $state(false);
  let editing = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function tokenOf(kind: TokenKind): TokenView | null {
    return kind === 'helix' ? account.helix : account.gql;
  }

  async function attempt(action: () => Promise<void>) {
    try {
      await action();
    } catch (e) {
      toast.error(String(e));
    }
  }

  function removeAccount() {
    if (!armed) {
      armed = true;
      timer = setTimeout(() => (armed = false), 3000);
      return;
    }
    clearTimeout(timer);
    armed = false;
    void attempt(() => accountsIpc.remove(account.userId));
  }

  /** Twitch's sign-in runs right away; GQL shows its two options first. */
  function add(kind: TokenKind) {
    if (kind === 'helix') void signIn.start('helix', account.userId);
    else unlocking = true;
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
      <span class="name">{t.title}</span>
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

  {#if account.helix && !needsSignIn(account.helix) && account.missingScopes.length}
    <p class="warn" use:tooltip={{ label: account.missingScopes.join(', ') }}>
      <Icon name="alert" size={14} />
      {missingScopesText(account.missingScopes.length)}
    </p>
  {/if}

  <div class="line">
    <span class="name">Joined chats</span>
    <span class="status">
      {formatNumber(conn?.joined ?? 0)}
      {#if conn?.refused.length}
        <span class="warn">· Twitch refused {conn.refused.map((l) => `#${l}`).join(', ')}</span>
      {/if}
    </span>
  </div>

  {#if editing}<ProfileEditor account={account.userId} />{/if}

  <footer>
    {#if accounts.can('displayName', account.userId)}
      <Button small variant="ghost" icon="pencil" onclick={() => (editing = !editing)}>
        {editing ? 'Close profile' : 'Twitch profile'}
      </Button>
    {:else}
      <!-- V102-20 -->
      <span class="locked"><Button small variant="ghost" icon="pencil" disabled>Twitch profile</Button><GateFlag kind="gql" /></span>
    {/if}
    <Button
      small
      variant="ghost"
      onclick={() =>
        attempt(async () => {
          await subsIpc.signOut(account.userId);
          toast.info('Signed out of Twitch pages in the app.');
        })}
    >
      Sign out of Twitch pages
    </Button>
    <Button small variant={armed ? 'danger' : 'ghost'} icon="trash" onclick={removeAccount}>
      {armed ? 'Click again to remove' : 'Remove account'}
    </Button>
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
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    border-top: 1px solid var(--border);
  }
  .name {
    flex: none;
    width: 96px;
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
