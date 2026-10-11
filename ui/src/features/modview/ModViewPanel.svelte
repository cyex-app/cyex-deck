<!-- The mod view (1.46, 1.47) for the route `modview/<login>`: in its own window, or in
     the main window next to the tabs (V102-16), where it pops out, or inside a tab (V102-26),
     where it pops out and closes. The moderating account comes from the roles, live
     (V102-21): unmodded, the view goes; modded again, it's back. -->
<script lang="ts">
  import ModView from './ModView.svelte';
  import { mods } from './moderation.svelte';

  let { route, onpopout, onclose }: { route: string; popout?: boolean; onpopout?: () => void; onclose?: () => void } = $props();
  const login = $derived(route.split('/')[1] ?? '');
  const account = $derived(mods.moderating(login));
</script>

{#if account}
  {#key account}<ModView {login} {account} {onpopout} {onclose} />{/key}
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
