<!-- V111-47 One quiet line for a tool that's missing a sign-in: the lock flag, what it
     costs, and an Add button that goes to that account's own row in Settings → Accounts.
     `centered`: for an empty state. -->
<script lang="ts">
  import type { TokenKind } from '../ipc/bindings/TokenKind';
  import { gate } from '../lib/gate.svelte';
  import Button from './Button.svelte';
  import GateFlag from './GateFlag.svelte';

  let {
    kind,
    account = null,
    text,
    centered = false,
  }: {
    kind: TokenKind;
    /** The account that lacks the sign-in (the default one if left out). */
    account?: string | null;
    /** One short sentence after the flag, e.g. "Earlier actions need it." */
    text?: string;
    centered?: boolean;
  } = $props();
</script>

<span class="needs" class:centered>
  <GateFlag {kind} still />
  {#if text}<span class="text">{text}</span>{/if}
  <Button small onclick={() => gate.ask(kind, account)}>Add</Button>
</span>

<style>
  .needs {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 8px;
    min-width: 0;
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .centered {
    justify-content: center;
  }
  .text {
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
