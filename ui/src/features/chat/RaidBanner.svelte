<!-- V110-U43 A raid counting down from the channel this chat shows, a card in the pinned
     message's slot and look: the target's avatar, "Raiding <name>" (opens their usercard), their
     stream title and category when known with how many viewers go along (live from Twitch, about
     once a second), the time left and a bar along the bottom edge that drains over the countdown
     (one compositor animation; under reduced motion a static bar updated each second). It goes
     when the raid goes or is called off; Rust adds "Raided X with N viewers" to the chat.
     A signed-in chat of another channel always has a "Join" tick, unticked for every new raid.
     Ticking joins through Twitch; unticking before it goes only drops the pop-up (Twitch has no
     leave call we know of). When the ticked raid goes, Rust tells the main window, which opens the
     raided channel in a pop-out (features/tabs/raidPop.ts). Nothing joins without the tick. In
     your own channel you are the one raiding: no tick, and "Cancel raid" (asks first). -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import { hideTip, tooltip } from '../../components/tooltip.svelte';
  import { chatIpc } from '../../ipc/chat';
  import { raidIpc } from '../../ipc/raid';
  import { smIpc } from '../../ipc/streammanager';
  import { tabsIpc } from '../../ipc/tabs';
  import { nowSecond } from '../../lib/clock.svelte';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { reduced } from '../../lib/motion';
  import { toast } from '../../lib/toast.svelte';
  import { cancelShows, ownChannel, raidAbout, raidBar, raidFrac, raidLeft, tickShows, viewersText } from './raidBanner';
  import { raids } from './raids.svelte';

  interface Props {
    /** The chat's channel login, for the usercard. */
    channel: string;
    channelId: string | null;
    /** The chat's account (user id); signed out: no Join. */
    account?: string | null;
  }

  let { channel, channelId, account = null }: Props = $props();

  $effect(() => {
    const id = channelId;
    if (!id) return;
    return raids.follow(id);
  });

  const raid = $derived(raids.of(channelId));
  const left = $derived(raid ? raidLeft(raid.goesAt, nowSecond()) : '');

  let busy = $state(false);
  /** The tick while a join or leave is on its way, so a failed call puts it back. */
  let pending = $state<boolean | null>(null);
  const own = $derived(ownChannel(account, channelId));
  const canTick = $derived(tickShows(raid?.id ?? null, account, own));
  const ticked = $derived(pending ?? raids.ticked(raid?.id ?? null));

  // The account that runs the stream manager, asked only in your own channel during a raid.
  let managerId = $state<string | null>(null);
  $effect(() => {
    if (!own || !raid) return;
    let gone = false;
    smIpc
      .manager()
      .then((m) => !gone && (managerId = m?.accountId ?? null))
      .catch(() => {});
    return () => {
      gone = true;
    };
  });
  const canCancel = $derived(cancelShows(raid?.id ?? null, own, managerId, channelId));
  let canceling = $state(false);

  async function cancel() {
    if (canceling) return;
    canceling = true;
    try {
      await smIpc.unraid();
      toast.success('Raid canceled.');
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      canceling = false;
    }
  }

  // The target's stream title and category, when Twitch lists them; looked up once per target.
  let about = $state('');
  $effect(() => {
    const login = raid?.targetLogin;
    if (!login) {
      about = '';
      return;
    }
    let gone = false;
    tabsIpc
      .lookup(login)
      .then((list) => {
        if (gone) return;
        const s = list.find((c) => c.login === login)?.stream;
        about = raidAbout(s?.title, s?.game);
      })
      .catch(() => {
        if (!gone) about = '';
      });
    return () => {
      gone = true;
    };
  });

  // Reduce motion can flip while a raid shows.
  let still = $state(reduced());
  $effect(() => {
    if (typeof matchMedia !== 'function') return;
    const q = matchMedia('(prefers-reduced-motion: reduce)');
    const flip = () => (still = q.matches);
    q.addEventListener('change', flip);
    return () => q.removeEventListener('change', flip);
  });
  /** The bar from the real start and go times. Read when the times change (the key below
   * restarts the animation), not every second. */
  const timeKey = $derived(raid ? `${raid.id}:${raid.createdAt}:${raid.goesAt}` : '');
  const bar = $derived.by(() => {
    timeKey;
    return untrack(() => (raid ? raidBar(raid.createdAt, raid.goesAt, Date.now()) : null));
  });
  /** Reduced motion: the bar's level, moved by the clock. */
  const level = $derived(raid && still ? raidFrac(raid.createdAt, raid.goesAt, nowSecond()) : 1);

  function usercard(login: string) {
    hideTip();
    chatIpc.openUsercard(channel, login).catch((err) => toast.error(errorText(err)));
  }

  function cardKey(e: KeyboardEvent, login: string) {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    e.preventDefault();
    usercard(login);
  }

  /** Ticking joins the raid on Twitch; unticking before it goes only drops the pop-up. */
  async function tick(on: boolean) {
    const r = raid;
    const who = account;
    if (!r || !who || busy) return;
    busy = true;
    pending = on;
    try {
      if (on) await raidIpc.join(who, r.id);
      else await raidIpc.leave(r.id);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
      pending = null;
    }
  }
</script>

{#if raid}
  <div class="wrap">
    <div class="raid" role="status" aria-label="Raiding {raid.targetName}, {viewersText(raid.viewers, formatNumber)}">
      <span class="av" aria-hidden="true"><Avatar src={raid.targetAvatar} name={raid.targetName} size={32} /></span>
      <div class="body">
        <span class="l1">
          <span class="dim">Raiding</span>
          <span
            class="name"
            role="link"
            tabindex="0"
            aria-label="{raid.targetName}, open usercard"
            use:tooltip={{ label: raid.targetName, hint: 'Opens their usercard.' }}
            onclick={() => usercard(raid.targetLogin)}
            onkeydown={(e) => cardKey(e, raid.targetLogin)}>{raid.targetName}</span
          >
                  </span>
        <span class="l2">
          {#if about}<span class="about" use:tooltip={about}>{about}</span><span class="sep" aria-hidden="true">·</span>{/if}
          <span class="going">{formatNumber(raid.viewers)} {raid.viewers === 1 ? 'viewer' : 'viewers'} going</span>
        </span>
      </div>
      <div class="end">
        <!-- V110-F23 Not read out: it changes every second. -->
        <span class="left" aria-hidden="true">{left}</span>
        {#if canTick}
          <span class="tick"><Checkbox label="Join" hint="Go along when the raid goes" checked={ticked} disabled={busy} onchange={tick} /></span>
        {:else if canCancel}
          <ConfirmButton label="Cancel raid" confirm="Click to cancel the raid" variant="danger" disabled={canceling} onconfirm={cancel} />
        {/if}
      </div>
      {#if bar}
        {#key `${timeKey}:${still}`}
          <div
            class="bar"
            class:run={!still}
            style:--from={bar.from}
            style:--dur="{bar.ms}ms"
            style:--delay="{bar.delay}ms"
            style:transform={still ? `scaleX(${level})` : undefined}
            aria-hidden="true"
          ></div>
        {/key}
      {/if}
    </div>
  </div>
{/if}

<style>
  .wrap {
    container-type: inline-size;
    margin: 6px 8px 2px;
  }
  .raid {
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px 10px;
    min-height: 48px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: calc(var(--chat-size) * 0.9);
    line-height: 1.3;
  }
  .av {
    flex: none;
    display: flex;
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .l1,
  .l2 {
    display: flex;
    align-items: baseline;
    gap: 4px;
    min-width: 0;
    white-space: nowrap;
  }
  .l2 {
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .dim {
    color: var(--dim);
  }
  .name {
    color: var(--text);
    font-weight: 700;
    cursor: pointer;
    border-radius: var(--radius-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .name:hover {
    text-decoration: underline;
  }
  .name:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  .about {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .going,
  .sep {
    flex: none;
    font-variant-numeric: tabular-nums;
  }
  .end {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 3px;
  }
  .left {
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-hint);
    font-variant-numeric: tabular-nums;
    min-width: 3ch;
    text-align: right;
    white-space: nowrap;
  }
  .tick {
    color: var(--muted);
    font-size: var(--text-hint);
    line-height: 1;
  }
  /* Drains over the countdown on the compositor: one animation, set from the remaining time. */
  .bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    /* Readable on either theme, whatever the accent. */
    background: var(--accent-text);
    transform-origin: left center;
    transform: scaleX(var(--from, 1));
  }
  .bar.run {
    will-change: transform;
    animation: drain var(--dur) var(--delay) linear forwards;
  }
  @keyframes drain {
    from {
      transform: scaleX(var(--from, 1));
    }
    to {
      transform: scaleX(0);
    }
  }
  /* Narrow chats: one line, the viewers beside the name. */
  @container (max-width: 320px) {
    .raid {
      padding: 6px 8px 9px;
      min-height: 0;
      gap: 8px;
    }
    /* No room for the count beside the name; it stays in the screen reader label. */
    .l2 {
      display: none;
    }
    .end {
      flex-direction: row-reverse;
      align-items: center;
      gap: 8px;
    }
  }
</style>
