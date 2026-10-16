<!-- V1051-03 Your look in both themes, drawn by the chat's own line. The status is in Status.svelte. -->
<script lang="ts">
  import { looks } from '../emotes/looks.svelte';
  import Line from '../chat/Line.svelte';
  import { previewMessage, type CosmeticsStore } from './cosmetics.svelte';

  interface Props {
    store: CosmeticsStore;
    who: { userId: string; login: string; displayName: string };
    /** What's shown: the draft while a combo is edited. */
    shown: { paint: string | null; badge: string | null };
  }

  let { store, who, shown }: Props = $props();

  const themeNow = () => (document.documentElement.dataset.theme === 'light' ? 'light' : 'dark');
  let current = $state<'dark' | 'light'>(themeNow());
  const other = $derived(current === 'dark' ? 'light' : 'dark');
  const boxes = $derived([current, other] as const);

  // A theme change while the menu is open turns the boxes with it.
  $effect(() => {
    const mo = new MutationObserver(() => (current = themeNow()));
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => mo.disconnect();
  });

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
</script>

<div class="wrap">
  <div class="boxes">
    {#each boxes as theme (theme)}
      <div class="box" class:theme-dark={theme !== current && theme === 'dark'} class:theme-light={theme !== current && theme === 'light'}>
        <Line m={previewMessage(who, theme)} channel="" {look} />
      </div>
    {/each}
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
  @container (max-width: 300px) {
    .boxes {
      grid-template-columns: 1fr;
    }
  }
  .box {
    min-width: 0;
    padding: 6px 4px;
    /* V1051-03 Light: a soft grey, so pale paints stay visible. */
    background: var(--paint-back);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
  }
</style>
