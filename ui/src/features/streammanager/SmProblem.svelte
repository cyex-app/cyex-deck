<!-- V107-R36..R40 A stream manager panel that couldn't load: what went wrong and Retry.
     V111-47 When the cause is a missing sign-in, Retry can't help: the line to add it instead. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import NeedsSignIn from '../../components/NeedsSignIn.svelte';
  import { needsKind } from '../../lib/gate';
  import { useOwner } from './owner';

  let {
    text,
    onretry,
    account,
  }: { text: string; onretry: () => void; account?: string | null } = $props();

  const owner = useOwner();
  const need = $derived(needsKind(text));
  const who = $derived(account ?? owner.who?.accountId ?? owner.account);
</script>

<div class="problem" role="alert">
  <EmptyState icon={need ? 'lock' : 'alert'} {text}>
    {#if need}<NeedsSignIn kind={need} account={who} centered />{:else}<Button small onclick={onretry}>Retry</Button>{/if}
  </EmptyState>
</div>

<style>
  .problem {
    flex: 1;
    display: flex;
    padding: 16px 12px;
    text-align: center;
    overflow-wrap: anywhere;
  }
</style>
