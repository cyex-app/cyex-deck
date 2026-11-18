<!-- V105-16 What Chatterino's highlights bring: new words, people and badges, duplicates (keep
     yours or take theirs) and what can't come. V111-60 Shared by the highlights dialog and
     Settings › Import. -->
<script lang="ts">
  import Segmented from '../../../components/Segmented.svelte';
  import type { HighlightPreview } from '../../../ipc/bindings/HighlightPreview';
  import { formatNumber } from '../../../lib/format';
  import { highlightHeadline } from './importAll';
  import SkippedList from './SkippedList.svelte';

  const CHOICES = [
    { value: 'mine', label: 'Keep mine' },
    { value: 'theirs', label: 'Use Chatterino’s' },
  ] as const;

  interface Props {
    preview: HighlightPreview;
    choice: 'mine' | 'theirs';
  }

  let { preview, choice = $bindable() }: Props = $props();

  const dupes = $derived(preview.duplicates.length);
</script>

<p class="head">{highlightHeadline(preview)}</p>
{#if dupes}
  <p class="why">{formatNumber(dupes)} already there.</p>
  <Segmented options={CHOICES} value={choice} label="Duplicates" onchange={(v) => (choice = v)} />
{/if}
<SkippedList list={preview.invalid} />

<style>
  .head {
    margin: 0 0 8px;
    font-weight: 600;
  }
  .why {
    margin: 6px 0;
    font-size: var(--text-hint);
    color: var(--dim);
  }
</style>
