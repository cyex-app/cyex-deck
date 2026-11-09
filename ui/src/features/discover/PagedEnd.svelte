<!-- Bottom of a paged list: loads more as it comes into view, or says why it can't. -->
<script lang="ts">
  import { type Paged, nearView } from './paged.svelte';
  import Button from '../../components/Button.svelte';
  import Loading from '../../components/Loading.svelte';

  /** `shown`: how many of the loaded items the page actually lists (some may be hidden). */
  let { list, empty, shown }: { list: Paged<{ id: string }>; empty: string; shown?: number } = $props();
</script>

{#if list.error}
  <p class="end" role="alert">{list.error}</p>
  <!-- V110-F84 A failed later page keeps its cursor, so it can be asked for again. -->
  {#if list.cursor && list.canPage}
    <div class="retry"><Button small onclick={() => void list.more()}>Retry</Button></div>
  {/if}
{:else if list.loading}
  <Loading cards={shown ?? list.items.length ? 0 : 8} minHeight="72px" />
{:else if !(shown ?? list.items.length) && !list.cursor}
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
  .retry {
    display: flex;
    justify-content: center;
    padding-bottom: 24px;
  }
  .sentinel {
    height: 1px;
  }
</style>
