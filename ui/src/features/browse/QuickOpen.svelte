<!-- 1.6 The three ways to open a channel, plus its star (H2). Dark over pictures; `flat`
     sits on the page, in list rows. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { type Channel, LAYOUT_LABELS, type Layout, type Nav, isFavorite, openChannel, toggleFavorite } from './open.svelte';

  let { channel, nav, flat = false }: { channel: Channel; nav: Nav; flat?: boolean } = $props();

  const BUTTONS: { layout: Layout; icon: 'tv' | 'panelRight' | 'chat' }[] = [
    { layout: 'stream', icon: 'tv' },
    { layout: 'both', icon: 'panelRight' },
    { layout: 'chat', icon: 'chat' },
  ];

  const starred = $derived(isFavorite(channel.id));
</script>

<div class="quick" class:flat>
  {#each BUTTONS as b (b.layout)}
    <button
      aria-label={LAYOUT_LABELS[b.layout]}
      use:tooltip={LAYOUT_LABELS[b.layout]}
      onclick={() => void openChannel(nav, channel, { layout: b.layout })}
    >
      <Icon name={b.icon} size={15} />
    </button>
  {/each}
  <button
    class="star"
    class:on={starred}
    aria-label={starred ? 'Unfavorite' : 'Favorite'}
    aria-pressed={starred}
    use:tooltip={starred ? 'Unfavorite' : { label: 'Favorite', hint: 'Keeps it on top of home.' }}
    onclick={() => toggleFavorite(channel)}
  >
    <Icon name={starred ? 'starFilled' : 'star'} size={15} />
  </button>
</div>

<style>
  .quick {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--scrim-strong);
    border-radius: var(--radius);
  }
  button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 26px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--on-video-dim);
  }
  button:hover {
    background: rgba(255, 255, 255, 0.16);
    color: var(--on-video);
  }
  .star.on {
    color: #f5c518;
  }
  .flat {
    padding: 0;
    background: none;
  }
  .flat button {
    color: var(--dim);
  }
  .flat button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .flat .star.on {
    color: var(--yellow);
  }
</style>
