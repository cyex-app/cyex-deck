<!-- H3 Channels picked with Ctrl+click, opened side by side in one tab. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { LAYOUT_LABELS, type Layout, type Nav, picked } from './open.svelte';

  let { nav }: { nav: Nav } = $props();

  const BUTTONS: { layout: Layout; icon: 'tv' | 'panelRight' | 'chat' }[] = [
    { layout: 'stream', icon: 'tv' },
    { layout: 'both', icon: 'panelRight' },
    { layout: 'chat', icon: 'chat' },
  ];
</script>

{#if picked.list.length}
  <div class="bar" role="toolbar" aria-label="Picked channels">
    <span class="names" title={picked.list.map((c) => c.name).join(', ')}>
      {picked.list.length === 1 ? picked.list[0].name : `${picked.list.length} channels`}
    </span>
    <span class="hint">Side by side:</span>
    {#each BUTTONS as b (b.layout)}
      <button class="open" onclick={() => void picked.open(nav, b.layout)}>
        <Icon name={b.icon} size={14} />{LAYOUT_LABELS[b.layout]}
      </button>
    {/each}
    <button class="clear" aria-label="Clear" onclick={() => picked.clear()}><Icon name="x" size={14} /></button>
  </div>
{/if}

<style>
  .bar {
    position: absolute;
    left: 50%;
    bottom: 16px;
    z-index: 5;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: calc(100% - 32px);
    padding: 6px 6px 6px 12px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px var(--shadow);
  }
  .names {
    font-weight: 600;
    font-size: 0.85rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hint {
    color: var(--dim);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .open {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .open:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .clear {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
  }
  .clear:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
