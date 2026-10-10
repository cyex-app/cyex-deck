<!-- A button that needs a second click within 3 s, for things viewers notice right away
     (an ad, a payout, ending a session). -->
<script lang="ts">
  import Button from './Button.svelte';
  import type { IconName } from './icons';

  interface Props {
    label: string;
    /** Shown after the first click, e.g. "Click again to run a 60s ad". */
    confirm: string;
    icon?: IconName;
    disabled?: boolean;
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    onconfirm: () => void;
  }

  let { label, confirm, icon, disabled = false, variant = 'secondary', onconfirm }: Props = $props();

  let armed = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function click() {
    if (!armed) {
      armed = true;
      timer = setTimeout(() => (armed = false), 3000);
      return;
    }
    clearTimeout(timer);
    armed = false;
    onconfirm();
  }

  $effect(() => () => clearTimeout(timer));
</script>

<Button small {icon} {disabled} variant={armed ? 'danger' : variant} onclick={click}>{armed ? confirm : label}</Button>
