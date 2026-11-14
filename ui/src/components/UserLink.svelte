<!-- V111-16 A person's name that opens their usercard in `channel`, like a name in chat. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { chatIpc } from '../ipc/chat';
  import { errorText } from '../lib/errors';
  import { toast } from '../lib/toast.svelte';

  let { channel, login, children }: { channel: string; login: string; children: Snippet } = $props();

  const open = () => chatIpc.openUsercard(channel, login).catch((e) => toast.error(errorText(e)));
</script>

<span
  class="user-link"
  role="link"
  tabindex="0"
  data-user={login}
  onclick={() => void open()}
  onkeydown={(e) => e.key === 'Enter' && void open()}>{@render children()}</span
>

<style>
  .user-link {
    cursor: pointer;
  }
  .user-link:hover,
  .user-link:focus-visible {
    text-decoration: underline;
  }
</style>
