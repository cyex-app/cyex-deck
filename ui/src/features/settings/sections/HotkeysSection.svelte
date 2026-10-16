<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import {
    ACTION_IDS,
    ACTIONS,
    type ActionId,
    bindingOf,
    comboFromEvent,
    isGlobal,
    rebind,
    recording,
    resetBinding,
  } from '../../../lib/hotkeys.svelte';
  import { modesIpc } from '../../../ipc/modes';
  import { toast } from '../../../lib/toast.svelte';
  import ResetSection from '../ResetSection.svelte';

  let editing = $state<ActionId | null>(null);
  /** Global combos another app already holds. */
  let failed = $state<string[]>([]);

  $effect(() => {
    void modesIpc.hotkeysFailed().then((f) => (failed = f));
    const un = modesIpc.onHotkeysFailed((f) => (failed = f));
    return () => void un.then((f) => f());
  });

  const local = ACTION_IDS.filter((id) => !isGlobal(id));
  const everywhere = ACTION_IDS.filter(isGlobal);

  function start(id: ActionId) {
    editing = id;
    recording.active = true;
  }

  function stop() {
    editing = null;
    recording.active = false;
  }

  async function onKey(e: KeyboardEvent) {
    if (!editing) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') return stop();
    const combo = e.key === 'Backspace' || e.key === 'Delete' ? '' : comboFromEvent(e);
    if (combo === null) return; // lone modifier, keep listening
    const id = editing;
    if (combo && isGlobal(id) && !/(^|\+)(Ctrl|Alt|Win)\+/.test(combo)) {
      toast.error('Hotkeys that work everywhere need Ctrl, Alt or Win.');
      return stop();
    }
    stop();
    const moved = await rebind(id, combo);
    if (moved) toast.info(`Removed it from "${ACTIONS[moved].label}".`);
  }

  $effect(() => () => stop());
</script>

<svelte:window onkeydown={onKey} />

<h2>Hotkeys</h2>
<p class="intro" data-setting="hotkeys.bindings">Click a shortcut, then press the new keys. Esc cancels, Backspace clears it.</p>

{#snippet binding(id: ActionId)}
  {@const combo = bindingOf(id)}
  <Row setting={`hotkey:${id}`} label={ACTIONS[id].label} hint={combo && isGlobal(id) && failed.includes(combo) ? 'Another app already uses these keys.' : undefined}>
    <button class="combo" class:listening={editing === id} onclick={() => start(id)}>
      {#if editing === id}
        Press keys…
      {:else if combo}
        {#each combo.split('+') as key, i (i)}<kbd>{key}</kbd>{/each}
      {:else}
        <span class="none">Not set</span>
      {/if}
    </button>
    <Button variant="ghost" small disabled={combo === ACTIONS[id].default} onclick={() => resetBinding(id)}
      >{ACTIONS[id].default ? 'Reset' : 'Clear'}</Button
    >
  </Row>
{/snippet}

{#each local as id (id)}{@render binding(id)}{/each}

<h3>Work everywhere</h3>
<p class="intro">These work even while a game or another app is in front.</p>
{#each everywhere as id (id)}{@render binding(id)}{/each}

<ResetSection partition="hotkeys" />

<style>
  h3 {
    margin: 22px 0 8px;
    font-size: 0.92rem;
  }
  .intro {
    margin: -6px 0 6px;
    color: var(--dim);
  }
  .combo {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 140px;
    height: 30px;
    padding: 0 8px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
  }
  .combo:hover {
    border-color: var(--dim);
  }
  .combo.listening {
    border-color: var(--accent-border);
    color: var(--accent-text);
  }
  kbd {
    padding: 1px 6px;
    background: var(--elevated-2);
    border: 1px solid var(--border-strong);
    border-radius: 2px;
    font: 600 0.75rem var(--font-mono);
    color: var(--text);
  }
  .none {
    color: var(--dim);
  }
</style>
