<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import {
    ACTION_IDS,
    ACTIONS,
    type ActionId,
    actionFor,
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
  import SectionHead from '../SectionHead.svelte';

  let editing = $state<ActionId | null>(null);
  /** H1 The keys just pressed are another action's: pressing them again moves them here. */
  let clash = $state<{ id: ActionId; combo: string; owner: ActionId } | null>(null);
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
    clash = null;
    editing = id;
    recording.active = true;
  }

  function stop() {
    editing = null;
    clash = null;
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
    const owner = combo ? actionFor(combo) : undefined;
    if (owner && owner !== id && !(clash?.id === id && clash.combo === combo)) {
      // First press: say who has it. The same keys again take them.
      clash = { id, combo, owner };
      return;
    }
    const before = bindingOf(id);
    stop();
    const moved = await rebind(id, combo);
    if (moved) {
      toast.info(`Moved from “${ACTIONS[moved].label}”.`, {
        action: {
          label: 'Undo',
          run: () => void rebind(moved, combo).then(() => rebind(id, before)),
        },
      });
    }
  }

  $effect(() => () => stop());
</script>

<svelte:window onkeydown={onKey} />

<SectionHead title="Hotkeys" level={2} docs="hotkeys" setting="hotkeys.bindings">
  Click a shortcut, then press the new keys. Esc cancels, Backspace clears it.
</SectionHead>

{#snippet binding(id: ActionId)}
  {@const combo = bindingOf(id)}
  {@const taken = clash?.id === id ? clash : null}
  <Row
    setting={`hotkey:${id}`}
    label={ACTIONS[id].label}
    hint={taken
      ? `Used by ${ACTIONS[taken.owner].label}. Press again to move it.`
      : combo && isGlobal(id) && failed.includes(combo)
        ? 'Another app already uses these keys.'
        : undefined}
  >
    <button class="combo" class:listening={editing === id} class:clash={!!taken} onclick={() => start(id)}>
      {#if taken}
        {#each taken.combo.split('+') as key, i (i)}<kbd>{key}</kbd>{/each}
      {:else if editing === id}
        Press keys…
      {:else if combo}
        {#each combo.split('+') as key, i (i)}<kbd>{key}</kbd>{/each}
      {:else}
        <span class="none">Not set</span>
      {/if}
    </button>
    <Button variant="ghost" small disabled={combo === ACTIONS[id].default} onclick={() => resetBinding(id)}
      >Reset</Button
    >
  </Row>
{/snippet}

{#each local as id (id)}{@render binding(id)}{/each}

<SectionHead title="Work everywhere">These work even while a game or another app is in front.</SectionHead>
{#each everywhere as id (id)}{@render binding(id)}{/each}

<ResetSection partition="hotkeys" />

<style>
  .combo {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    width: 200px;
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
  .combo.clash {
    border-color: var(--yellow);
  }
  .combo.clash kbd {
    border-color: var(--yellow);
  }
  kbd {
    padding: 1px 6px;
    background: var(--elevated-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    font: 600 0.75rem var(--font-mono);
    color: var(--text);
  }
  .none {
    color: var(--dim);
  }
</style>
