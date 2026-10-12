<!-- U5 Name and color for a new tab group, or for an existing one. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { GroupColor } from '../../ipc/bindings/GroupColor';
  import { groupDialog, type GroupDialogMode } from './dialog.svelte';
  import { COLOR_ORDER, GROUP_COLORS, nextColor } from './groupColors';
  import { tabs } from './tabs.svelte';

  let { mode }: { mode: GroupDialogMode } = $props();

  // svelte-ignore state_referenced_locally
  const existing = 'group' in mode ? tabs.group(mode.group) : undefined;

  let name = $state(existing?.name ?? '');
  let color = $state<GroupColor>(existing?.color ?? nextColor(tabs.groups.map((g) => g.color)));

  const close = () => (groupDialog.mode = null);

  async function save() {
    const trimmed = name.trim();
    if (existing) {
      await tabs.apply({ op: 'editGroup', group: existing.id, name: trimmed, color, collapsed: null });
    } else if ('tabs' in mode) {
      await tabs.apply({ op: 'newGroup', tabs: mode.tabs, name: trimmed, color });
    }
    close();
  }
</script>

<Dialog title={existing ? 'Edit group' : 'New group'} width={340} onclose={close}>
  <div class="form">
    <input
      class="field"
      maxlength="40"
      placeholder="Name (optional)"
      bind:value={name}
      onkeydown={(e) => e.key === 'Enter' && save()}
    />
    <div class="colors" role="radiogroup" aria-label="Color">
      {#each COLOR_ORDER as c (c)}
        <button
          class="swatch"
          class:on={c === color}
          role="radio"
          aria-checked={c === color}
          aria-label={GROUP_COLORS[c].name}
          style:--c={GROUP_COLORS[c].css}
          onclick={() => (color = c)}
          use:tooltip={GROUP_COLORS[c].name}
        ></button>
      {/each}
    </div>
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={close}>Cancel</Button>
    <Button variant="primary" onclick={save}>{existing ? 'Save' : 'Create group'}</Button>
  {/snippet}
</Dialog>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .field {
    width: 100%;
  }
  .colors {
    display: flex;
    gap: 8px;
  }
  .swatch {
    width: 24px;
    height: 24px;
    padding: 0;
    background: var(--c);
    border: 2px solid transparent;
    border-radius: 50%;
  }
  .swatch.on {
    box-shadow:
      0 0 0 2px var(--elevated),
      0 0 0 4px var(--c);
  }
</style>
