<!-- V102-13 Every Twitch badge someone owns in the channel (Twitch's public
     `channelViewer.earnedBadges`, no sign-in needed), then their 7TV, BTTV and FFZ badges.
     One line at most, with "Show more" when they don't fit, like the 7TV extension's card.
     Until the list loads, or if it can't, the badges from their last message show. -->
<script lang="ts">
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Badge } from '../../ipc/bindings/Badge';
  import type { BadgeInfo } from '../../ipc/bindings/BadgeInfo';
  import { usercardIpc } from '../../ipc/rules';
  import { density } from '../../lib/density.svelte';
  import { settings } from '../../lib/settings.svelte';
  import Badges from '../chat/Badges.svelte';
  import { badgeSrc } from '../chat/urls';

  interface Props {
    login: string;
    userId: string;
    channel: string;
    /** From their last message here. */
    fallback: Badge[];
  }

  let { login, userId, channel, fallback }: Props = $props();

  let earned = $state.raw<BadgeInfo[] | null>(null);
  let open = $state(false);
  let overflows = $state(false);
  let row = $state<HTMLElement>();

  const twitch = $derived(settings.v.chat.badges.twitch);

  $effect(() => {
    let live = true;
    usercardIpc
      .badges(login, channel)
      .then((list) => live && (earned = list))
      .catch(() => {});
    return () => {
      live = false;
    };
  });

  // Whether the closed line hides any: measured, since badge counts and widths vary.
  $effect(() => {
    const el = row;
    // Again when the list arrives: the closed row keeps its size, so nothing else says so.
    void earned;
    void twitch;
    if (!el || open) return;
    const measure = () => (overflows = el.scrollHeight > el.clientHeight + 1);
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    measure();
    return () => ro.disconnect();
  });
</script>

<div class="row" class:open bind:this={row}>
  {#if earned && twitch}
    {#each earned as b (`${b.set}/${b.version}`)}
      <img
        class="badge"
        src={badgeSrc(b, density.scale)}
        alt={b.title}
        loading="lazy"
        decoding="async"
        use:tooltip={{ label: b.title, hint: b.description && b.description !== b.title ? b.description : undefined }}
      />
    {/each}
    <Badges badges={[]} {channel} {userId} />
  {:else}
    <Badges badges={fallback} {channel} {userId} />
  {/if}
</div>
{#if overflows || open}
  <button class="more" onclick={() => (open = !open)}>{open ? 'Show less' : 'Show more'}</button>
{/if}

<style>
  /* One line of 18 px badges until opened. */
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    row-gap: 3px;
    max-height: 18px;
    overflow: hidden;
  }
  .row.open {
    max-height: none;
  }
  .badge {
    width: 18px;
    height: 18px;
    margin-right: 3px;
  }
  .more {
    align-self: flex-start;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
    font-size: 0.72rem;
  }
  .more:hover {
    color: var(--text);
  }
</style>
