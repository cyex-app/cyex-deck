<!-- Account manager (1.3, 1.23) and the three ways to add an account (V101-08): Twitch's
     sign-in (1.5), a code to approve on Twitch, or a token you have (1.4). The last two
     unlock the GQL tools (1.54). -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Icon from '../../../components/Icon.svelte';
  import Row from '../../../components/Row.svelte';
  import AccountCard from '../../accounts/AccountCard.svelte';
  import ClientIdSetup from '../../accounts/ClientIdSetup.svelte';
  import DeviceCode from '../../accounts/DeviceCode.svelte';
  import TokenForm from '../../accounts/TokenForm.svelte';
  import { signIn } from '../../accounts/signIn.svelte';
  import { accounts } from '../../../lib/accounts.svelte';

  let changingClientId = $state(false);
  let manual = $state(false);
</script>

<h2>Accounts</h2>

{#if !accounts.hasClientId}<ClientIdSetup />{/if}
{#if signIn.kind}<DeviceCode />{/if}

{#each accounts.list as account (account.userId)}
  <AccountCard {account} />
{:else}
  <p class="note">No accounts yet. You can watch streams and read chat without one.</p>
{/each}

<h3 class="label">{accounts.list.length ? 'Add another account' : 'Add an account'}</h3>
<div class="ways">
  <div class="way">
    <Icon name="login" size={18} />
    <div class="text">
      <strong>Sign in with Twitch</strong>
      <span>The usual Twitch sign-in. Chat, moderation and who you follow.</span>
    </div>
    <Button variant="primary" small disabled={!accounts.hasClientId || signIn.kind !== null} onclick={() => signIn.start('helix')}>
      Sign in
    </Button>
  </div>
  <div class="way">
    <Icon name="key" size={18} />
    <div class="text">
      <strong>Sign in with a code</strong>
      <span>Approve a short code on Twitch. Chat, the miner, following, channel points, GIFs and power-ups; no mod tools.</span>
    </div>
    <Button small disabled={signIn.kind !== null} onclick={() => signIn.start('gql')}>Get a code</Button>
  </div>
  <div class="way">
    <Icon name="pencil" size={18} />
    <div class="text">
      <strong>Use a token you have</strong>
      <span>From Chatterino or another app. Tokens from Twitch's own apps unlock the same tools as a code.</span>
    </div>
    <Button small variant={manual ? 'ghost' : undefined} onclick={() => (manual = !manual)}>{manual ? 'Close' : 'Enter token'}</Button>
  </div>
  {#if manual}<TokenForm ondone={() => (manual = false)} />{/if}
</div>

{#if accounts.hasClientId}
  {#if changingClientId}
    <ClientIdSetup onsaved={() => (changingClientId = false)} />
  {:else}
    <Row label="Twitch client ID" hint="For Sign in with Twitch. Saved on this PC. Change it if Twitch doesn't recognize it.">
      <Button small onclick={() => (changingClientId = true)}>Change</Button>
    </Row>
  {/if}
{/if}

<style>
  .note {
    margin: 4px 0 14px;
    color: var(--dim);
  }
  h3 {
    margin: 24px 0 8px;
  }
  .ways {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 18px;
  }
  .way {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .way > :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .text span {
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
