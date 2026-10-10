<!-- 1.48, 1.55 Mass actions: paste a list of logins and ban or time them out. Helix bans
     one at a time (slow); a GQL sign-in batches 35 at a time (fast), so Helix-only users
     get a hint. M1 /nuke lives on the chat input; here is the bulk-list tool for
     followbots and raids. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { MassbanProgress } from '../../ipc/bindings/MassbanProgress';
  import { modIpc } from '../../ipc/moderation';
  import { accounts } from '../../lib/accounts.svelte';
  import { toast } from '../../lib/toast.svelte';

  let { account, login, channelId }: { account: string; login: string; channelId: string | null } = $props();

  let text = $state('');
  let reason = $state('');
  let minutes = $state(0);
  let running = $state(false);
  let progress = $state<MassbanProgress | null>(null);

  // A GQL sign-in batches; Helix-only bans one at a time.
  const hasGql = $derived(accounts.list.find((a) => a.userId === account)?.features.includes('points') ?? false);

  const logins = $derived(
    text
      .split(/[\s,]+/)
      .map((s) => s.trim().replace(/^@/, '').toLowerCase())
      .filter((s) => /^[a-z0-9_]{1,25}$/.test(s)),
  );

  $effect(() => {
    if (!channelId) return;
    let stop: (() => void) | undefined;
    let gone = false;
    void modIpc
      .onMassban((p) => {
        if (p.channelId === channelId) {
          progress = p;
          running = p.running;
        }
      })
      .then((u) => (gone ? u() : (stop = u)));
    return () => {
      gone = true;
      stop?.();
    };
  });

  function go() {
    if (!logins.length) return;
    const secs = minutes > 0 ? minutes * 60 : null;
    running = true;
    modIpc
      .massban(account, login, logins, secs, reason)
      .then((n) => toast.info(`Done: ${n} of ${logins.length}.`))
      .catch((e) => toast.error(String(e)))
      .finally(() => (running = false));
  }

  function stop() {
    if (channelId) void modIpc.stopMassban(channelId);
  }
</script>

<div class="mass">
  <textarea placeholder="Logins, separated by spaces, commas or lines" bind:value={text} rows={4}></textarea>
  <div class="opts">
    <label use:tooltip={'0 bans; above 0 times out for that many minutes'}>
      Timeout
      <input type="number" min="0" max="20160" bind:value={minutes} /> min
    </label>
  </div>
  <input class="reason" placeholder="Reason (optional)" bind:value={reason} />
  <div class="go">
    {#if running}
      <Button small variant="danger" onclick={stop}>Stop</Button>
    {:else}
      <Button small disabled={logins.length === 0} onclick={go}>
        {minutes > 0 ? 'Time out' : 'Ban'} {logins.length || ''}
      </Button>
    {/if}
    {#if progress}
      <span class="prog">{progress.done}/{progress.total}{progress.refused ? ` · ${progress.refused} refused` : ''}</span>
    {/if}
  </div>
  {#if !hasGql}
    <p class="hint">Add a GQL sign-in to this account in Settings → Accounts to ban far faster.</p>
  {/if}
</div>

<style>
  .mass {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  textarea,
  .reason {
    width: 100%;
    padding: 6px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.82rem;
    resize: vertical;
  }
  .opts label {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.82rem;
    color: var(--muted);
  }
  .opts input {
    width: 64px;
    padding: 3px 6px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
  }
  .go {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .prog {
    font-size: 0.8rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .hint {
    margin: 0;
    font-size: 0.76rem;
    color: var(--muted);
  }
</style>
