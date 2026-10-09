<!-- Shown in the main window for a tab that's in its own window (1.51, 3.14). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import { label, tabs } from './tabs.svelte';

  let { tab }: { tab: Tab } = $props();
</script>

<div class="popped">
  <Icon name="popout" size={28} />
  <p><b>{label(tab)}</b> is open in its own window.</p>
  <div class="actions">
    <Button icon="popout" onclick={() => tabs.popOut(tab)}>Show it</Button>
    <Button variant="primary" icon="undo" onclick={() => tabs.dock(tab.id)}>Bring it back</Button>
  </div>
</div>

<style>
  .popped {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--muted);
  }
  p {
    margin: 0;
  }
  b {
    color: var(--text);
  }
  .actions {
    display: flex;
    gap: 8px;
  }
</style>
