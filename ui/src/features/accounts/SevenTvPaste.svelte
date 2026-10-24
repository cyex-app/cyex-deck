<!-- V108-7K/7L Connects 7TV from the user's own browser, for when 7TV's sign-in window won't
     do: the "Send to cyex deck" bookmark (7L), or the token pasted by hand (7K, the fallback).
     Rust checks it with 7TV and keeps it only when it's this Twitch account's; never logged. -->
<script lang="ts">
  import { onDestroy } from 'svelte';
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
  /** V108-7L The port is open for the bookmark. */
  let waiting = $state(false);
  let refused = $state('');
  /** 7TV has no yes from this account yet: Connect gives it, with the consent shown. */
  const consent = $derived(!account.seventv.consent);

  onDestroy(() => {
    if (waiting) void cosmeticsIpc.linkStop();
  });

  function open() {
    cosmeticsIpc.open7tv().catch((e) => toast.error(errorText(e)));
  }

  async function copyBookmark() {
    try {
      await copy(await cosmeticsIpc.bookmarklet(), 'Bookmark');
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  function connected() {
    token = '';
    toast.success('7TV connected.');
    onclose();
  }

  async function wait() {
    if (waiting || busy) return;
    refused = '';
    waiting = true;
    try {
      if (consent) await cosmeticsIpc.allow(account.userId);
      await cosmeticsIpc.linkWait(account.userId);
      connected();
    } catch (err) {
      // '' = stopped.
      if (err !== '') refused = errorText(err);
    } finally {
      waiting = false;
    }
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
      connected();
    } catch (err) {
      refused = errorText(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="paste" role="group" aria-label="Connect 7TV from your browser">
  <div class="head">
    <strong>Connect 7TV from your browser</strong>
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
      <span>Add a bookmark and paste this as its address, then click it on 7tv.app.</span>
      <Button small variant="ghost" icon="copy" onclick={copyBookmark}>Copy bookmark</Button>
    </li>
    <li>
      <span class="n" aria-hidden="true">3</span>
      {#if waiting}
        <span class="live" aria-live="polite">Waiting for the bookmark…</span>
        <Button small variant="ghost" onclick={() => void cosmeticsIpc.linkStop()}>Stop</Button>
      {:else}
        <span>Wait for it here, then click the bookmark.</span>
        <Button small variant="primary" disabled={busy} onclick={wait}>{consent ? 'Allow and wait' : 'Wait for the bookmark'}</Button>
      {/if}
    </li>
  </ol>
  {#if refused}<p class="refused" role="alert">{refused}</p>{/if}
  <details>
    <summary>Or paste the token instead</summary>
    <ol>
      <li>
        <span>On 7tv.app, open the browser console (F12) and run:</span>
        <div class="code">
          <code>{CONSOLE_LINE}</code>
          <Button small variant="ghost" icon="copy" onclick={() => void copy(CONSOLE_LINE, 'Command')}>Copy</Button>
        </div>
      </li>
      <li>
        <form class="way" onsubmit={submit}>
          <input
            class="field mono"
            type="password"
            placeholder="Paste the token"
            aria-label="7TV token"
            aria-invalid={refused ? 'true' : undefined}
            autocomplete="off"
            spellcheck="false"
            bind:value={token}
            oninput={() => (refused = '')}
          />
          <Button type="submit" small variant="primary" disabled={!token.trim() || busy || waiting}>
            {busy ? 'Checking…' : consent ? 'Allow and connect' : 'Connect'}
          </Button>
        </form>
      </li>
    </ol>
  </details>
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
    flex-basis: 100%;
    min-width: 0;
  }
  .live {
    color: var(--text);
  }
  details {
    font-size: 0.82rem;
  }
  summary {
    width: fit-content;
    color: var(--muted);
    cursor: pointer;
  }
  summary:hover {
    color: var(--text);
  }
  details[open] summary {
    margin-bottom: 8px;
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
