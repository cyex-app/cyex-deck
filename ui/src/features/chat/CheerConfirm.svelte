<!-- V107-B From the threshold in Settings → Chat up, a pasted or recalled cheer and the same
     cheer again right after ask once more. A held or doubled Enter from the box can't
     spend: Enter counts only after a short moment. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import { formatNumber } from '../../lib/format';

  let {
    bits,
    text,
    again = false,
    onyes,
    onno,
  }: { bits: number; text: string; again?: boolean; onyes: () => void; onno: () => void } = $props();

  const READY_MS = 400;
  const opened = performance.now();

  function guard(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.repeat || performance.now() - opened < READY_MS)) {
      e.preventDefault();
      e.stopPropagation();
    }
  }
</script>

<svelte:window onkeydowncapture={guard} />

<Dialog title="Spend {formatNumber(bits)} Bits?" onclose={onno} width={380}>
  <p class="text">{text}</p>
  <p class="hint">{again ? 'You just sent this cheer. This sends it again.' : 'This spends real Bits.'}</p>
  {#snippet footer()}
    <Button variant="ghost" onclick={onno}>Cancel</Button>
    <Button variant="primary" onclick={onyes}>Cheer</Button>
  {/snippet}
</Dialog>

<style>
  p {
    margin: 0;
  }
  .text {
    margin-bottom: 8px;
    overflow-wrap: anywhere;
    color: var(--text);
  }
  .hint {
    color: var(--muted);
    font-size: var(--text-sub);
  }
</style>
