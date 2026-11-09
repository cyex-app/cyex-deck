<!-- A button for things viewers notice right away (an ad, a payout, ending a session) or that
     are hard to undo. V108-C21 The click opens a small bubble clear of the button; moving onto
     it and clicking (or Enter) does it. Escape, a click elsewhere or 3 s closes it. -->
<script lang="ts">
  import Button from './Button.svelte';
  import ConfirmCallout from './ConfirmCallout.svelte';
  import type { IconName } from './icons';

  interface Props {
    label: string;
    /** Shown in the bubble, e.g. "Click to run a 60s ad". */
    confirm: string;
    icon?: IconName;
    disabled?: boolean;
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    onconfirm: () => void;
    /** V107-R4 Bind to show a warning while the bubble waits. */
    armed?: boolean;
  }

  let { label, confirm, icon, disabled = false, variant = 'secondary', onconfirm, armed = $bindable(false) }: Props = $props();

  let anchor = $state<HTMLElement>();
  let pressX = $state<number>();

  function click(e: MouseEvent) {
    if (armed) return;
    anchor = e.currentTarget as HTMLElement;
    // Keyboard clicks have no pointer position.
    pressX = e.detail === 0 ? undefined : e.clientX;
    armed = true;
  }

  function done() {
    armed = false;
    anchor?.focus({ preventScroll: true });
  }

  $effect(() => {
    if (disabled) armed = false;
  });
</script>

<Button small {icon} {disabled} {variant} onclick={click}>{label}</Button>
{#if armed && anchor}
  <ConfirmCallout
    {anchor}
    {pressX}
    label={confirm}
    onconfirm={() => {
      armed = false;
      // V110-F81 The bubble held focus and is gone now: back to the button, not the page body.
      anchor?.focus({ preventScroll: true });
      onconfirm();
    }}
    ondismiss={done}
  />
{/if}
