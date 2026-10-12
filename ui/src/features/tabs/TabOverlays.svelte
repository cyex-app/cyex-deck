<!-- Dialogs and drag feedback every window with tabs needs. -->
<script lang="ts">
  import { groupDialog, tabDialog } from './dialog.svelte';
  import DragGhost from './DragGhost.svelte';
  import GroupDialog from './GroupDialog.svelte';
  import TabDialog from './TabDialog.svelte';

  let { onopen }: { onopen: (id: string) => void } = $props();
</script>

{#if tabDialog.mode}
  {#key tabDialog.mode}
    <TabDialog mode={tabDialog.mode} {onopen} />
  {/key}
{/if}
{#if groupDialog.mode}
  {#key groupDialog.mode}
    <GroupDialog mode={groupDialog.mode} />
  {/key}
{/if}
<DragGhost />
