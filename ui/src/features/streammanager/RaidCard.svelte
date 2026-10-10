<!-- 1.35 Start a raid to another channel. Twitch counts down 90 s before it goes, and it
     can be canceled until then. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { smIpc } from '../../ipc/streammanager';
  import { toast } from '../../lib/toast.svelte';
  import Card from './Card.svelte';

  let target = $state('');
  let raiding = $state<string | null>(null);
  let busy = $state(false);

  const login = $derived(target.trim().replace(/^@/, '').toLowerCase());

  function raid() {
    if (!login) return;
    busy = true;
    smIpc
      .raid(login)
      .then(() => {
        raiding = login;
        target = '';
      })
      .catch((e) => toast.error(String(e)))
      .finally(() => (busy = false));
  }

  function cancel() {
    busy = true;
    smIpc
      .unraid()
      .then(() => {
        raiding = null;
        toast.info('Raid canceled.');
      })
      .catch((e) => toast.error(String(e)))
      .finally(() => (busy = false));
  }
</script>

<Card title="Raid">
  {#if raiding}
    <p class="note">Raiding {raiding}. Twitch sends everyone over in about 90 seconds.</p>
    <div class="line"><Button small variant="danger" disabled={busy} onclick={cancel}>Cancel raid</Button></div>
  {:else}
    <form
      class="line"
      onsubmit={(e) => {
        e.preventDefault();
        raid();
      }}
    >
      <input class="field grow" placeholder="Channel name" bind:value={target} />
      <Button small type="submit" disabled={!login || busy}>Raid</Button>
    </form>
  {/if}
</Card>

<style>
  .grow {
    flex: 1;
  }
</style>
