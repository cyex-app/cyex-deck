<!-- V107-R36..R40 A stream manager panel that couldn't load: what went wrong and Retry. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';

  let { text, onretry }: { text: string; onretry: () => void } = $props();
</script>

<div class="problem" role="alert">
  <EmptyState icon="alert" {text}><Button small onclick={onretry}>Retry</Button></EmptyState>
</div>

<style>
  .problem {
    flex: 1;
    display: flex;
    padding: 16px 12px;
    text-align: center;
    overflow-wrap: anywhere;
  }
</style>
