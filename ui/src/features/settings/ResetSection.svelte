<!-- Reset one settings partition. Needs a second click to confirm.
     t5 With `page`, only what that page shows: its partition also feeds other pages. -->
<script lang="ts">
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import type { Partition } from '../../ipc/bindings/Partition';
  import type { DeepPartial, SettingsValues } from '../../ipc/settings';
  import { settings } from '../../lib/settings.svelte';
  import { ENTRIES, type PageId } from './registry';
  import { pageResetPatch } from './resetPatch';

  let { partition, page, onreset }: { partition: Partition; page?: PageId; onreset?: () => void } = $props();

  function reset() {
    const done = page
      ? settings.patch(partition, pageResetPatch(ENTRIES, partition, page) as DeepPartial<SettingsValues[typeof partition]>)
      : settings.reset(partition);
    void done.then(() => onreset?.());
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
