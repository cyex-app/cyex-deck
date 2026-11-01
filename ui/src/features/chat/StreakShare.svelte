<!-- V108-N23 A slim row above the message box, styled like the reply row, when the account has a
     watch streak it can share here. Share posts Twitch's own streak notice; nothing is sent
     until the button is pressed. The x or Escape hides it for the run. -->
<script module lang="ts">
  import { SvelteSet } from 'svelte/reactivity';

  /** Milestones the user waved off or shared, for this run (any chat). */
  const waved = new SvelteSet<string>();
</script>

<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Streak } from '../../ipc/bindings/Streak';
  import { subsIpc } from '../../ipc/subs';
  import { accounts } from '../../lib/accounts.svelte';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { tabs } from '../tabs/tabs.svelte';
  import { shown, streakShort, streakText } from './streakShare';

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
  let row = $state<HTMLElement>();
  let seq = 0;

  const ready = $derived(account !== null && accounts.can('subs', account));
  /** Who the shown answer was asked for. */
  let asked = $state({ account: null as string | null, login: '' });
  const milestone = $derived(
    shown(streak, { ...asked, channelId: null }, { account, login, channelId }, (id) => waved.has(id)),
  );

  async function load() {
    const [a, l] = [account, login];
    if (!a || !ready) return;
    const mine = ++seq;
    try {
      const next = await subsIpc.streak(a, l);
      if (mine === seq) {
        asked = { account: a, login: l };
        streak = next;
      }
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
    // The channel Twitch answered for (the chat's own id can still be loading).
    const channel = channelId ?? streak?.channelId;
    if (!account || !channel || !milestone || busy) return;
    const m = milestone;
    busy = true;
    try {
      await subsIpc.shareStreak(account, channel, m.id);
      handBack(() => waved.add(m.id));
      toast.success('Shared in chat.');
    } catch (e) {
      // Never a dead button: a streak that didn't share stays hidden this run.
      handBack(() => waved.add(m.id));
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  function dismiss() {
    const m = milestone;
    if (m) handBack(() => waved.add(m.id));
  }

  /** Hides the row; focus inside it goes to the message box, not the page. */
  function handBack(hide: () => void) {
    const box = row?.contains(document.activeElement) ? row.closest('.input')?.querySelector('textarea') : null;
    hide();
    box?.focus();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      dismiss();
    }
  }
</script>

{#if milestone}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="streak" role="group" aria-label="Watch streak" bind:this={row} {onkeydown}>
    <Icon name="zap" size={13} />
    <span class="text"><span class="long">{streakText(milestone.value, formatNumber)}</span><span class="short">{streakShort(milestone.value, formatNumber)}</span></span>
    <button
      class="share"
      disabled={busy}
      aria-busy={busy}
      use:tooltip={{ label: 'Share in chat', hint: 'Twitch posts it in this chat for everyone.' }}
      onclick={share}>{busy ? 'Sharing' : 'Share'}</button
    >
    <button class="close" aria-label="Not now" use:tooltip={'Not now'} onclick={dismiss}><Icon name="x" size={13} /></button>
  </div>
{/if}

<style>
  /* Same row as the reply bar (ChatInput .reply-bar): no fill, dim text, 22px controls. */
  .streak {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
    color: var(--dim);
    font-size: 0.8rem;
    animation: in var(--base) ease-out;
    container-type: inline-size;
  }
  .short {
    display: none;
  }
  /* Narrow chat: "5-stream streak". */
  @container (max-width: 279px) {
    .long {
      display: none;
    }
    .short {
      display: inline;
    }
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .share {
    flex: none;
    height: 22px;
    padding: 0 8px;
    background: none;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--text);
    font: inherit;
    font-weight: 600;
    transition:
      background var(--fast),
      border-color var(--fast);
  }
  .share:hover:not(:disabled) {
    background: var(--surface);
    border-color: var(--accent-border);
  }
  .share:active:not(:disabled) {
    opacity: 0.6;
    transition: none;
  }
  .share:disabled {
    color: var(--dim);
  }
  .close {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 22px;
    height: 22px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .close::before {
    content: "";
    position: absolute;
    inset: -1px;
  }
  .close:hover {
    color: var(--text);
  }
  .close:active {
    opacity: 0.6;
    transition: none;
  }
  @keyframes in {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .streak {
      animation: none;
    }
  }
</style>
