<!-- Twitch badges (1.40, 2.3). Unknown badges are skipped until their catalog loads. -->
<script lang="ts">
  import type { BadgeInfo } from '../../ipc/bindings/BadgeInfo';
  import { catalogs } from './catalog.svelte';

  type Ref = { set: string; version: string; info?: string };

  let { badges, channel }: { badges: Ref[]; channel: string } = $props();

  function hint(ref: Ref, b: BadgeInfo): string | undefined {
    if (ref.set === 'subscriber' || ref.set === 'founder') return ref.info ? `${ref.info} months` : undefined;
    return ref.info ?? (b.description && b.description !== b.title ? b.description : undefined);
  }
</script>

{#each badges as ref (ref.set)}
  {@const b = catalogs.badge(channel, ref.set, ref.version)}
  {#if b}
    <img
      class="badge"
      src={b.image1x}
      srcset="{b.image2x} 2x, {b.image4x} 4x"
      alt={b.title}
      data-tip={b.title}
      data-hint={hint(ref, b)}
    />
  {/if}
{/each}

<style>
  .badge {
    width: 18px;
    height: 18px;
    margin-right: 3px;
    vertical-align: -3px;
  }
</style>
