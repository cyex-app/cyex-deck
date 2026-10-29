<!-- V108-N23 Above the message box when the account has a watch streak it can share here.
     Share posts Twitch's own streak notice; nothing is sent until the button is pressed. -->
<script module lang="ts">
  import { SvelteSet } from 'svelte/reactivity';

  /** Milestones the user waved off or shared, for this run (any chat). */
  const waved = new SvelteSet<string>();
</script>

<script lang="ts">
  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import type { Streak } from '../../ipc/bindings/Streak';
  import { subsIpc } from '../../ipc/subs';
  import { accounts } from '../../lib/accounts.svelte';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { tabs } from '../tabs/tabs.svelte';
  import { streakText } from './streakShare';

  interface Props {
    account: string | null;
    login: string;
    /** The chat's channel id; the share needs it. */
    channelId: string | null;
  }

  let { account, login, channelId }: Props = $props();

  /** Re-read every minute while the channel is live (the app’s live state), as a fallback
   * to the PubSub topic. A failed read doesn’t stop it. */
  const POLL_MS = 60_000;

  let streak = $state<Streak | null>(null);
  let busy = $state(false);
  let seq = 0;

  const ready = $derived(account !== null && accounts.can('subs', account));
  const milestone = $derived(streak?.milestone && !waved.has(streak.milestone.id) ? streak.milestone : null);

  async function load() {
    const [a, l] = [account, login];
    if (!a || !ready) return;
    const mine = ++seq;
    try {
      const next = await subsIpc.streak(a, l);
      if (mine === seq) streak = next;
    } catch {
      // A failed read leaves what is shown; the next event or the next poll tries again.
    }
  }

  $effect(() => {
    const a = account;
    void login;
    streak = null;
    seq++;
    if (!a || !ready) return;
    void load();
    void subsIpc.watchStreak(a, true).catch(() => {});
    const off = subsIpc.onStreak((e) => {
      if (e.account === a && (!channelId || e.channelId === channelId)) void load();
    });
    const timer = setInterval(() => {
      if (streak?.live || (channelId && tabs.live[channelId]?.stream)) void load();
    }, POLL_MS);
    return () => {
      clearInterval(timer);
      void off.then((f) => f());
      void subsIpc.watchStreak(a, false).catch(() => {});
    };
  });

  async function share() {
    if (!account || !channelId || !milestone || busy) return;
    const m = milestone;
    busy = true;
    try {
      await subsIpc.shareStreak(account, channelId, m.id);
      waved.add(m.id);
      toast.success('Shared in chat.');
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }
</script>

{#if milestone}
  <div class="streak" role="status">
    <span class="text">{streakText(milestone.value, formatNumber)}</span>
    <Button small variant="primary" icon="star" disabled={busy || !channelId} onclick={share}>Share</Button>
    <IconButton icon="x" label="Not now" size={14} onclick={() => waved.add(milestone.id)} />
  </div>
{/if}

<style>
  .streak {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 8px 6px;
    padding: 6px 6px 6px 10px;
    border-left: 3px solid var(--twitch);
    background: color-mix(in srgb, var(--twitch) 8%, var(--surface));
    border-radius: var(--radius);
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-weight: 600;
  }
</style>
