<!-- Joined-chat counter in the top bar (1.13, D7): the chats the account picked here has open.
     Turns yellow and names them when Twitch refused some (V105-05). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import { counterView } from './counter';

  const v = $derived(
    counterView(
      accounts.connections,
      accounts.current?.userId ?? '',
      (id) => accounts.list.find((a) => a.userId === id)?.displayName ?? id,
    ),
  );
</script>

{#if accounts.current}
  <span class="counter" class:warn={v.warn} use:tooltip={{ label: v.label, hint: v.hint }}>
    <Icon name={v.warn ? 'alert' : 'chat'} size={13} />
    {v.text}
  </span>
{/if}

<style>
  .counter {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    margin-right: 4px;
    border-radius: var(--radius);
    color: var(--dim);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
  }
  .counter:hover {
    background: var(--surface-hover);
    color: var(--muted);
  }
  .warn {
    color: var(--yellow);
  }
</style>
