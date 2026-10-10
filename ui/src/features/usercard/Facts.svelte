<!-- 1.36 Sub status (api.ivr.fi), follow age (only when an account here moderates the
     channel) and earlier names (logs.cyex.app). Each loads on its own. -->
<script lang="ts">
  import type { FollowAge } from '../../ipc/bindings/FollowAge';
  import type { NameChange } from '../../ipc/bindings/NameChange';
  import type { Profile } from '../../ipc/bindings/Profile';
  import type { SubStatus } from '../../ipc/bindings/SubStatus';
  import { usercardIpc } from '../../ipc/rules';
  import { formatDate } from '../../lib/format';

  let { profile, channel }: { profile: Profile; channel: string } = $props();

  let sub = $state<SubStatus | null | undefined>(undefined);
  let follow = $state<FollowAge | null>(null);
  let names = $state.raw<NameChange[]>([]);

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

  /** "1 year 3 months", from a date to now. */
  function age(rfc: string): string {
    const from = new Date(rfc);
    const now = new Date();
    let months = (now.getFullYear() - from.getFullYear()) * 12 + now.getMonth() - from.getMonth();
    if (now.getDate() < from.getDate()) months -= 1;
    if (months < 1) return 'under a month';
    const y = Math.floor(months / 12);
    const m = months % 12;
    const part = (n: number, w: string) => (n ? `${n} ${w}${n === 1 ? '' : 's'}` : '');
    return [part(y, 'year'), part(m, 'month')].filter(Boolean).join(' ');
  }

  const subText = $derived.by(() => {
    if (sub === undefined) return 'Loading…';
    if (sub === null) return '—';
    if (sub.hidden) return 'Hidden';
    if (sub.active) {
      const how = sub.tier ? (TIERS[sub.tier] ?? sub.tier) : 'Subbed';
      return `${how}${sub.gift ? ', gifted' : ''} · ${sub.months} month${sub.months === 1 ? '' : 's'}`;
    }
    return sub.months ? `Not now · was ${sub.months} month${sub.months === 1 ? '' : 's'}` : 'Never';
  });
</script>

<dl>
  {#if !own}
    <dt>Sub</dt>
    <dd>{subText}</dd>
    {#if follow?.known}
      <dt>Following</dt>
      <dd>{follow.since ? `${day(follow.since)} · ${age(follow.since)}` : 'No'}</dd>
    {/if}
  {/if}
  {#if names.length}
    <dt>Was</dt>
    <dd>
      {#each names as n, i (n.login)}{i ? ', ' : ''}<span title="{day(n.firstSeen)} – {day(n.lastSeen)}">{n.login}</span
        >{/each}
    </dd>
  {/if}
</dl>

<style>
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 12px;
    margin: 0 0 12px;
    font-size: 0.85rem;
  }
  dl:empty {
    display: none;
  }
  dt {
    color: var(--dim);
  }
  dd {
    margin: 0;
  }
</style>
