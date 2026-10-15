<!-- V105-11 "Ask before uploading": one question for the whole paste or drop. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';

  interface Props {
    host: string;
    count: number;
    onyes: () => void;
    onno: () => void;
  }

  let { host, count, onyes, onno }: Props = $props();
</script>

<Dialog title={count === 1 ? 'Upload this image?' : `Upload ${count} images?`} onclose={onno}>
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
</style>
