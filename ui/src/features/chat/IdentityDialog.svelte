<!-- 1.40 Chat identity from the account chip: the same panel as the chat box's popover. -->
<script lang="ts">
  import Dialog from '../../components/Dialog.svelte';
  import IdentityPanel from './IdentityPanel.svelte';

  interface Props {
    account: string;
    login: string;
    onclose: () => void;
  }

  let { account, login, onclose }: Props = $props();
</script>

<Dialog title="Chat identity" width={400} {onclose}>
  <IdentityPanel {account} {login} />
</Dialog>
