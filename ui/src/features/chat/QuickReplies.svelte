<!-- V108-N20 Saved phrases one click from the message box. A pick puts the text in the box;
     it never sends. They are edited in Settings → Chat. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { settings } from '../../lib/settings.svelte';

  interface Props {
    /** Puts the phrase at the caret. */
    oninsert: (text: string) => void;
  }

  let { oninsert }: Props = $props();

  const replies = $derived(settings.v.chat.quickReplies);

  /** Long phrases are cut for the menu; the whole text goes in the box. */
  const shown = (r: string) => (r.length > 60 ? `${r.slice(0, 59)}…` : r);

  function open(e: MouseEvent) {
    openMenuAt(
      e,
      replies.map((r) => ({ label: shown(r), note: r.length > 60 ? r : undefined, run: () => oninsert(r) })),
    );
  }
</script>

{#if replies.length}
  <button
    class="quick"
    aria-label="Quick replies"
    aria-haspopup="menu"
    use:tooltip={{ label: 'Quick replies', hint: 'Saved phrases. A click puts one in the box.' }}
    onclick={open}
  >
    <Icon name="chat" size={16} />
  </button>
{/if}

<style>
  .quick {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    margin-bottom: 1px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .quick:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
</style>
