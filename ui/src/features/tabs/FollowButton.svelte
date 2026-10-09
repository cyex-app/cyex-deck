<!-- 1.11 Follow or unfollow, as the tile's account. Needs its GQL sign-in, so it's hidden
     without one (1.54). Unfollowing takes a second click. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { tabsIpc } from '../../ipc/tabs';
  import { accounts } from '../../lib/accounts.svelte';
  import { toast } from '../../lib/toast.svelte';

  let { channelId, account }: { channelId: string; account: string | null } = $props();

  const CONFIRM_MS = 3000;

  const can = $derived(!!account && accounts.can('follow', account));
  let following = $state<boolean | null>(null);
  let confirming = $state(false);
  let busy = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    following = null;
    if (!can || !account) return;
    let current = true;
    tabsIpc
      .following(channelId, account)
      .then((v) => current && (following = v))
      .catch(() => {});
    return () => (current = false);
  });

  async function click() {
    if (following === null || busy || !account) return;
    if (following && !confirming) {
      confirming = true;
      clearTimeout(timer);
      timer = setTimeout(() => (confirming = false), CONFIRM_MS);
      return;
    }
    confirming = false;
    busy = true;
    const next = !following;
    following = next;
    try {
      await tabsIpc.follow(channelId, account, next);
    } catch (e) {
      following = !next;
      toast.error(String(e));
    }
    busy = false;
  }
</script>

{#if can && following !== null}
  <button
    class="follow"
    class:on={following}
    class:confirming
    aria-label={following ? 'Unfollow' : 'Follow'}
    onclick={click}
    onpointerdown={(e) => e.stopPropagation()}
    use:tooltip={following ? (confirming ? 'Click again to unfollow' : 'Following') : 'Follow'}
  >
    <Icon name="heart" size={15} />
    {#if confirming}<span>Unfollow?</span>{/if}
  </button>
{/if}

<style>
  .follow {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    min-width: 28px;
    justify-content: center;
    padding: 0 6px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 600;
  }
  .follow:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .on {
    color: var(--red);
  }
  .on :global(svg) {
    fill: currentColor;
  }
  .confirming {
    color: var(--red);
    background: var(--surface-hover);
  }
</style>
