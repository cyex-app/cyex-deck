<!-- V108-S1 Settings as a sheet over the window. The frame is the shared Sheet (V109-B15). -->
<script lang="ts">
  import Sheet from '../../components/Sheet.svelte';
  import SettingsView from './SettingsView.svelte';

  interface Props {
    section: string;
    onclose: () => void;
  }

  let { section = $bindable(), onclose }: Props = $props();
</script>

<Sheet label="Settings" {onclose} ownEscape>
  <SettingsView bind:section {onclose} />
</Sheet>
