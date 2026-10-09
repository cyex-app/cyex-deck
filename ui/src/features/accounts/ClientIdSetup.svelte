<!-- First-run prompt for this app's Twitch client ID (D5). Saved to .env in the app's
     data folder, never in settings or exports. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { accountsIpc } from '../../ipc/accounts';
  import { systemIpc } from '../../ipc/system';
  import { toast } from '../../lib/toast.svelte';

  let { onsaved }: { onsaved?: () => void } = $props();

  let id = $state('');

  async function save(e: SubmitEvent) {
    e.preventDefault();
    try {
      await accountsIpc.setClientId(id);
      id = '';
      toast.info('Client ID saved.');
      onsaved?.();
    } catch (err) {
      toast.error(String(err));
    }
  }

  function openConsole() {
    systemIpc.openLink('https://dev.twitch.tv/console/apps').catch((e) => toast.error(String(e)));
  }
</script>

<div class="setup">
  <strong>{onsaved ? 'Change your Twitch client ID' : 'Add your Twitch client ID'}</strong>
  <p>
    Signing in with Twitch needs the client ID of your own app on dev.twitch.tv, with the client type set to Public.
    It stays on this PC.
  </p>
  <form onsubmit={save}>
    <input class="field" placeholder="Client ID" autocomplete="off" spellcheck="false" bind:value={id} />
    <Button type="submit" variant="primary" disabled={!id.trim()}>Save</Button>
    <Button variant="ghost" icon="popout" onclick={openConsole}>Open Twitch console</Button>
  </form>
</div>

<style>
  .setup {
    margin: 12px 0 18px;
    padding: 14px 16px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  p {
    margin: 6px 0 12px;
    font-size: 0.85rem;
    color: var(--muted);
  }
  form {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .field {
    flex: 1;
    min-width: 200px;
    font-family: var(--font-mono);
  }
</style>
