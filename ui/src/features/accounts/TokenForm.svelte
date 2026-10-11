<!-- V101-08 Add an account with a token someone already has, with the fields Chatterino's
     manual screen asks for (1.4). Only the token is needed; the rest is checked against it. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { accountsIpc } from '../../ipc/accounts';
  import type { ManualToken } from '../../ipc/bindings/ManualToken';
  import { toast } from '../../lib/toast.svelte';
  import { parseLoginLine } from './manual';

  let { ondone }: { ondone?: () => void } = $props();

  const EMPTY: ManualToken = { login: '', userId: '', clientId: '', token: '' };
  let f = $state<ManualToken>({ ...EMPTY });
  let busy = $state(false);

  /** A whole Chatterino login line pasted anywhere fills every field. */
  function onpaste(e: ClipboardEvent) {
    const line = parseLoginLine(e.clipboardData?.getData('text') ?? '');
    if (!line) return;
    e.preventDefault();
    f = { ...f, ...line };
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    try {
      const name = await accountsIpc.addManual($state.snapshot(f));
      f = { ...EMPTY };
      toast.info(`Added ${name}.`);
      ondone?.();
    } catch (err) {
      toast.error(String(err));
    } finally {
      busy = false;
    }
  }
</script>

<form class="manual" onsubmit={submit} {onpaste}>
  <p class="note">Only the token is needed. Whatever else you fill in has to match it. A Chatterino login line pasted here fills everything.</p>
  <label for="m-login">Username</label>
  <input id="m-login" class="field" autocomplete="off" spellcheck="false" bind:value={f.login} />
  <label for="m-id">User ID</label>
  <input id="m-id" class="field" autocomplete="off" spellcheck="false" inputmode="numeric" bind:value={f.userId} />
  <label for="m-client">Client ID</label>
  <input id="m-client" class="field mono" autocomplete="off" spellcheck="false" bind:value={f.clientId} />
  <label for="m-token">OAuth token</label>
  <input id="m-token" class="field mono" type="password" autocomplete="off" spellcheck="false" bind:value={f.token} />
  <div class="buttons">
    <Button type="submit" variant="primary" small disabled={!f.token.trim() || busy}>{busy ? 'Checking…' : 'Add account'}</Button>
    <Button type="button" variant="ghost" small onclick={() => (f = { ...EMPTY })}>Clear</Button>
  </div>
</form>

<style>
  .manual {
    display: grid;
    grid-template-columns: 100px minmax(0, 1fr);
    align-items: center;
    gap: 6px 12px;
    padding: 12px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .note {
    grid-column: 1 / -1;
    margin: 0 0 4px;
    color: var(--muted);
    font-size: 0.82rem;
  }
  label {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .buttons {
    grid-column: 2;
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }
</style>
