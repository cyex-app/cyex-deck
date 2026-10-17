<!-- Stream and Chat checkboxes for a tab; one of them always stays on. -->
<script lang="ts">
  import Checkbox from '../../components/Checkbox.svelte';
  import type { TileShow } from '../../ipc/bindings/TileShow';

  let { show = $bindable() }: { show: TileShow } = $props();
</script>

<div class="row">
  <Checkbox
    label="Stream"
    checked={show.player}
    disabled={show.player && !show.chat}
    onchange={(v) => (show = { ...show, player: v })}
  />
  <Checkbox
    label="Chat"
    checked={show.chat}
    disabled={show.chat && !show.player}
    onchange={(v) => (show = { ...show, chat: v })}
  />
</div>

<style>
  .row {
    display: flex;
    gap: 18px;
  }
</style>
