<!-- Badges (1.37-1.40, 2.3): Twitch's, with FFZ's room pictures for mods and VIPs and
     FFZ's bot badge in place of the mod one; then 7TV, BTTV and FFZ badges. Each provider
     has its toggle. Unknown Twitch badges are skipped until their catalog loads. -->
<script lang="ts">
  import type { BadgeDef } from '../../ipc/bindings/BadgeDef';
  import type { BadgeInfo } from '../../ipc/bindings/BadgeInfo';
  import type { Look } from '../../ipc/bindings/Look';
  import { density } from '../../lib/density.svelte';
  import { focus } from '../../lib/focus.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { looks } from '../emotes/looks.svelte';
  import { rooms } from '../emotes/rooms.svelte';
  import { catalogs } from './catalog.svelte';
  import { saved } from './saved';
  import { badgeSrc } from './urls';

  type Ref = { set: string; version: string; info?: string };

  interface Props {
    badges: Ref[];
    channel: string;
    /** The sender, for provider badges. Left out for shared-chat source badges. */
    userId?: string;
    /** V1051-03 The cosmetics preview: draw this look instead of the user's. */
    look?: Look | null;
  }

  let { badges, channel, userId, look: override }: Props = $props();

  // V1051-03 The preview stands still with the rest of the menu.
  const still = $derived(focus.still || (override !== undefined && focus.idle));

  const on = $derived(settings.v.chat.badges);
  const room = $derived(on.ffz ? rooms.get(channel) : undefined);
  const extra = $derived.by(() => {
    if (!userId && override === undefined) return [];
    const look = override !== undefined ? override : looks.look(userId!);
    if (!look) return [];
    return look.badges
      .filter((b) => on[b.provider])
      .map((b) => looks.badge(b.provider, b.id))
      .filter((b): b is BadgeDef => !!b);
  });
  /** FFZ badges that stand in for a Twitch one (the bot badge for mod). */
  const swaps = $derived(new Map(extra.filter((b) => b.replaces).map((b) => [b.replaces!, b])));
  const isMod = (set: string) => set === 'moderator' || set === 'lead_moderator';
  /** Provider badges not already drawn in a Twitch badge's place. */
  const rest = $derived(
    extra.filter((b) => !(on.twitch && b.replaces === 'moderator' && badges.some((r) => isMod(r.set)))),
  );

  function hint(ref: Ref, b: BadgeInfo): string | undefined {
    if (ref.set === 'subscriber' || ref.set === 'founder') return ref.info ? `${ref.info} ${ref.info === '1' ? 'month' : 'months'}` : undefined;
    return ref.info ?? (b.description && b.description !== b.title ? b.description : undefined);
  }

  /** What draws in a Twitch badge's place: FFZ's version, or Twitch's own. */
  function stand(ref: Ref): BadgeDef | undefined {
    if (isMod(ref.set)) return swaps.get('moderator') ?? room?.moderator ?? undefined;
    if (ref.set === 'vip') return room?.vip ?? undefined;
    return undefined;
  }
</script>

{#snippet provider(b: BadgeDef)}
  <img
    class="badge"
    class:tile={!!b.color}
    style:background-color={b.color}
    src={still && b.still ? b.still : badgeSrc(b, density.scale)}
    alt={b.title}
    data-tip={b.title}
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
      {@const b = catalogs.badge(channel, ref.set, ref.version)}
      {@const copy = saved(`badge:${ref.set}/${ref.version}`)}
      {#if copy}
        <img class="badge" src={copy} alt={b?.title ?? ref.set} data-tip={b?.title ?? ref.set} />
      {:else if b}
        <img
          class="badge"
          src={badgeSrc(b, density.scale)}
          alt={b.title}
          data-tip={b.title}
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
