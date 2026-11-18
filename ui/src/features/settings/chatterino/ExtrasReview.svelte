<!-- V111-60 What the rest of Chatterino's settings.json brings: ignores, never highlight, muted
     chats, filters and nicknames. Rows you have stay; what can't come says why. -->
<script lang="ts">
  import type { ExtrasPreview } from '../../../ipc/bindings/ExtrasPreview';
  import { formatNumber } from '../../../lib/format';
  import { extrasHeadline } from './importAll';
  import SkippedList from './SkippedList.svelte';

  interface Props {
    preview: ExtrasPreview;
  }

  let { preview }: Props = $props();
</script>

<p class="head">{extrasHeadline(preview)}</p>
{#if preview.already}<p class="why">{formatNumber(preview.already)} already there, yours stay.</p>{/if}
{#if preview.filters}<p class="why">Pick filters from a chat’s menu.</p>{/if}
<SkippedList list={preview.skipped} />

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
