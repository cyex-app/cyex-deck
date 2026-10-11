<!-- Code to approve on twitch.tv/activate while a Device Code sign-in runs (1.5). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { systemIpc } from '../../ipc/system';
  import { accounts } from '../../lib/accounts.svelte';
  import { formatDuration } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { signIn } from './signIn.svelte';

  let now = $state(Date.now());

  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  /** V102-19 Adding to an account: the code has to be approved as that account. */
  const as = $derived(signIn.forUser ? accounts.list.find((a) => a.userId === signIn.forUser)?.displayName : undefined);

  const left = $derived(signIn.code ? signIn.code.expiresIn - (now - signIn.codeAt) / 1000 : 0);

  async function open() {
    if (!signIn.code) return;
    try {
      await systemIpc.openLink(signIn.code.uri);
    } catch (e) {
      toast.error(String(e));
    }
  }

  async function copy() {
    if (!signIn.code) return;
    await navigator.clipboard.writeText(signIn.code.userCode);
    toast.info('Code copied.');
  }
</script>

<div class="device">
  {#if signIn.code}
    <p class="lead">Enter this code on Twitch{as ? `, signed in as ${as}` : ''}</p>
    <div class="code">{signIn.code.userCode}</div>
    <div class="actions">
      <Button variant="primary" icon="popout" onclick={open}>Open twitch.tv/activate</Button>
      <Button icon="copy" onclick={copy}>Copy code</Button>
      <Button variant="ghost" onclick={() => signIn.cancel()}>Cancel</Button>
    </div>
    <p class="note">
      Waiting for you to approve. The code works for {formatDuration(left)}.
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
