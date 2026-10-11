<!-- 1.40 The button next to the message box that opens the emote menu above it. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
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

  $effect(() => {
    if (!open) return;
    const down = (e: MouseEvent) => {
      if (wrap && !wrap.contains(e.target as Node)) open = false;
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') open = false;
    };
    addEventListener('mousedown', down, true);
    addEventListener('keydown', key);
    return () => {
      removeEventListener('mousedown', down, true);
      removeEventListener('keydown', key);
    };
  });
</script>

<div class="wrap" bind:this={wrap}>
  <button class="smile" class:on={open} aria-label="Emotes" use:tooltip={{ label: 'Emotes', hint: 'Twitch, 7TV, BTTV, FFZ and emoji. Right-click one to keep it in Favorites.' }} onclick={() => (open = !open)}>
    <Icon name="smile" size={17} />
  </button>
  {#if open}
    <div class="pop" use:floating={{ anchor: wrap, side: 'above', align: 'end', gap: 8 }}>
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
