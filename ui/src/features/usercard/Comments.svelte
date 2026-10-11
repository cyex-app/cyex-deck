<!-- M3 Mod comments on this user, yours and those shared from other channels, with a box
     at the bottom for a new one (V101-05, the 7TV extension's comments tab). -->
<script lang="ts">
  import type { ModComment } from '../../ipc/bindings/ModComment';
  import { modIpc } from '../../ipc/moderation';
  import { formatDate } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';

  interface Props {
    comments: ModComment[];
    channel: string;
    account: string;
    targetId: string;
    /** After adding or deleting one, so the card reads them again. */
    onchange: () => void;
  }

  let { comments, channel, account, targetId, onchange }: Props = $props();

  let text = $state('');
  let shareable = $state(false);
  let busy = $state(false);
  let box = $state<HTMLElement>();
  const oldest = $derived([...comments].sort((a, b) => a.at.localeCompare(b.at)));

  $effect(() => {
    void oldest.length;
    if (box) box.scrollTop = box.scrollHeight;
  });

  async function add() {
    const t = text.trim();
    if (!t) return;
    busy = true;
    try {
      await modIpc.addComment(account, channel, targetId, t, shareable);
      text = '';
      onchange();
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  async function remove(id: string) {
    try {
      await modIpc.deleteComment(account, channel, id);
      onchange();
    } catch (e) {
      toast.error(String(e));
    }
  }
</script>

<div class="list" bind:this={box}>
  {#each oldest as c (c.id)}
    <div class="comment">
      <div class="meta">
        {c.author} · {c.at ? formatDate(new Date(c.at)) : ''}{c.shared ? ' · shared' : ''}
        {#if !c.shared}<button class="del" onclick={() => remove(c.id)} aria-label="Delete comment">×</button>{/if}
      </div>
      <div class="text">{c.text}</div>
    </div>
  {:else}
    <p class="empty">No mod comments yet.</p>
  {/each}
</div>
<form
  class="add"
  onsubmit={(e) => {
    e.preventDefault();
    void add();
  }}
>
  <input class="field" placeholder="Add a comment" maxlength="500" bind:value={text} disabled={busy} />
  <label class="share"><input type="checkbox" bind:checked={shareable} /> Share</label>
</form>

<style>
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 0;
  }
  .comment {
    padding: 4px 14px;
    font-size: 0.82rem;
  }
  .meta {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--dim);
    font-size: 0.72rem;
  }
  .del {
    margin-left: auto;
    padding: 0 4px;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .del:hover {
    color: var(--red);
  }
  .text {
    overflow-wrap: anywhere;
  }
  .empty {
    margin: 0;
    padding: 40px 14px 0;
    color: var(--dim);
    font-size: 0.82rem;
    text-align: center;
  }
  .add {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 8px;
    border-top: 1px solid var(--border);
  }
  .add .field {
    flex: 1;
    min-width: 0;
  }
  .share {
    display: flex;
    gap: 4px;
    align-items: center;
    color: var(--muted);
    font-size: 0.78rem;
  }
</style>
