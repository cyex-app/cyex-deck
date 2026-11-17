<!-- 1.36 One row each, icon first (V101-05, the 7TV extension's way): created, followers,
     sub status (api.ivr.fi), follow age (only when an account here moderates the channel)
     and earlier names (logs.cyex.app). Each loads on its own. V111-13 Last live with its title
     and the gifter (api.ivr.fi), the chat colour; each row can be turned off in Settings >
     Usercard, and one that is off isn't looked up. -->
<script lang="ts" module>
  import { chatIpc } from '../../ipc/chat';

  /** V107-R26 One channel profile read per channel and session; a failed read is tried again next card. */
  const sublessCache = new Map<string, Promise<boolean>>();
  function subless(room: string): Promise<boolean> {
    let p = sublessCache.get(room);
    if (!p) {
      p = chatIpc.profile(room).then((c) => !!c && !c.affiliate && !c.partner && !c.suspended);
      p.catch(() => sublessCache.delete(room));
      sublessCache.set(room, p);
    }
    return p.catch(() => false);
  }
</script>

<script lang="ts">
  import { uniqueKeys } from '../../lib/keys';
  import Icon from '../../components/Icon.svelte';
  import type { FollowAge } from '../../ipc/bindings/FollowAge';
  import type { NameChange } from '../../ipc/bindings/NameChange';
  import type { Profile } from '../../ipc/bindings/Profile';
  import type { SubStatus } from '../../ipc/bindings/SubStatus';
  import { usercardIpc } from '../../ipc/rules';
  import { formatDate, formatNumber } from '../../lib/format';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { IvrUser } from '../../ipc/bindings/IvrUser';
  import { settings } from '../../lib/settings.svelte';
  import NeedsSignIn from '../../components/NeedsSignIn.svelte';
  import { rawError } from '../../lib/errors';
  import { needsKind } from '../../lib/gate';
  import { copied } from '../../lib/copied.svelte';
  import { chatHex, giftText, lastLiveHint, lastLiveText, lookups } from './facts';

  let { profile, channel, ivr = null, account = null }: { profile: Profile; channel: string; ivr?: IvrUser | null; account?: string | null } = $props();

  const show = $derived(settings.v.chat.usercard);
  // What to ask is settled when the card opens; turning a row on later shows it on the next card.
  // svelte-ignore state_referenced_locally
  const ask = lookups(settings.v.chat.usercard);

  let sub = $state<SubStatus | null | undefined>(undefined);
  /** V107-R26 The channel's own profile: a channel that is neither affiliate nor partner has no subs. */
  let noSubs = $state(false);
  let follow = $state<FollowAge | null>(null);
  /** V111-47 Follow age is read with the Twitch sign-in; a mod with only the extra features gets a line instead. */
  let followNeeds = $state(false);
  let names = $state.raw<NameChange[]>([]);
  const nameKeys = $derived(uniqueKeys(names, (n) => n.login));

  // A card shows one user in one channel for its whole life.
  // svelte-ignore state_referenced_locally
  const { login, id } = profile;
  // svelte-ignore state_referenced_locally
  const room = channel;
  const own = login === room;

  if (!own) {
    if (ask.sub) {
      subless(room).then((v) => (noSubs = v));
      usercardIpc
        .subStatus(login, room)
        .then((s) => (sub = s))
        .catch(() => (sub = null));
    }
    if (ask.followAge) {
      usercardIpc
        .followAge(room, id)
        .then((f) => (follow = f))
        .catch((e) => (followNeeds = needsKind(rawError(e)) === 'helix'));
    }
  }
  if (ask.names) {
    usercardIpc
      .nameHistory(id)
      .then((n) => (names = n.filter((x) => x.login !== login)))
      .catch(() => {});
  }

  const live = $derived(lastLiveText(ivr));
  const hex = $derived(chatHex(profile.color, ivr));
  const cp = copied();

  const TIERS: Record<string, string> = { '1': 'Tier 1', '2': 'Tier 2', '3': 'Tier 3', prime: 'Prime' };
  const day = (rfc: string) => formatDate(new Date(rfc));
  const months = (n: number) => `${n} month${n === 1 ? '' : 's'}`;

  /** "1 year 3 months", from a date to now. */
  function age(rfc: string): string {
    const from = new Date(rfc);
    const now = new Date();
    let m = (now.getFullYear() - from.getFullYear()) * 12 + now.getMonth() - from.getMonth();
    if (now.getDate() < from.getDate()) m -= 1;
    if (m < 1) return 'under a month';
    const y = Math.floor(m / 12);
    const part = (n: number, w: string) => (n ? `${n} ${w}${n === 1 ? '' : 's'}` : '');
    return [part(y, 'year'), part(m % 12, 'month')].filter(Boolean).join(' ');
  }

  const subText = $derived.by(() => {
    if (noSubs) return 'Not affiliate';
    if (sub === undefined) return 'Sub: loading…';
    if (sub === null) return 'Sub: unknown';
    if (sub.hidden) return 'Sub status hidden';
    if (sub.active) {
      const how = sub.tier ? (TIERS[sub.tier] ?? sub.tier) : 'Subbed';
      const gift = giftText(sub, show.gifter);
      return `${how}${gift ? `, ${gift}` : ''} · ${months(sub.months)}`;
    }
    return sub.months ? `Was subbed ${months(sub.months)}` : 'Never subbed';
  });
</script>

<ul>
  {#if show.created}
    <li><Icon name="cake" size={14} />Created {profile.createdAt ? day(profile.createdAt) : '—'}</li>
  {/if}
  {#if show.followers && profile.followers != null}
    <li><Icon name="users" size={14} />{formatNumber(profile.followers)} {profile.followers === 1 ? 'follower' : 'followers'}</li>
  {/if}
  {#if show.lastLive && ask.ivr && live}
    <li class="live">
      <Icon name="tv" size={14} />{#if ivr?.lastTitle}<!-- V111-18 One line; the title is the tip (keyboard too). -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <span class="when" tabindex="0" use:tooltip={{ label: ivr.lastTitle, hint: lastLiveHint(ivr) }}>{live}</span>
      {:else}<span>{live}</span>{/if}
    </li>
  {/if}
  {#if !own}
    {#if show.sub && ask.sub}
      <li class:dim={noSubs || !sub?.active}><Icon name="star" size={14} />{subText}</li>
    {/if}
    {#if show.followAge && followNeeds}
      <li><Icon name="heart" size={14} /><NeedsSignIn kind="helix" {account} text="Follow age needs it." /></li>
    {/if}
    {#if show.followAge && follow?.known}
      <li>
        <Icon name="heart" size={14} />{follow.since ? `Following since ${day(follow.since)} · ${age(follow.since)}` : 'Not following'}
      </li>
    {/if}
  {/if}
  {#if show.chatColor && hex}
    <li>
      <span class="swatch" style:background={hex} aria-hidden="true"></span><span class="hex">Chat color {hex}</span><span class="sr-only" aria-live="polite">{cp.said}</span>
      <button class="cp" aria-label="Copy color" use:tooltip={'Copy color'} onclick={() => cp.run(hex, 'Color', 'hex')}
        ><Icon name={cp.is('hex') ? 'check' : 'copy'} size={11} /></button
      >
    </li>
  {/if}
  {#if show.names && names.length}
    <li>
      <Icon name="history" size={14} /><span
        >Was {#each names as n, i (nameKeys[i])}{i ? ', ' : ''}<span use:tooltip={`${day(n.firstSeen)} – ${day(n.lastSeen)}`}>{n.login}</span
          >{/each}</span
      >
    </li>
  {/if}
</ul>

<style>
  ul {
    display: grid;
    gap: 3px;
    margin: 0;
    padding: 8px 14px;
    list-style: none;
    font-size: 0.82rem;
  }
  li {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }
  li :global(svg) {
    flex: none;
    color: var(--dim);
  }
  li > span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .dim {
    color: var(--muted);
  }
  .swatch {
    flex: none;
    width: 14px;
    height: 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .hex {
    font-variant-numeric: tabular-nums;
  }
  .cp {
    display: grid;
    flex: none;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
  }
  .cp:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
