<!-- 1.36 One row each, icon first (V101-05, the 7TV extension's way): created, followers,
     sub status (api.ivr.fi), follow age (only when an account here moderates the channel)
     and earlier names (logs.cyex.app). Each loads on its own. -->
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

  let { profile, channel }: { profile: Profile; channel: string } = $props();

  let sub = $state<SubStatus | null | undefined>(undefined);
  let follow = $state<FollowAge | null>(null);
  let names = $state.raw<NameChange[]>([]);
  const nameKeys = $derived(uniqueKeys(names, (n) => n.login));

  // A card shows one user in one channel for its whole life.
  // svelte-ignore state_referenced_locally
  const { login, id } = profile;
  // svelte-ignore state_referenced_locally
  const room = channel;
  const own = login === room;

  if (!own) {
    usercardIpc
      .subStatus(login, room)
      .then((s) => (sub = s))
      .catch(() => (sub = null));
    usercardIpc
      .followAge(room, id)
      .then((f) => (follow = f))
      .catch(() => {});
  }
  usercardIpc
    .nameHistory(id)
    .then((n) => (names = n.filter((x) => x.login !== login)))
    .catch(() => {});

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
    if (sub === undefined) return 'Sub: loading…';
    if (sub === null) return 'Sub: unknown';
    if (sub.hidden) return 'Sub status hidden';
    if (sub.active) {
      const how = sub.tier ? (TIERS[sub.tier] ?? sub.tier) : 'Subbed';
      return `${how}${sub.gift ? ', gifted' : ''} · ${months(sub.months)}`;
    }
    return sub.months ? `Was subbed ${months(sub.months)}` : 'Never subbed';
  });
</script>

<ul>
  <li><Icon name="cake" size={14} />Created {profile.createdAt ? day(profile.createdAt) : '—'}</li>
  {#if profile.followers != null}
    <li><Icon name="users" size={14} />{formatNumber(profile.followers)} {profile.followers === 1 ? 'follower' : 'followers'}</li>
  {/if}
  {#if !own}
    <li class:dim={!sub?.active}><Icon name="star" size={14} />{subText}</li>
    {#if follow?.known}
      <li>
        <Icon name="heart" size={14} />{follow.since ? `Following since ${day(follow.since)} · ${age(follow.since)}` : 'Not following'}
      </li>
    {/if}
  {/if}
  {#if names.length}
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
</style>
