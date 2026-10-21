<!-- V107-R37, R39 A row's tick box in a stream manager list; the name is for screen readers. -->
<script lang="ts">
  import Checkbox from '../../components/Checkbox.svelte';

  let { label, checked, onchange }: { label: string; checked: boolean; onchange: () => void } = $props();
</script>

<span class="pickbox"><Checkbox {label} {checked} onchange={() => onchange()} /></span>

<style>
  .pickbox {
    flex: none;
    display: inline-flex;
  }
  /* The box only. */
  .pickbox :global(.check) {
    gap: 0;
    font-size: 0;
  }
</style>
