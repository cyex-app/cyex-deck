<!-- Settings (spec 12). Opens in the main window or as its own window. -->
<script lang="ts">
  import type { Component } from 'svelte';

  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { IconName } from '../../components/icons';
  import AccountsSection from './sections/AccountsSection.svelte';
  import AppearanceSection from './sections/AppearanceSection.svelte';
  import AutomationsSection from './sections/AutomationsSection.svelte';
  import ChatSection from './sections/ChatSection.svelte';
  import CommandsSection from './sections/CommandsSection.svelte';
  import DataSection from './sections/DataSection.svelte';
  import EmotesSection from './sections/EmotesSection.svelte';
  import FiltersSection from './sections/FiltersSection.svelte';
  import GeneralSection from './sections/GeneralSection.svelte';
  import HighlightsSection from './sections/HighlightsSection.svelte';
  import HotkeysSection from './sections/HotkeysSection.svelte';
  import IgnoresSection from './sections/IgnoresSection.svelte';
  import MinerSection from './sections/MinerSection.svelte';
  import PlayerSection from './sections/PlayerSection.svelte';
  import RecordingsSection from './sections/RecordingsSection.svelte';
  import StreamManagerSection from './sections/StreamManagerSection.svelte';
  import WindowsSection from './sections/WindowsSection.svelte';

  interface Props {
    popout?: boolean;
    onpopout?: () => void;
    /** Open section id. Bindable so other views can jump to a section. */
    section?: string;
    /** Panel route, given to every pop-out; settings has no parameters. */
    route?: string;
  }

  let { popout = false, onpopout, section = $bindable('general') }: Props = $props();

  const SECTIONS: { id: string; label: string; icon: IconName; component: Component }[] = [
    { id: 'general', label: 'General', icon: 'sliders', component: GeneralSection },
    { id: 'accounts', label: 'Accounts', icon: 'user', component: AccountsSection },
    { id: 'appearance', label: 'Appearance', icon: 'palette', component: AppearanceSection },
    { id: 'chat', label: 'Chat', icon: 'chat', component: ChatSection },
    { id: 'highlights', label: 'Highlights', icon: 'star', component: HighlightsSection },
    { id: 'ignores', label: 'Ignores', icon: 'eyeOff', component: IgnoresSection },
    { id: 'filters', label: 'Filters', icon: 'filter', component: FiltersSection },
    { id: 'commands', label: 'Commands', icon: 'pencil', component: CommandsSection },
    { id: 'automations', label: 'Automations', icon: 'zap', component: AutomationsSection },
    { id: 'recordings', label: 'Recordings', icon: 'record', component: RecordingsSection },
    { id: 'emotes', label: 'Emotes', icon: 'smile', component: EmotesSection },
    { id: 'miner', label: 'Miner', icon: 'pickaxe', component: MinerSection },
    { id: 'streammanager', label: 'Stream manager', icon: 'radio', component: StreamManagerSection },
    { id: 'player', label: 'Player', icon: 'tv', component: PlayerSection },
    { id: 'hotkeys', label: 'Hotkeys', icon: 'keyboard', component: HotkeysSection },
    { id: 'windows', label: 'Windows', icon: 'layout', component: WindowsSection },
    { id: 'data', label: 'Backup and sharing', icon: 'archive', component: DataSection },
  ];

  const Section = $derived((SECTIONS.find((s) => s.id === section) ?? SECTIONS[0]).component);
</script>

<div class="settings">
  <nav>
    {#each SECTIONS as s (s.id)}
      <button class:on={section === s.id} onclick={() => (section = s.id)} title={s.label}>
        <Icon name={s.icon} size={15} />
        <span>{s.label}</span>
      </button>
    {/each}
    {#if !popout && onpopout}
      <div class="foot">
        <Button variant="ghost" small icon="popout" onclick={onpopout}>Pop out</Button>
      </div>
    {/if}
  </nav>
  <div class="content">
    <div class="inner"><Section /></div>
  </div>
</div>

<style>
  .settings {
    flex: 1;
    display: flex;
    min-height: 0;
    container-type: inline-size;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 210px;
    flex: none;
    padding: 14px 10px;
    border-right: 1px solid var(--border);
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-weight: 600;
    text-align: left;
    white-space: nowrap;
  }
  nav button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  nav button.on {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .foot {
    margin-top: auto;
  }
  .content {
    flex: 1;
    overflow-y: auto;
  }
  .inner {
    max-width: 680px;
    padding: 22px 28px 32px;
  }
  /* Narrow windows: icons only (3.10). */
  @container (max-width: 600px) {
    nav {
      width: 52px;
      padding-inline: 8px;
    }
    nav button span,
    .foot {
      display: none;
    }
    .inner {
      padding: 18px 16px 28px;
    }
  }
</style>
