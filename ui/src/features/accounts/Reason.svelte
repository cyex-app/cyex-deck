<!-- V109-B13 A turned-down picture's reason: three lines at most, "More" shows the rest. A long
     word breaks instead of running out of the box. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';

  let { text }: { text: string } = $props();

  let open = $state(false);
  let cut = $state(false);
  let el = $state<HTMLElement>();

  // Measured while clamped; once open the toggle stays so "Less" is there.
  $effect(() => {
    void text;
    const node = el;
    if (!node || open) return;
    const read = () => (cut = node.scrollHeight > node.clientHeight + 1);
    read();
    const ro = new ResizeObserver(read);
    ro.observe(node);
    return () => ro.disconnect();
  });
</script>

<span class="reason-box">
  <span class="reason" class:clamp={!open} bind:this={el}>{text}</span>
  {#if cut || open}
    <Button small variant="link" onclick={() => (open = !open)}>{open ? 'Less' : 'More'}</Button>
  {/if}
</span>

<style>
  .reason-box {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    min-width: 0;
    max-width: 100%;
  }
  .reason {
    color: var(--muted);
    font-size: var(--text-hint);
    overflow-wrap: anywhere;
  }
  .reason.clamp {
    display: -webkit-box;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
  }
</style>
