<!-- V111-11 Whispers and Global mentions float over the window as a sheet, like Settings (user,
     2026/11/14). The frame is the shared Sheet. Escape (an empty field first), the x or a click
     outside closes it; keys behind it wait. Whispers can still pop out into their own window. -->
<script lang="ts">
  import Sheet from '../components/Sheet.svelte';
  import { GLOBAL_FEED } from '../features/chat/mentionsTab';
  import FeedView from '../features/chat/FeedView.svelte';
  import WhispersView from '../features/whispers/WhispersView.svelte';

  interface Props {
    kind: 'whispers' | 'mentions';
    onclose: () => void;
    onpopout: (login: string | null) => void;
    onsignin: () => void;
  }

  let { kind, onclose, onpopout, onsignin }: Props = $props();
</script>

<Sheet label={kind === 'whispers' ? 'Whispers' : 'Global mentions'} {onclose}>
  <div class="over">
    {#if kind === 'whispers'}
      <WhispersView sheet {onpopout} {onsignin} />
    {:else}
      <FeedView name={GLOBAL_FEED} />
    {/if}
  </div>
</Sheet>

<style>
  .over {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
  }
  /* The sheet's close button sits at the top right: the headers leave room for it. */
  .over :global(header) {
    padding-right: 40px;
  }
</style>
