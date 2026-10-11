<!-- V103-06 Hands a panel's controls (drag, close) to whatever frame it draws. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { type PanelCtl, setPanel } from './panel';

  let { ctl, children }: { ctl: PanelCtl; children: Snippet } = $props();

  setPanel(() => ctl);
</script>

{@render children()}
