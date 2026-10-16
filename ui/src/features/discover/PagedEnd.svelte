<!-- Bottom of a paged list: loads more as it comes into view, or says why it can't. -->
<script lang="ts">
  import { type Paged, nearView } from './paged.svelte';
  import Loading from '../../components/Loading.svelte';

  let { list, empty }: { list: Paged<{ id: string }>; empty: string } = $props();
</script>

{#if list.error}
  <p class="end">{list.error}</p>
{:else if list.loading}
  <Loading cards={list.items.length ? 0 : 8} minHeight="72px" />
{:else if !list.items.length}
  <p class="end">{empty}</p>
{:else if list.cursor && !list.canPage}
  <p class="end">Twitch shows more with a GQL sign-in.</p>
{:else if list.cursor}
  <div class="sentinel" use:nearView={() => void list.more()}></div>
{/if}

<style>
  .end {
    margin: 0;
    padding: 24px 0;
    text-align: center;
    color: var(--dim);
    font-size: 0.85rem;
  }
  .sentinel {
    height: 1px;
  }
</style>
