<!-- V107-Mk Chat identity next to the message box, as on twitch.tv: badge, name color and
     smiley style for this account in this channel. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import IdentityPanel from './IdentityPanel.svelte';

  let { account, login }: { account: string; login: string } = $props();

  let open = $state(false);
</script>

<div class="wrap">
  <button
    class="trigger"
    class:on={open}
    aria-label="Chat identity"
    aria-expanded={open}
    use:tooltip={{ label: 'Chat identity', hint: 'Badge, name color and smileys.' }}
    onclick={() => (open = !open)}
  >
    <Icon name="badge" size={16} />
  </button>
  <Popover bind:open align="end" side="above" label="Chat identity">
    <IdentityPanel {account} {login} />
  </Popover>
</div>

<style>
  .wrap {
    position: relative;
    display: flex;
    /* Centred in the box like the emote button next to it. */
    margin-bottom: 1px;
  }
  /* A narrow chat keeps its room for typing; the account chip still opens this. */
  @container chat (max-width: 260px) {
    .wrap {
      display: none;
    }
  }
  .trigger {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .trigger:hover,
  .trigger.on {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
