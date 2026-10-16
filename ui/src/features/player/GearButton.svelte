<!-- The gear in a player's bar. It opens the same menu as a right-click on the picture
     (quality, audio only, speed, video stats and the rest), so there is one menu to learn. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { type MenuEntry, openMenu } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';

  let { menu }: { menu: () => MenuEntry[] } = $props();
</script>

<button
  class="ctl"
  aria-label="Settings"
  aria-haspopup="menu"
  use:tooltip={{ label: 'Settings', hint: 'Quality, speed and video stats. Right-click the picture for the same menu.' }}
  onclick={(e) => openMenu(e.currentTarget, menu(), 'end')}
>
  <Icon name="settings" size={17} />
</button>
