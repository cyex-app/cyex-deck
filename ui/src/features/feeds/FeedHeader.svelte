<!-- V106-07 The top bar of every feed tab: icon, name, then the tab's own buttons. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import type { IconName } from '../../components/icons';
  import { tooltip } from '../../components/tooltip.svelte';

  interface Props {
    icon: IconName;
    name: string;
    /** Tooltip on the name. */
    hint?: string;
    /** Before the tab's buttons, e.g. Clear. */
    extra?: Snippet;
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
  }

  let { icon, name, hint, extra, actions, onheaderdown }: Props = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header onpointerdown={onheaderdown}>
  <Icon name={icon} size={14} />
  {#if hint}
    <span class="name" use:tooltip={hint}>{name}</span>
  {:else}
    <span class="name">{name}</span>
  {/if}
  {@render extra?.()}
  {@render actions?.()}
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 6px 0 10px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
  }
  .name {
    flex: 1;
    color: var(--text);
    font-weight: 700;
  }
</style>
