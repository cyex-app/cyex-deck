<!-- 1.26 The whisper box: Enter sends, Up recalls the last one, the emote menu with your
     Twitch and 7TV/BTTV/FFZ emotes, and Twitch's 500-character cap. -->
<script lang="ts">
  import { tick } from 'svelte';

  import IconButton from '../../components/IconButton.svelte';
  import { whispersIpc } from '../../ipc/whispers';
  import EmoteButton from '../chat/EmoteButton.svelte';
  import { errorText } from '../../lib/errors';
  import { settings } from '../../lib/settings.svelte';

  interface Props {
    to: string;
    account: string | null;
    disabled?: boolean;
  }

  let { to, account, disabled = false }: Props = $props();

  const MAX = 500;

  let box = $state<HTMLTextAreaElement>();
  let text = $state('');
  let error = $state('');
  let sending = $state(false);
  let last = '';
  const chars = $derived([...text].length);

  export function focus() {
    box?.focus();
  }

  async function send() {
    const t = text.trim();
    if (!t || sending) return;
    sending = true;
    error = '';
    try {
      await whispersIpc.send(to, t);
      last = t;
      text = '';
    } catch (e) {
      error = errorText(e);
    } finally {
      sending = false;
      await tick();
      box?.focus();
    }
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void send();
    } else if (e.key === 'ArrowUp' && !text && last) {
      e.preventDefault();
      text = last;
    }
  }

  async function insert(value: string) {
    const start = box?.selectionStart ?? text.length;
    const before = text.slice(0, start);
    const lead = before && !before.endsWith(' ') ? ' ' : '';
    text = `${before}${lead}${value} ${text.slice(start).replace(/^ /, '')}`;
    await tick();
    const caret = before.length + lead.length + value.length + 1;
    box?.setSelectionRange(caret, caret);
    box?.focus();
  }
</script>

{#if error}<div class="error">{error}</div>{/if}
<div class="input">
  <div class="field" class:over={chars > MAX}>
  <textarea
    bind:this={box}
    bind:value={text}
    rows="1"
    placeholder={disabled ? 'You can’t whisper them.' : `Whisper ${to}`}
    {disabled}
    onkeydown={keydown}
    spellcheck="true"
  ></textarea>
  {#if chars > MAX - 50}<span class="count" class:over={chars > MAX}>{MAX - chars}</span>{/if}
  <EmoteButton login="" {account} oninsert={insert} />
  </div>
  {#if settings.v.chat.sendButton}
    <IconButton
      icon="send"
      label="Send"
      hint="Enter"
      disabled={disabled || sending || !text.trim() || chars > MAX}
      onclick={() => {
        void send();
        box?.focus();
      }}
    />
  {/if}
</div>

<style>
  .error {
    padding: 6px 12px;
    color: var(--red);
    font-size: 12px;
  }
  .input {
    display: flex;
    align-items: flex-end;
    gap: 6px;
    margin: 0 10px 10px;
  }
  /* The same box as the chat input: emote button inside, send outside. */
  .field {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding-right: 2px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .field:focus-within {
    border-color: var(--accent-border);
  }
  .field.over,
  .field.over:focus-within {
    border-color: var(--red);
  }
  .field :global(.smile) {
    width: 30px;
    height: 30px;
    margin-bottom: 1px;
  }
  textarea {
    flex: 1 1 0;
    min-width: 0;
    min-height: 32px;
    max-height: 120px;
    padding: 6px 10px;
    resize: none;
    field-sizing: content;
    background: none;
    border: 0;
    outline: none;
    color: var(--text);
    font: var(--chat-size) / 1.45 var(--chat-font);
  }
  .count {
    flex: none;
    align-self: flex-end;
    padding: 0 4px 7px;
    color: var(--dim);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .count.over {
    color: var(--red);
  }
</style>
