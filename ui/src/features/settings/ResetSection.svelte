<!-- Reset one settings partition. Needs a second click to confirm. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { Partition } from '../../ipc/bindings/Partition';
  import { settings } from '../../lib/settings.svelte';

  let { partition, onreset }: { partition: Partition; onreset?: () => void } = $props();

  let armed = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function click() {
    if (!armed) {
      armed = true;
      timer = setTimeout(() => (armed = false), 3000);
      return;
    }
    clearTimeout(timer);
    armed = false;
    void settings.reset(partition).then(() => onreset?.());
  }
</script>

<div class="reset">
  <Button variant={armed ? 'danger' : 'ghost'} small icon="undo" onclick={click}>
    {armed ? 'Click again to reset' : 'Reset to defaults'}
  </Button>
</div>

<style>
  .reset {
    display: flex;
    justify-content: flex-end;
    padding-top: 16px;
  }
</style>
