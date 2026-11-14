<!-- V111-13 Usercard: one switch per thing a usercard shows. Off hides it and skips its lookup. -->
<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import { settings } from '../../../lib/settings.svelte';
  import { USERCARD_ITEMS } from '../registry/usercard';
  import ResetSection from '../ResetSection.svelte';
  import SectionHead from '../SectionHead.svelte';

  const show = $derived(settings.v.chat.usercard);
</script>

<SectionHead title="Usercard" level={2} page="usercard" />
<p class="intro">The buttons on a card are in Settings → Menus.</p>

{#each USERCARD_ITEMS as item (item.key)}
  <Row setting={`chat.usercard.${item.key}`} label={item.label} hint={item.hint}>
    <Toggle
      label={item.label}
      checked={show[item.key]}
      onchange={(v) => settings.patch('chat', { usercard: { [item.key]: v } })}
    />
  </Row>
{/each}

<ResetSection partition="chat" page="usercard" />

<style>
  .intro {
    margin: 0 0 8px;
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
