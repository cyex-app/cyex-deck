<!-- Draws the shared right-click menu at the pointer, kept inside the window. -->
<script lang="ts">
  import Icon from './Icon.svelte';
  import { back, closeMenu, menu, pick } from './menu.svelte';

  const EDGE = 6;
  let box = $state<HTMLElement>();
  let width = $state(0);
  let height = $state(0);
  let vw = $state(0);
  let vh = $state(0);

  const left = $derived(Math.max(EDGE, Math.min(menu.x, vw - width - EDGE)));
  const top = $derived(Math.max(EDGE, Math.min(menu.y, vh - height - EDGE)));
  const list = $derived(menu.stack.at(-1));

  $effect(() => {
    if (!menu.open) return;
    const onDown = (e: MouseEvent) => {
      if (!box?.contains(e.target as Node)) closeMenu();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeMenu();
    };
    addEventListener('mousedown', onDown, true);
    addEventListener('keydown', onKey);
    addEventListener('blur', closeMenu);
    addEventListener('resize', closeMenu);
    return () => {
      removeEventListener('mousedown', onDown, true);
      removeEventListener('keydown', onKey);
      removeEventListener('blur', closeMenu);
      removeEventListener('resize', closeMenu);
    };
  });
</script>

<svelte:window bind:innerWidth={vw} bind:innerHeight={vh} />

{#if menu.open && list}
  <div
    class="menu"
    role="menu"
    tabindex="-1"
    bind:this={box}
    bind:clientWidth={width}
    bind:clientHeight={height}
    style:left="{left}px"
    style:top="{top}px"
    oncontextmenu={(e) => e.preventDefault()}
  >
    {#if menu.stack.length > 1}
      <button class="item head" role="menuitem" onclick={back}>
        <Icon name="chevronLeft" size={14} />
        <span class="text">{list.title}</span>
      </button>
      <hr />
    {/if}
    {#each list.items as entry, i (i)}
      {#if entry === 'separator'}
        <hr />
      {:else}
        <button
          class="item"
          class:danger={entry.danger}
          role={entry.checked === undefined ? 'menuitem' : 'menuitemcheckbox'}
          aria-checked={entry.checked}
          disabled={entry.disabled}
          onclick={() => pick(entry)}
        >
          {#if entry.swatch}<span class="swatch" style:background={entry.swatch}></span>
          {:else if entry.icon}<Icon name={entry.icon} size={14} />
          {:else}<span class="blank"></span>{/if}
          <span class="text">{entry.label}</span>
          {#if entry.hint}<span class="hint">{entry.hint}</span>{/if}
          {#if entry.checked}<span class="check"><Icon name="check" size={14} /></span>{/if}
          {#if entry.items}<Icon name="chevronRight" size={13} />{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: var(--z-popover);
    min-width: 200px;
    max-width: 300px;
    max-height: calc(100vh - 12px);
    overflow-y: auto;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px var(--shadow);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
  }
  .item:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .item:disabled {
    opacity: 0.45;
  }
  .item.danger {
    color: var(--red);
  }
  .head {
    color: var(--muted);
    font-weight: 600;
  }
  .text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    color: var(--dim);
    font-size: 0.76rem;
  }
  .swatch {
    width: 12px;
    height: 12px;
    margin: 0 1px;
    border-radius: 50%;
  }
  .blank {
    width: 14px;
  }
  .check {
    display: flex;
    color: var(--accent-text);
  }
  hr {
    margin: 4px 0;
    border: 0;
    border-top: 1px solid var(--border);
  }
</style>
