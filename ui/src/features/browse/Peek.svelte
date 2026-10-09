<!-- The picture `peekAt` (peeking.svelte.ts) asks for. One per view. -->
<script lang="ts">
  import { formatShort } from '../../lib/format';
  import { preview } from './lists';
  import { peek } from './peeking.svelte';

  const stream = $derived(peek.card?.stream);
</script>

{#if peek.card && stream}
  <div class="peek" style:left="{peek.x}px" style:top="{peek.y}px">
    <img src={preview(peek.card.login, 320, 180)} alt="" />
    <div class="text">
      <span class="title">{stream.title || 'Untitled'}</span>
      <span class="sub">{[stream.game?.name, `${formatShort(stream.viewers)} viewers`].filter(Boolean).join(' · ')}</span>
    </div>
  </div>
{/if}

<style>
  .peek {
    position: fixed;
    z-index: var(--z-tooltip);
    width: 320px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px var(--shadow);
    overflow: hidden;
    pointer-events: none;
  }
  img {
    display: block;
    width: 320px;
    height: 180px;
    background: var(--surface);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 10px;
  }
  .title {
    font-size: 0.82rem;
    font-weight: 600;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .sub {
    color: var(--dim);
    font-size: 0.75rem;
  }
</style>
