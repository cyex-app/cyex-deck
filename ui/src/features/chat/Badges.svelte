<!-- Badges (1.37-1.40, 2.3): Twitch's, with FFZ's room pictures for mods and VIPs and
     FFZ's bot badge in place of the mod one; then cyex's (V109-B02), 7TV, BTTV, FFZ and the extra sets (V108-N06). Each
     provider and set has its toggle. Unknown Twitch badges are skipped until their catalog loads
     (V108-BD or is read again because it lacks them). -->
<script lang="ts">
  import type { BadgeDef } from '../../ipc/bindings/BadgeDef';
  import type { BadgeInfo } from '../../ipc/bindings/BadgeInfo';
  import type { Look } from '../../ipc/bindings/Look';
  import { density } from '../../lib/density.svelte';
  import { focus } from '../../lib/focus.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { badgeOn } from '../emotes/badgeSets';
  import { looks } from '../emotes/looks.svelte';
  import { rooms } from '../emotes/rooms.svelte';
  import { catalogs } from './catalog.svelte';
  import { saved } from './saved';
  import { subHint } from './subTier';
  import type { BadgeRef } from './sharedBadges';
  import { badgeSrc } from './urls';

  type Ref = BadgeRef;

  /** Most lines have no FFZ stand-ins: one empty map for all of them. */
  const NO_SWAPS: ReadonlyMap<string, BadgeDef> = new Map();

  interface Props {
    badges: Ref[];
    channel: string;
    /** The sender, for provider badges. */
    userId?: string;
    /** V1051-03 The cosmetics preview: draw this look instead of the user's. */
    look?: Look | null;
  }

  let { badges, channel, userId, look: override }: Props = $props();

  // V1051-03 The preview stands still with the rest of the menu.
  const still = $derived(focus.still || (override !== undefined && focus.idle));

  const on = $derived(settings.v.chat.badges);
  /** V108-N06 FFZ:AP, Chatsen, DankChat and Homies, each its own switch. */
  const sets = $derived(settings.v.emotes.badgeSets);
  const room = $derived(on.ffz ? rooms.get(channel) : undefined);
  const extra = $derived.by(() => {
    if (!userId && override === undefined) return [];
    const look = override !== undefined ? override : looks.look(userId!);
    if (!look) return [];
    return look.badges
      .filter((b) => badgeOn(b, on, sets))
      .map((b) => looks.badge(b.provider, b.id))
      .filter((b): b is BadgeDef => !!b);
  });
  /** FFZ badges that stand in for a Twitch one (the bot badge for mod). */
  const swaps = $derived(
    extra.some((b) => b.replaces) ? new Map(extra.filter((b) => b.replaces).map((b) => [b.replaces!, b])) : NO_SWAPS,
  );
  const isMod = (set: string) => set === 'moderator' || set === 'lead_moderator';
  /** Provider badges not already drawn in a Twitch badge's place. */
  const rest = $derived(
    extra.filter(
      (b) =>
        !(on.twitch && b.replaces === 'moderator' && badges.some((r) => isMod(r.set))) &&
        // V108-C10 An FFZ VIP badge next to the VIP one (Twitch's or the room's) shows twice.
        !(on.twitch && b.replaces === 'vip' && badges.some((r) => r.set === 'vip')),
    ),
  );

  // V108-C10 Shared Chat badges from another room draw from that room's catalog, read once per room.
  // V108-BD A badge neither catalog has (new since they were read) asks for a re-read, unless
  // something else draws in its place.
  $effect(() => {
    for (const b of badges) {
      if (b.room) void catalogs.loadSource(b.room);
      if (on.twitch && !stand(b) && (b.room || !saved(`badge:${b.set}/${b.version}`))) {
        catalogs.missing(channel, b);
      }
    }
  });

  function hint(ref: Ref, b: BadgeInfo): string | undefined {
    if (ref.set === 'subscriber' || ref.set === 'founder') return subHint(ref.set, ref.version, ref.info);
    return ref.info ?? (b.description && b.description !== b.title ? b.description : undefined);
  }

  /** What draws in a Twitch badge's place: FFZ's version, or Twitch's own. */
  function stand(ref: Ref): BadgeDef | undefined {
    // V108-C10 FFZ's bot art is the person's, so it also stands in for another room's mod badge.
    if (ref.room) return isMod(ref.set) ? swaps.get('moderator') : undefined;
    if (isMod(ref.set)) return swaps.get('moderator') ?? room?.moderator ?? undefined;
    if (ref.set === 'vip') return room?.vip ?? undefined;
    return undefined;
  }
</script>

{#snippet provider(b: BadgeDef)}
  <img
    class="badge pale-edge"
    class:tile={!!b.color}
    style:background-color={b.color}
    src={still && b.still ? b.still : badgeSrc(b, density.scale)}
    alt={b.title}
    data-tip={b.title}
    data-tip-img={b.image4x}
    loading="lazy"
    decoding="async"
  />
{/snippet}

{#if on.twitch}
  {#each badges as ref (ref.set)}
    {@const sub = stand(ref)}
    {#if sub}
      {@render provider(sub)}
    {:else}
      {@const b = ref.room ? catalogs.sourceBadge(ref.room, ref.set, ref.version) : catalogs.badge(channel, ref.set, ref.version)}
      {@const copy = ref.room ? undefined : saved(`badge:${ref.set}/${ref.version}`)}
      {#if copy}
        <img class="badge pale-edge" src={copy} alt={b?.title ?? ref.set} data-tip={b?.title ?? ref.set} data-tip-img={b?.image4x} />
      {:else if b}
        <img
          class="badge pale-edge"
          src={badgeSrc(b, density.scale)}
          alt={b.title}
          data-tip={b.title}
          data-tip-img={b.image4x}
          data-hint={hint(ref, b)}
          loading="lazy"
          decoding="async"
        />
      {/if}
    {/if}
  {/each}
{/if}
{#each rest as b (`${b.provider}:${b.id}`)}
  {@render provider(b)}
{/each}

<style>
  .badge {
    width: 18px;
    height: 18px;
    margin-right: 3px;
    vertical-align: -3px;
  }
  .tile {
    border-radius: var(--radius-sm);
  }
</style>
