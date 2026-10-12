<!-- V102-19 Unlocks the GQL tools for an account signed in with Twitch: paste a GQL token
     (like Chatterino's advanced sign-in, but the name and user ID are the account's), or
     sign in with a code. Either way it has to be the same Twitch account. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { accountsIpc } from '../../ipc/accounts';
  import type { AccountView } from '../../ipc/bindings/AccountView';
  import { toast } from '../../lib/toast.svelte';
  import { signIn } from './signIn.svelte';

  let { account, onclose }: { account: AccountView; onclose?: () => void } = $props();

  let token = $state('');
  let busy = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    try {
      await accountsIpc.addGqlToken(account.userId, token);
      token = '';
      toast.info('GQL tools unlocked.');
      onclose?.();
    } catch (err) {
      toast.error(String(err));
    } finally {
      busy = false;
    }
  }
</script>

<div class="unlock">
  <div class="head">
    <strong>Unlock the GQL tools</strong>
    <span>The miner, following, channel points, GIFs and power-ups.</span>
  </div>
  <div class="way">
    <span>Approve a code on Twitch as {account.displayName}.</span>
    <Button small disabled={signIn.kind !== null} onclick={() => signIn.start('gql', account.userId)}>Sign in with a code</Button>
  </div>
  <form class="way" onsubmit={submit}>
    <input
      class="field mono"
      type="password"
      placeholder="Or paste a GQL token"
      aria-label="GQL token"
      autocomplete="off"
      spellcheck="false"
      bind:value={token}
    />
    <Button type="submit" small disabled={!token.trim() || busy}>{busy ? 'Checking…' : 'Add token'}</Button>
  </form>
  {#if onclose}
    <div class="end"><Button small variant="ghost" onclick={onclose}>Not now</Button></div>
  {/if}
</div>

<style>
  .unlock {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 4px 0 8px;
    padding: 10px 12px;
    background: var(--bg);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius);
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .head span,
  .way span {
    color: var(--muted);
    font-size: 0.82rem;
  }
  .way {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .way span,
  .way input {
    flex: 1;
    min-width: 0;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .end {
    display: flex;
    justify-content: flex-end;
  }
</style>
