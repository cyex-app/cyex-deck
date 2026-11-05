<!-- V108-C40 The chat box while this account can't talk here: a timeout counting down, a ban
     with its reason and the unban request, or a warning to acknowledge. Takes the message
     box's place; the box comes back by itself when the lock ends. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import { formatDate, formatNumber } from '../../lib/format';
  import { left, REQUEST_MAX, ticking, title, unbanLine, type Lock } from './chatLock';
  import type { Standing } from './standing.svelte';

  interface Props {
    lock: Lock;
    standing: Standing;
  }

  let { lock, standing }: Props = $props();

  let asking = $state(false);
  let text = $state('');
  let field = $state<HTMLTextAreaElement>();
  let opener = $state<HTMLElement>();
  let status = $state<HTMLElement>();
  const uid = $props.id();

  /** V108-F60b The box above hands its focus here when the lock replaces it. */
  export function focus() {
    status?.focus();
  }

  const now = $derived(standing.now);
  const unban = $derived(lock.kind === 'ban' ? lock.unban : null);
  const waitAt = $derived(unban?.state === 'wait' ? unban.at : null);
  // V108-F57a The countdowns tick in aria-hidden spans: the status itself stays still.
  const line = $derived(unban && unban.state !== 'wait' ? unbanLine(unban, now) : '');
  const chars = $derived([...text].length);
  /** "since 2026/11/04". V108-C40e Never who did it: Twitch doesn't tell the account. */
  const meta = $derived(lock.kind === 'ban' && lock.since ? `since ${formatDate(new Date(lock.since))}` : '');

  // A countdown ticks once a second; nothing ticks otherwise.
  $effect(() => (lock.kind === 'timeout' || unban?.state === 'wait' ? standing.tick() : undefined));

  // The form goes when a request can't go out anymore (sent, or the ban ended).
  $effect(() => {
    if (asking && unban?.state !== 'ready') asking = false;
  });

  async function ask() {
    asking = true;
    await tick();
    field?.focus();
  }

  async function close() {
    asking = false;
    text = '';
    await tick();
    opener?.querySelector('button')?.focus();
  }

  /** V108-F57b Focus lands on the status once the control that had it is gone; a click elsewhere stays. */
  async function settle() {
    await tick();
    const a = document.activeElement;
    if (!a || a === document.body) status?.focus();
  }

  async function send() {
    if (!text.trim() || chars > REQUEST_MAX || standing.busy) return;
    if (await standing.requestUnban(text.trim())) {
      text = '';
      asking = false;
      await settle();
    }
  }

  async function acknowledge() {
    await standing.acknowledge();
    if (!standing.error) await settle();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      void close();
    } else if (e.key === 'Enter' && e.ctrlKey) {
      e.preventDefault();
      void send();
    }
  }
</script>

<div class="lock {lock.kind}" role="status" tabindex="-1" bind:this={status}>
  <div class="head">
    <Icon name={lock.kind === 'warn' ? 'alert' : lock.kind === 'ban' ? 'ban' : 'clock'} size={14} />
    <b>{title(lock)}{#if lock.kind === 'timeout'}<span aria-hidden="true">&nbsp;· {ticking(lock, now)}</span>{/if}</b>
  </div>

  {#if lock.kind === 'warn'}
    {#if lock.warning.reason}<p class="reason">{lock.warning.reason}</p>{/if}
    {#if lock.warning.rules.length}
      <ul class="rules" aria-label="Rules cited">
        {#each lock.warning.rules as rule (rule)}<li>{rule}</li>{/each}
      </ul>
    {/if}
    <p class="hint">Acknowledge it to chat again.</p>
    <div class="actions">
      <Button variant="primary" small disabled={standing.busy} onclick={() => void acknowledge()}>Acknowledge</Button>
    </div>
  {:else}
    {#if lock.reason}<p class="reason">{lock.reason}</p>{/if}
    {#if meta}<p class="hint">{meta}</p>{/if}
    {#if unban}
      {#if asking}
        <div class="form">
          <textarea
            bind:this={field}
            bind:value={text}
            rows="3"
            aria-label="Why should you be unbanned?"
            aria-describedby="{uid}-count"
            placeholder="Why should you be unbanned?"
            maxlength={REQUEST_MAX * 2}
            {onkeydown}
          ></textarea>
          <div class="actions">
            <span id="{uid}-count" class="count" class:over={chars > REQUEST_MAX}>{formatNumber(chars)}/{formatNumber(REQUEST_MAX)}</span>
            <Button variant="ghost" small onclick={() => void close()}>Cancel</Button>
            <Button variant="primary" small disabled={standing.busy || !text.trim() || chars > REQUEST_MAX} onclick={() => void send()}>Send</Button>
          </div>
        </div>
      {:else}
        {#if line}<p class="hint">{line}</p>{/if}
        {#if waitAt !== null}<p class="hint">Available in <span aria-hidden="true">{left(waitAt, now)}</span></p>{/if}
        {#if unban.state === 'ready' || unban.state === 'wait'}
          <div class="actions">
            <span bind:this={opener} class="opener">
              <Button small disabled={unban.state !== 'ready'} onclick={() => void ask()}>Request unban</Button>
            </span>
          </div>
        {/if}
      {/if}
    {/if}
  {/if}
  {#if standing.error}<p class="error" role="alert">{standing.error}</p>{/if}
</div>

<style>
  .lock {
    --tint: var(--yellow);
    /* V108-C40b Takes the message box's place in the row; a one-liner is as tall as the box. */
    flex: 1 1 0;
    min-width: 0;
    min-height: 32px;
    box-sizing: border-box;
    justify-content: center;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 7px 9px;
    background: color-mix(in srgb, var(--tint) 12%, var(--surface));
    border: 1px solid color-mix(in srgb, var(--tint) 40%, var(--border-strong));
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .lock.ban {
    --tint: var(--red);
  }
  /* V108-F60c Focus is set by code (a lock replacing the box), so :focus, not :focus-visible. */
  .lock:focus {
    outline: 1px solid var(--focus-ring);
    outline-offset: 1px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .head :global(svg) {
    flex: none;
    color: var(--tint);
  }
  .head b {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  p {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .reason {
    color: var(--text);
  }
  .hint {
    color: var(--muted);
  }
  .hint {
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .rules {
    margin: 0;
    padding-left: 18px;
    color: var(--muted);
  }
  .actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
  }
  .opener {
    display: contents;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  textarea {
    width: 100%;
    resize: vertical;
    min-height: 56px;
    max-height: 160px;
    padding: 6px 8px;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    font: inherit;
  }
  textarea:focus-visible {
    outline: none;
    border-color: var(--accent-border);
  }
  .count {
    margin-right: auto;
    color: var(--dim);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  .count.over {
    color: var(--red);
  }
  .error {
    color: var(--red);
    font-size: 0.8rem;
  }
</style>
