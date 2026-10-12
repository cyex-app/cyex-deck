<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  interface Props {
    icon: IconName;
    text: string;
  }

  let { icon, text }: Props = $props();
</script>

<div class="empty">
  <Icon name={icon} size={28} />
  <p>{text}</p>
</div>

<style>
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--dim);
  }
  p {
    margin: 0;
  }
</style>
