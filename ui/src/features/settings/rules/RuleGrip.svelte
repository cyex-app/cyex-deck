<!-- V106-10 The drag grip of a reorderable rule row, or an empty grip-wide slot so fixed
     rows line up. U1/U19 The hint lives inside the button, so the row's dimming rule
     can't mistake it for the next cell. -->
<script lang="ts">
  import Icon from '../../../components/Icon.svelte';
  import { tooltip } from '../../../components/tooltip.svelte';

  interface Props {
    /** The grip's accessible name, e.g. "Move /hi". No button when unset. */
    handle?: string;
    /** An empty grip-wide column when there is no handle. */
    slot?: boolean;
  }
  let { handle, slot = false }: Props = $props();
  const uid = $props.id();
</script>

{#if handle}
  <button
    type="button"
    class="grip"
    data-drag-handle
    aria-label={handle}
    aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown"
    aria-describedby="{uid}-grip"
    use:tooltip={{ label: 'Drag to move', hint: 'Or press Alt+↑ or Alt+↓.' }}
    ><Icon name="grip" size={14} /><span class="sr-only" id="{uid}-grip">Press Alt+↑ or Alt+↓ to move.</span></button
  >
{:else if slot}
  <span class="grip" aria-hidden="true"></span>
{/if}

<style>
  .grip {
    width: 18px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    background: none;
    color: var(--dim);
    border-radius: var(--radius);
    align-self: start;
  }
  button.grip {
    cursor: grab;
    position: relative;
  }
  /* 24 px hit area; the column stays 18 px. */
  button.grip::before {
    content: '';
    position: absolute;
    inset: -2px -3px;
  }
  button.grip:hover {
    color: var(--text);
  }
  /* s1-8 Press feedback. */
  button.grip:active {
    cursor: grabbing;
    color: var(--text);
    background: var(--surface-hover);
  }
  button.grip:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
</style>
