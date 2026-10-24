<!-- The running sign-in (1.5). V108-01 Sign in with Twitch: waits for Twitch's sign-in page.
     Sign in with a code: the code to approve on twitch.tv/activate, which opens with it filled
     in. V108-02 Either opens in the browser or a fresh app window, as picked. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { accountsIpc } from '../../ipc/accounts';
  import { accounts } from '../../lib/accounts.svelte';
  import { formatDuration } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { signIn } from './signIn.svelte';
  import { errorText } from '../../lib/errors';
  import { copy } from '../../lib/clipboard';

  let now = $state(Date.now());

  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  /** V102-19 Adding to an account: the sign-in has to be that account. */
  const as = $derived(signIn.forUser ? accounts.list.find((a) => a.userId === signIn.forUser)?.displayName : undefined);
  const where = $derived(signIn.at === 'app' ? 'the sign-in window' : 'your browser');

  const left = $derived(signIn.code ? signIn.code.expiresIn - (now - signIn.codeAt) / 1000 : 0);

  async function reopen() {
    try {
      await accountsIpc.reopenSignIn();
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  function copyCode() {
    if (signIn.code) void copy(signIn.code.userCode, 'Code');
  }
</script>

<div class="device">
  {#if signIn.kind === 'helix'}
    {#if signIn.opened}
      <p class="lead">Finish signing in in {where}{as ? `, as ${as}` : ''}</p>
      <div class="actions">
        <Button variant="primary" icon="popout" onclick={reopen}>Open again</Button>
        <Button variant="ghost" onclick={() => signIn.cancel()}>Cancel</Button>
      </div>
      <p class="note">Log in there if Twitch asks, then come back here.</p>
    {:else}
      <p class="lead">Opening Twitch…</p>
      <div class="actions">
        <Button variant="ghost" onclick={() => signIn.cancel()}>Cancel</Button>
      </div>
    {/if}
  {:else if signIn.code}
    <p class="lead">Approve this code in {where}{as ? `, as ${as}` : ''}</p>
    <div class="code">{signIn.code.userCode}</div>
    <div class="actions">
      <Button variant="primary" icon="popout" onclick={reopen}>Open again</Button>
      <Button icon="copy" onclick={copyCode}>Copy code</Button>
      <Button variant="ghost" onclick={() => signIn.cancel()}>Cancel</Button>
    </div>
    <p class="note">
      Log in there if Twitch asks. The code works for {formatDuration(left)}.
    </p>
  {:else}
    <p class="lead">Getting a code…</p>
    <div class="actions">
      <Button variant="ghost" onclick={() => signIn.cancel()}>Cancel</Button>
    </div>
  {/if}
</div>

<style>
  .device {
    margin: 4px 0 18px;
    padding: 16px 18px;
    background: var(--surface);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius);
  }
  .lead {
    margin: 0 0 8px;
    font-weight: 600;
  }
  .code {
    font: 700 1.9rem/1.2 var(--font-mono);
    letter-spacing: 0.12em;
    user-select: all;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
  .note {
    margin: 12px 0 0;
    font-size: 0.82rem;
    color: var(--dim);
  }
</style>
