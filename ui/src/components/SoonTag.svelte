<!-- V112-23 The tag beside something that isn't open yet (cyex.tv). Not a button. -->
<script lang="ts">
  import { SOON } from '../lib/cyextv';
  import { tooltip } from './tooltip.svelte';

  let { hint }: { hint?: string } = $props();
</script>

<span class="soon" use:tooltip={{ label: SOON, hint, when: () => !!hint }}>{SOON}</span>

<style>
  .soon {
    display: inline-flex;
    flex: none;
    align-items: center;
    height: 18px;
    padding: 0 6px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.7rem;
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
  }
</style>
