<!-- V1051-03 Your look in both themes, drawn by the chat's own line, plus the save status. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import Button from '../../components/Button.svelte';
  import { looks } from '../emotes/looks.svelte';
  import Line from '../chat/Line.svelte';
  import { errorText, previewMessage, type CosmeticsStore } from './cosmetics.svelte';

  interface Props {
    store: CosmeticsStore;
    who: { userId: string; login: string; displayName: string };
    compact: boolean;
    /** What's shown: the draft while a combo is edited. */
    shown: { paint: string | null; badge: string | null };
    /** Takes the place of the status line. */
    bar?: Snippet;
  }

  let { store, who, compact, shown, bar }: Props = $props();

  const current = document.documentElement.dataset.theme === 'light' ? 'light' : 'dark';
  const other = current === 'dark' ? 'light' : 'dark';
  const boxes = $derived([current, other] as const);

  const paint = $derived(shown.paint ? store.paintById.get(shown.paint) : undefined);
  const badge = $derived(shown.badge ? store.badgeById.get(shown.badge) : undefined);
  const look = $derived({
    paint: paint?.style.id ?? null,
    badges: badge ? [{ provider: 'seventv' as const, id: badge.def.id }] : [],
  });

  $effect(() => {
    // Only the chosen ones become global rules.
    looks.define(paint ? [paint.style] : [], badge ? [badge.def] : []);
  });

  const status = $derived(store.saveStatus);
</script>

<div class="wrap">
  <div class="boxes" class:stack={compact}>
    {#each boxes as theme (theme)}
      <div class="box" class:theme-dark={theme !== current && theme === 'dark'} class:theme-light={theme !== current && theme === 'light'}>
        <Line m={previewMessage(who, theme)} channel="" {look} />
      </div>
    {/each}
  </div>
  <div class="status" aria-live="polite">
    {#if bar}
      {@render bar()}
    {:else if store.note}
      <span>{store.note.text}</span>
      {#if store.note.undo}<Button variant="ghost" small onclick={() => store.undo()}>Undo</Button>{/if}
    {:else if status === 'saving'}
      <span>Saving…</span>
    {:else if status === 'saved'}
      <span>Saved</span>
      {#if store.canUndo}<Button variant="ghost" small onclick={() => store.undo()}>Undo</Button>{/if}
    {:else if typeof status === 'object'}
      <span class="err">{errorText(status.error)}</span>
    {/if}
  </div>
</div>

<style>
  .wrap {
    container-type: inline-size;
  }
  .boxes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .stack {
    grid-template-columns: 1fr;
  }
  @container (max-width: 360px) {
    .boxes {
      grid-template-columns: 1fr;
    }
  }
  .box {
    min-width: 0;
    padding: 6px 4px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .err {
    color: var(--red);
  }
</style>
