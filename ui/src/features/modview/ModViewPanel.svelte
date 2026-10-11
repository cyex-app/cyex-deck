<!-- The mod view as a pop-out window (1.46, 1.47). The route is `modview/<login>`; the
     moderating account comes from the open chats' roles, live (V102-21): unmodded, the
     view goes; modded again, it's back. -->
<script lang="ts">
  import ModView from './ModView.svelte';
  import { mods } from './moderation.svelte';

  let { route }: { route: string } = $props();
  const login = $derived(route.split('/')[1] ?? '');
  const account = $derived(mods.moderating(login));
</script>

{#if account}
  {#key account}<ModView {login} {account} />{/key}
{:else}
  <p class="gone">You're not moderating {login} right now. Open its chat with your mod account first.</p>
{/if}

<style>
  .gone {
    padding: 24px;
    color: var(--muted);
    font-size: 0.9rem;
  }
</style>
