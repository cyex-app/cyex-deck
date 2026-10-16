<!-- Reset one settings partition. Needs a second click to confirm. -->
<script lang="ts">
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import type { Partition } from '../../ipc/bindings/Partition';
  import { settings } from '../../lib/settings.svelte';

  let { partition, onreset }: { partition: Partition; onreset?: () => void } = $props();

  function reset() {
    void settings.reset(partition).then(() => onreset?.());
  }
</script>

<div class="reset">
  <ConfirmButton label="Reset" confirm="Click again to reset" icon="undo" variant="ghost" onconfirm={reset} />
</div>

<style>
  .reset {
    display: flex;
    justify-content: flex-end;
    padding-top: 16px;
  }
</style>
