<!-- Add an account by pasting a token (1.4). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Row from '../../components/Row.svelte';
  import { accountsIpc } from '../../ipc/accounts';
  import { toast } from '../../lib/toast.svelte';

  let text = $state('');
  let busy = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    try {
      const name = await accountsIpc.addToken(text);
      text = '';
      toast.info(`Added ${name}.`);
    } catch (err) {
      toast.error(String(err));
    } finally {
      busy = false;
    }
  }
</script>

<form onsubmit={submit}>
  <Row
    label="Paste a token"
    hint="Chatterino logins, oauth: tokens and tokens that never expire all work. Tokens from Twitch's own apps unlock the GQL tools."
    stacked
  >
    <input
      class="field"
      type="password"
      placeholder="Token"
      autocomplete="off"
      spellcheck="false"
      bind:value={text}
    />
    <Button type="submit" disabled={!text.trim() || busy}>{busy ? 'Checking…' : 'Add'}</Button>
  </Row>
</form>

<style>
  .field {
    flex: 1;
    min-width: 200px;
    font-family: var(--font-mono);
  }
</style>
