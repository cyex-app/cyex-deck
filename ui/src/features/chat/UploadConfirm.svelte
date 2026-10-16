<!-- V105-11 "Ask before uploading": one question for the whole paste or drop.
     V1051-05 A thumbnail of each pasted picture (a dropped file shows its name); Return uploads. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';

  interface Props {
    host: string;
    count: number;
    /** Pasted pictures, for thumbnails. */
    pictures?: File[];
    /** File names of dropped ones. */
    names?: string[];
    onyes: () => void;
    onno: () => void;
  }

  let { host, count, pictures = [], names = [], onyes, onno }: Props = $props();

  const SHOWN = 4;
  let urls = $state<string[]>([]);
  $effect(() => {
    const made = pictures.slice(0, SHOWN).map((f) => URL.createObjectURL(f));
    urls = made;
    return () => made.forEach((u) => URL.revokeObjectURL(u));
  });

  // Focus sits on the dialog, so Return is caught here; a focused button keeps its own Return.
  function onkey(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.ctrlKey || e.shiftKey || e.target instanceof HTMLButtonElement) return;
    e.preventDefault();
    e.stopPropagation();
    onyes();
  }
</script>

<svelte:window onkeydown={onkey} />

<Dialog title={count === 1 ? 'Upload this image?' : `Upload ${count} images?`} onclose={onno}>
  {#if urls.length}
    <div class="thumbs">
      {#each urls as u (u)}<img src={u} alt="" />{/each}
      {#if count > urls.length}<span class="more">+{count - urls.length}</span>{/if}
    </div>
  {:else if names.length}
    <p class="names">{names.slice(0, SHOWN).join(', ')}{names.length > SHOWN ? ` +${names.length - SHOWN}` : ''}</p>
  {/if}
  <p>
    {count === 1 ? 'It goes' : 'They go'} to {host || 'your upload host'}, a site cyex deck doesn’t run. You may not be able
    to delete {count === 1 ? 'it' : 'them'}.
  </p>
  {#snippet footer()}
    <Button variant="ghost" onclick={onno}>Cancel</Button>
    <Button variant="primary" onclick={onyes}>Upload</Button>
  {/snippet}
</Dialog>

<style>
  p {
    margin: 0;
    color: var(--muted);
  }
  .thumbs {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 10px;
  }
  img {
    width: 64px;
    height: 64px;
    object-fit: cover;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .more {
    color: var(--dim);
  }
  .names {
    margin-bottom: 8px;
    overflow-wrap: anywhere;
    color: var(--text);
    font-size: 0.85rem;
  }
</style>
