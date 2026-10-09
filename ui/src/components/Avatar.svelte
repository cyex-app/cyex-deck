<!-- Profile picture with a letter fallback when there's none or it fails to load. -->
<script lang="ts">
  interface Props {
    src: string | null | undefined;
    name: string;
    size?: number;
  }

  let { src, name, size = 24 }: Props = $props();

  let failed = $state(false);

  $effect(() => {
    void src;
    failed = false;
  });
</script>

{#if src && !failed}
  <img class="avatar" {src} alt="" width={size} height={size} loading="lazy" decoding="async" referrerpolicy="no-referrer" onerror={() => (failed = true)} />
{:else}
  <span class="avatar letter" style:width="{size}px" style:height="{size}px" style:font-size="{Math.round(size * 0.45)}px">
    {name.slice(0, 1).toUpperCase()}
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
