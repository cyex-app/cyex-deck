<!-- Account manager (1.3, 1.4, 1.5, 1.23). -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import AccountCard from '../../accounts/AccountCard.svelte';
  import ClientIdSetup from '../../accounts/ClientIdSetup.svelte';
  import DeviceCode from '../../accounts/DeviceCode.svelte';
  import PasteToken from '../../accounts/PasteToken.svelte';
  import { signIn } from '../../accounts/signIn.svelte';
  import { accounts } from '../../../lib/accounts.svelte';

  let changingClientId = $state(false);
</script>

<h2>Accounts</h2>

{#if !accounts.hasClientId}<ClientIdSetup />{/if}
{#if signIn.kind}<DeviceCode />{/if}

{#each accounts.list as account (account.userId)}
  <AccountCard {account} />
{:else}
  <p class="note">No accounts yet. You can watch streams and read chat without one.</p>
{/each}

<div class="actions">
  <Button
    variant="primary"
    icon="login"
    disabled={!accounts.hasClientId || signIn.kind !== null}
    onclick={() => signIn.start('helix')}
  >
    {accounts.list.length ? 'Add another account' : 'Sign in with Twitch'}
  </Button>
</div>

<h3 class="label">Other ways to sign in</h3>
<Row
  label="GQL tools only"
  hint="Signs in through Twitch's Android TV app. Gives chat, the miner, following, channel points, GIFs and power-ups, but no mod tools."
>
  <Button small icon="key" disabled={signIn.kind !== null} onclick={() => signIn.start('gql')}>Sign in</Button>
</Row>
<PasteToken />
{#if accounts.hasClientId}
  {#if changingClientId}
    <ClientIdSetup onsaved={() => (changingClientId = false)} />
  {:else}
    <Row label="Twitch client ID" hint="Saved on this PC. Change it if Twitch doesn't recognize it.">
      <Button small onclick={() => (changingClientId = true)}>Change</Button>
    </Row>
  {/if}
{/if}

<style>
  .note {
    margin: 4px 0 14px;
    color: var(--dim);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin: 6px 0 0;
  }
  h3 {
    margin: 28px 0 4px;
  }
</style>
