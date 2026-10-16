<!-- 1.40 The button next to the message box that opens the emote menu above it. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { layer } from '../../lib/escape';
  import { firstField, focusScope } from '../../lib/focusScope';
  import { floating } from '../../lib/place';
  import EmoteMenu from '../emotes/EmoteMenu.svelte';

  interface Props {
    login: string;
    account: string | null;
    oninsert: (text: string) => void;
  }

  let { login, account, oninsert }: Props = $props();

  let open = $state(false);
  let wrap = $state<HTMLElement>();
  let pop = $state<HTMLElement>();

  $effect(() => {
    if (!open || !pop) return;
    // A click in the shared menu (right-click > Add to favorites) stays inside the picker.
    const down = (e: MouseEvent) => {
      const t = e.target as Element;
      if (wrap && !wrap.contains(t) && !t.closest?.('[data-shared-menu]')) open = false;
    };
    const off = layer('popover', () => (open = false));
    const release = focusScope(pop, { initial: firstField });
    addEventListener('mousedown', down, true);
    return () => {
      off();
      release();
      removeEventListener('mousedown', down, true);
    };
  });
</script>

<div class="wrap" bind:this={wrap}>
  <button class="smile" class:on={open} aria-label="Emotes" use:tooltip={{ label: 'Emotes', hint: 'Twitch, 7TV, BTTV, FFZ and emoji. Right-click one to keep it in Favorites.' }} onclick={() => (open = !open)}>
    <Icon name="smile" size={17} />
  </button>
  {#if open}
    <div class="pop" bind:this={pop} tabindex="-1" use:floating={{ anchor: wrap, side: 'above', align: 'end', gap: 8 }}>
      <EmoteMenu {login} {account} {oninsert} />
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    display: flex;
  }
  .smile {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .smile:hover,
  .smile.on {
    color: var(--text);
    background: var(--surface-hover);
  }
  /* V102-03 Fixed to the window, so panels around a chat can't cut it off. */
  .pop {
    position: fixed;
    z-index: var(--z-popover);
    display: flex;
    flex-direction: column;
  }
</style>
