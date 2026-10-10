<!-- 1.49 The usercard's mod section, shown when your account moderates this channel (1.57):
     timeout presets, ban/unban, warn, monitor/restrict, mod/VIP for the broadcaster, plus
     this user's mod history and comments here (M3). Actions run as chat commands. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { ModHistory } from '../../ipc/bindings/ModHistory';
  import type { Profile } from '../../ipc/bindings/Profile';
  import { modIpc } from '../../ipc/moderation';
  import { formatDate } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { TIMEOUTS, mods } from '../modview/moderation.svelte';

  let { profile, channel, account }: { profile: Profile; channel: string; account: string } = $props();

  const role = $derived(mods.role(account, channel));
  let busy = $state(false);
  let history = $state<ModHistory | null>(null);
  let comment = $state('');
  let shareable = $state(false);

  // Mod history (GQL) for this user here. Best-effort; silent if unavailable.
  $effect(() => {
    history = null;
    modIpc
      .history(account, channel, profile.id)
      .then((h) => (history = h))
      .catch(() => {});
  });

  async function run(name: string, args: string, after?: () => void) {
    busy = true;
    try {
      const line = await modIpc.run(account, channel, name, args);
      if (line) toast.info(line);
      after?.();
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  async function addComment() {
    const text = comment.trim();
    if (!text) return;
    busy = true;
    try {
      await modIpc.addComment(account, channel, profile.id, text, shareable);
      comment = '';
      history = await modIpc.history(account, channel, profile.id);
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  async function delComment(id: string) {
    try {
      await modIpc.deleteComment(account, channel, id);
      if (history) history = { ...history, comments: history.comments.filter((c) => c.id !== id) };
    } catch (e) {
      toast.error(String(e));
    }
  }
</script>

{#if role}
  <section class="mod">
    <div class="line">
      <Button small disabled={busy} onclick={() => run('unban', profile.login)}>Unban</Button>
      {#each TIMEOUTS as t (t.secs)}
        <button class="to" disabled={busy} onclick={() => run('timeout', `${profile.login} ${t.secs}`)}>{t.label}</button>
      {/each}
      <Button small variant="danger" disabled={busy} onclick={() => run('ban', profile.login)}>Ban</Button>
    </div>
    <div class="line">
      <Button small variant="ghost" disabled={busy} onclick={() => run('monitor', profile.login)}>Monitor</Button>
      <Button small variant="ghost" disabled={busy} onclick={() => run('restrict', profile.login)}>Restrict</Button>
      {#if role === 'broadcaster'}
        <Button small variant="ghost" disabled={busy} onclick={() => run('mod', profile.login)}>Mod</Button>
        <Button small variant="ghost" disabled={busy} onclick={() => run('vip', profile.login)}>VIP</Button>
      {/if}
    </div>

    {#if history}
      <div class="stats">
        <span>{history.messageCount} msgs</span>
        <span>{history.bans} bans</span>
        <span>{history.timeouts} timeouts</span>
        <span>{history.warnings} warnings</span>
      </div>
      {#each history.comments as c (c.id)}
        <div class="comment">
          <div class="meta">
            {c.author} · {c.at ? formatDate(new Date(c.at)) : ''}{c.shared ? ' · shared' : ''}
            {#if !c.shared}<button class="del" onclick={() => delComment(c.id)} aria-label="Delete">×</button>{/if}
          </div>
          <div class="text">{c.text}</div>
        </div>
      {/each}
      <div class="add">
        <input placeholder="Mod comment" maxlength="500" bind:value={comment} />
        <label class="share"><input type="checkbox" bind:checked={shareable} /> Share</label>
        <Button small disabled={busy || !comment.trim()} onclick={addComment}>Add</Button>
      </div>
    {/if}
  </section>
{/if}

<style>
  .mod {
    margin: 10px 0;
    padding: 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 6px;
    align-items: center;
  }
  .to {
    padding: 2px 7px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.78rem;
    cursor: pointer;
  }
  .to:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .stats {
    display: flex;
    gap: 10px;
    margin: 6px 0;
    font-size: 0.78rem;
    color: var(--muted);
  }
  .comment {
    margin: 4px 0;
    font-size: 0.82rem;
  }
  .meta {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--muted);
    font-size: 0.74rem;
  }
  .del {
    background: none;
    border: 0;
    color: var(--muted);
    cursor: pointer;
  }
  .add {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-top: 6px;
  }
  .add input:not([type='checkbox']) {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.8rem;
  }
  .share {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 0.76rem;
    color: var(--muted);
  }
</style>
