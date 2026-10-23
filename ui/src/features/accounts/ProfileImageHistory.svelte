<!-- V107-R19c The last images changed in the app for one kind: click one, confirm inline, and it goes
     up again as it was (no new crop). ✕ drops one from the list. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { ProfileImageKind } from '../../ipc/bindings/ProfileImageKind';
  import type { ProfileImageSaved } from '../../ipc/bindings/ProfileImageSaved';

  interface Props {
    kind: ProfileImageKind;
    items: ProfileImageSaved[];
    disabled: boolean;
    onuse: (id: string) => void;
    onforget: (id: string) => void;
  }

  let { kind, items, disabled, onuse, onforget }: Props = $props();

  let asking = $state<string | null>(null);

  $effect(() => {
    if (asking && !items.some((i) => i.id === asking)) asking = null;
  });

  // V107-N5b A change or restore under way (from here or from the row) closes the question.
  $effect(() => {
    if (disabled) asking = null;
  });

  function use() {
    if (!asking) return;
    const id = asking;
    asking = null;
    onuse(id);
  }
</script>

<div class="history" data-kind={kind}>
  <span class="recent">Recent</span>
  <div class="items">
    {#each items as item (item.id)}
      <div class="item" class:asking={asking === item.id} class:current={item.current}>
        <button
          type="button"
          class="thumb {kind}"
          disabled={disabled || item.current}
          aria-label={item.current ? 'In use' : 'Use this one'}
          use:tooltip={item.current ? 'In use' : 'Use this one'}
          onclick={() => (asking = asking === item.id ? null : item.id)}
        >
          <img src={item.thumb} alt="" />
        </button>
        <button
          type="button"
          class="drop"
          {disabled}
          aria-label="Remove from recent"
          use:tooltip={'Remove from recent'}
          onclick={() => onforget(item.id)}
        >
          <Icon name="x" size={10} />
        </button>
      </div>
    {/each}
  </div>
  {#if asking}
    <div class="ask">
      <span>Use this one?</span>
      <Button small {disabled} onclick={use}>Use</Button>
      <Button small variant="ghost" onclick={() => (asking = null)}>Cancel</Button>
    </div>
  {/if}
</div>

<style>
  .history {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 10px;
    padding: 0 0 10px 108px;
  }
  .recent {
    color: var(--dim);
    font-size: 0.74rem;
  }
  .items {
    display: flex;
    gap: 8px;
  }
  .item {
    position: relative;
  }
  .thumb {
    display: block;
    padding: 0;
    overflow: hidden;
    width: 48px;
    height: 27px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg);
    cursor: pointer;
    transition:
      transform 100ms ease-out,
      box-shadow var(--fast);
  }
  .thumb.avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
  }
  .thumb img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .thumb:hover:not(:disabled) {
    border-color: var(--border-strong);
  }
  .thumb:active:not(:disabled) {
    transform: scale(0.94);
  }
  .thumb:disabled {
    cursor: default;
  }
  .item.current .thumb,
  .item.asking .thumb {
    border-color: transparent;
    box-shadow:
      0 0 0 1px var(--surface),
      0 0 0 3px var(--accent-border);
  }
  .item.asking .thumb {
    box-shadow:
      0 0 0 1px var(--surface),
      0 0 0 3px var(--accent-line);
  }
  .drop {
    position: absolute;
    top: -5px;
    right: -5px;
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--fast);
  }
  .item:hover .drop,
  .drop:focus-visible {
    opacity: 1;
  }
  .drop:hover:not(:disabled) {
    color: var(--red);
  }
  .drop:disabled {
    display: none;
  }
  .ask {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.82rem;
  }
</style>
