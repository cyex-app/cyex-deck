<!-- The mod view as a pop-out window (1.46, 1.47). The route is `modview/<login>`; the
     moderating account is resolved from the open chats. -->
<script lang="ts">
  import { modIpc } from '../../ipc/moderation';
  import ModView from './ModView.svelte';

  let { route }: { route: string } = $props();
  const login = $derived(route.split('/')[1] ?? '');

  let account = $state<string | null>(null);
  let checked = $state(false);

  $effect(() => {
    checked = false;
    modIpc
      .moderating(login)
      .then((a) => (account = a))
      .catch(() => (account = null))
      .finally(() => (checked = true));
  });
</script>

{#if account}
  <ModView {login} {account} />
{:else if checked}
  <p class="gone">You're not moderating {login} right now. Open its chat with your mod account first.</p>
{/if}

<style>
  .gone {
    padding: 24px;
    color: var(--muted);
    font-size: 0.9rem;
  }
</style>
