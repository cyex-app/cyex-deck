<!-- Reset one settings partition. Needs a second click to confirm.
     t5 With `page`, only what that page shows: its partition also feeds other pages. -->
<script lang="ts">
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import type { Partition } from '../../ipc/bindings/Partition';
  import type { DeepPartial, SettingsValues } from '../../ipc/settings';
  import { errorText } from '../../lib/errors';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { ENTRIES, type PageId } from './registry';
  import { pageResetPatch } from './resetPatch';

  /** V107-R4 `warn`: what else goes, shown under the button while it asks again. */
  let {
    partition,
    page,
    onreset,
    warn,
  }: { partition: Partition; page?: PageId; onreset?: () => void; warn?: string } = $props();
  let armed = $state(false);

  function reset() {
    const done = page
      ? settings.patch(partition, pageResetPatch(ENTRIES, partition, page) as DeepPartial<SettingsValues[typeof partition]>)
      : settings.reset(partition);
    // V110-F84 A failed reset says so instead of going unnoticed.
    void done.then(() => onreset?.()).catch((e) => toast.error(errorText(e)));
  }
</script>

<div class="reset">
  {#if armed && warn}<p class="warn" role="status">{warn}</p>{/if}
  <ConfirmButton label="Reset" confirm="Click to reset" icon="undo" variant="ghost" onconfirm={reset} bind:armed />
</div>

<style>
  .reset {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
    padding-top: 16px;
  }
  .warn {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-hint);
  }
</style>
