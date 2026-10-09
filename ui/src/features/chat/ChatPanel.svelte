<!-- A chat popped out into its own window (1.51). Route: `chat/<login>/<account id|anon>`. -->
<script lang="ts">
  import ChatView from './ChatView.svelte';

  let { route }: { route: string; popout?: boolean } = $props();

  // The route never changes for a window.
  // svelte-ignore state_referenced_locally
  const [, login = '', who = 'anon'] = route.split('/');
</script>

<ChatView {login} account={who === 'anon' ? null : who} popout />
