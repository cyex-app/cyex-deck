<!-- A date divider between days of messages: chat and whispers share it. -->
<script lang="ts">
  let { label }: { label: string } = $props();
</script>

<p class="day" role="separator">{label}</p>

<style>
  .day {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 10px;
    color: var(--dim);
    font: 600 0.72rem var(--font-mono);
  }
  .day::before,
  .day::after {
    content: '';
    flex: 1;
    border-top: 1px solid var(--border);
  }
</style>
