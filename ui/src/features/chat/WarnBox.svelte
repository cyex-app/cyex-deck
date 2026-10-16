<!-- M10 The reason box a line's Warn button opens: one at a time, Enter sends, Escape or a
     click outside closes, and focus goes back to where it was. -->
<script lang="ts">
  import { focusScope } from '../../lib/focusScope';
  import { layer } from '../../lib/escape';
  import { MAX_REASON, warnCommand, warnSpot } from './warnBox';

  interface Props {
    login: string;
    /** The warn button's box on screen. */
    anchor: { left: number; top: number; bottom: number };
    /** Runs the command; true when it went through. */
    send: (command: string) => Promise<boolean>;
    onclose: () => void;
  }

  let { login, anchor, send, onclose }: Props = $props();

  let box = $state<HTMLElement>();
  let reason = $state('');
  let busy = $state(false);
  let spot = $state({ left: 0, top: 0 });

  const command = $derived(warnCommand(login, reason));

  $effect(() => {
    if (!box) return;
    const r = box.getBoundingClientRect();
    spot = warnSpot(anchor, r, { width: window.innerWidth, height: window.innerHeight });
    const off = focusScope(box, { trap: true });
    const gone = layer('popover', onclose);
    return () => {
      gone();
      off();
    };
  });

  async function go(e: SubmitEvent) {
    e.preventDefault();
    if (!command || busy) return;
    busy = true;
    const ok = await send(command);
    busy = false;
    if (ok) onclose();
  }

  function outside(e: PointerEvent) {
    if (box && !box.contains(e.target as Node)) onclose();
  }
</script>

<svelte:window onpointerdown={outside} />

<form
  class="warnbox"
  bind:this={box}
  style:left="{spot.left}px"
  style:top="{spot.top}px"
  role="group"
  aria-label="Warn {login}"
  onsubmit={go}
>
  <input class="field" data-autofocus maxlength={MAX_REASON} placeholder="Reason" aria-label="Reason" bind:value={reason} />
  <button class="send" disabled={!command || busy}>Warn</button>
  {#if !command}<span class="hint">A warning needs a reason.</span>{/if}
</form>

<style>
  .warnbox {
    position: fixed;
    z-index: 50;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    width: 280px;
    padding: 6px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
  }
  .field {
    flex: 1;
    min-width: 0;
  }
  .send {
    height: 30px;
    padding: 0 12px;
    background: var(--elevated-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--text);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .send:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .hint {
    flex-basis: 100%;
    color: var(--dim);
    font-size: 0.78rem;
  }
</style>
