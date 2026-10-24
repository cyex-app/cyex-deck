<!-- V108-7K Connects 7TV with a token copied from 7tv.app in the user's own browser, for when
     7TV's sign-in window won't do. Rust checks it with 7TV and keeps it only when it's this
     Twitch account's; it's never logged. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { cosmeticsIpc } from '../../ipc/cosmetics';
  import type { AccountView } from '../../ipc/bindings/AccountView';
  import { copy } from '../../lib/clipboard';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import ConsentText from '../cosmetics/ConsentText.svelte';
  import { CONSOLE_LINE, looksLikeToken } from './sevenTvPaste';

  let { account, onclose }: { account: AccountView; onclose: () => void } = $props();

  let token = $state('');
  let busy = $state(false);
  let refused = $state('');
  /** 7TV has no yes from this account yet: Connect gives it, with the consent shown. */
  const consent = $derived(!account.seventv.consent);

  function open() {
    cosmeticsIpc.open7tv().catch((e) => toast.error(errorText(e)));
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    refused = '';
    if (!looksLikeToken(token)) {
      refused = 'That doesn’t look like a 7TV token.';
      return;
    }
    busy = true;
    try {
      if (consent) await cosmeticsIpc.allow(account.userId);
      await cosmeticsIpc.paste(account.userId, token);
      token = '';
      toast.success('7TV connected.');
      onclose();
    } catch (err) {
      refused = errorText(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="paste" role="group" aria-label="Paste a 7TV token">
  <div class="head">
    <strong>Paste a 7TV token</strong>
    <span>For when the 7TV window won’t sign you in.</span>
  </div>
  <ol>
    <li>
      <span class="n" aria-hidden="true">1</span>
      <span>Sign in on 7tv.app in your browser.</span>
      <Button small onclick={open}>Open 7tv.app</Button>
    </li>
    <li>
      <span class="n" aria-hidden="true">2</span>
      <span>In that tab, open the browser console (F12) and run:</span>
      <div class="code">
        <code>{CONSOLE_LINE}</code>
        <Button small variant="ghost" icon="copy" onclick={() => void copy(CONSOLE_LINE, 'Command')}>Copy</Button>
      </div>
    </li>
    <li>
      <span class="n" aria-hidden="true">3</span>
      <form class="way" onsubmit={submit}>
        <input
          class="field mono"
          type="password"
          placeholder="Paste the token"
          aria-label="7TV token"
          aria-invalid={refused ? 'true' : undefined}
          autocomplete="off"
          spellcheck="false"
          data-autofocus
          bind:value={token}
          oninput={() => (refused = '')}
        />
        <Button type="submit" small variant="primary" disabled={!token.trim() || busy}>
          {busy ? 'Checking…' : consent ? 'Allow and connect' : 'Connect'}
        </Button>
      </form>
    </li>
  </ol>
  {#if refused}<p class="refused" role="alert">{refused}</p>{/if}
  <p class="safe">Only paste this into cyex deck. Anyone with it can change your 7TV emotes.</p>
  {#if consent}<div class="consent"><ConsentText name={account.displayName} /></div>{/if}
  <div class="end"><Button small variant="ghost" onclick={onclose}>Not now</Button></div>
</div>

<style>
  .paste {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 2px 0 8px;
    padding: 10px 12px;
    background: var(--bg);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius);
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .head span,
  li > span:not(.n),
  .safe {
    color: var(--muted);
    font-size: 0.82rem;
  }
  ol {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--muted);
    font-size: 0.82rem;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 8px;
  }
  li > span:not(.n) {
    flex: 1;
    min-width: 0;
  }
  .n {
    flex: none;
    width: 14px;
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
  .code {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1 1 calc(100% - 22px);
    margin-left: 22px;
    min-width: 0;
  }
  code {
    flex: 1;
    min-width: 0;
    padding: 5px 8px;
    overflow-x: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 0.8rem;
    white-space: nowrap;
    user-select: all;
  }
  .way {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .way input {
    flex: 1;
    min-width: 0;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .refused {
    margin: 0;
    color: var(--red);
    font-size: 0.82rem;
  }
  .safe {
    margin: 0;
  }
  .consent :global(p) {
    font-size: 0.82rem;
  }
  .consent :global(p:last-child) {
    margin-bottom: 0;
  }
  .end {
    display: flex;
    justify-content: flex-end;
  }
</style>
