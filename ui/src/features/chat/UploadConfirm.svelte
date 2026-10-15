<!-- V105-11 The first upload to a host asks once. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import { settings } from '../../lib/settings.svelte';

  interface Props {
    host: string;
    onyes: () => void;
    onno: () => void;
  }

  let { host, onyes, onno }: Props = $props();

  function yes() {
    const hosts = settings.v.uploader.trustedHosts;
    void settings.patch('uploader', { trustedHosts: [...hosts, host] });
    onyes();
  }
</script>

<Dialog title={`Upload to ${host}?`} onclose={onno}>
  <p>Your image goes to a site cyex deck doesn’t run. You may not be able to delete it.</p>
  {#snippet footer()}
    <Button variant="ghost" onclick={onno}>Cancel</Button>
    <Button variant="primary" onclick={yes}>Upload</Button>
  {/snippet}
</Dialog>

<style>
  p {
    margin: 0;
    color: var(--muted);
  }
</style>
