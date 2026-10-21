<!-- Profile picture with a letter fallback when there's none or it fails to load, and a
     person when there's no letter either: a row is never blank. -->
<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    src: string | null | undefined;
    name: string;
    size?: number;
  }

  let { src, name, size = 24 }: Props = $props();

  const url = $derived(src?.trim() ?? '');
  /** The picture that failed. Kept by address, not reset by an effect, which ran after a
   *  quick failure and showed a broken (blank) picture again. */
  let failed = $state('');
  const letter = $derived([...name.trim()][0]?.toUpperCase() ?? '');
</script>

{#if url && failed !== url}
  <img class="avatar" src={url} alt="" width={size} height={size} loading="lazy" decoding="async" referrerpolicy="no-referrer" onerror={() => (failed = url)} />
{:else}
  <span class="avatar letter" style:width="{size}px" style:height="{size}px" style:font-size="{Math.round(size * 0.45)}px">
    {#if letter}{letter}{:else}<Icon name="user" size={Math.round(size * 0.55)} />{/if}
  </span>
{/if}

<style>
  .avatar {
    flex: none;
    display: block;
    border-radius: var(--radius);
    object-fit: cover;
  }
  .letter {
    display: grid;
    place-items: center;
    background: var(--elevated-2);
    color: var(--muted);
    font-weight: 700;
  }
</style>
