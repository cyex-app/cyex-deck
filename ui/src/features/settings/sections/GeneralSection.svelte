<!-- General (2.1): chat text, where chat sits, and the app itself. The chat font comes from
     a fixed list of 8 (design/fonts.ts). -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';

  import Icon from '../../../components/Icon.svelte';
  import Row from '../../../components/Row.svelte';
  import PositionPicker from '../../../components/PositionPicker.svelte';
  import Stepper from '../../../components/Stepper.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { ChatPosition } from '../../../ipc/bindings/ChatPosition';
  import { formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { MAX_HIDDEN_TAGS, withoutHidden, withTag } from '../../discover/hidden';
  import FontPicker from '../FontPicker.svelte';
  import ResetSection from '../ResetSection.svelte';

  const g = $derived(settings.v.general);
  const b = $derived(settings.v.browse);

  // V108-N05
  let newTag = $state('');
  function addTag() {
    const next = withTag(b.hiddenTags, newTag);
    newTag = '';
    if (next.length !== b.hiddenTags.length) void settings.patch('browse', { hiddenTags: next });
  }

  const POSITIONS: { value: ChatPosition; label: string }[] = [
    { value: 'left', label: 'Left' },
    { value: 'right', label: 'Right' },
    { value: 'top', label: 'Top' },
    { value: 'bottom', label: 'Bottom' },
  ];

  const pick = (family: string) => void settings.patch('general', { chatFontFamily: family });
</script>

<SectionHead title="General" level={2} />

<SectionHead title="Chat text">The font used in every chat.</SectionHead>
<Row setting="general.chatFontFamily" label="Font">
  <FontPicker value={g.chatFontFamily} onpick={pick} />
</Row>
<Row setting="general.chatFontSize" label="Size">
  <Stepper
    label="Chat font size"
    value={g.chatFontSize}
    min={9}
    max={32}
    unit="px"
    onchange={(v) => settings.patch('general', { chatFontSize: v })}
  />
</Row>
<Row setting="general.pinnedSizeStep" label="Pinned messages" hint="How much larger than chat text.">
  <Stepper
    label="Pinned message size"
    value={g.pinnedSizeStep}
    min={0}
    max={8}
    unit="px larger"
    onchange={(v) => settings.patch('general', { pinnedSizeStep: v })}
  />
</Row>
<div class="preview">
  <span class="time">{formatTime(new Date())}</span>
  <b>seryxx</b>: this is how chat will look
</div>

<SectionHead title="Layout">Where chat and panels sit.</SectionHead>
<Row setting="general.chatPosition" label="Chat position" hint="Which side of the player or a clip the chat sits on.">
  <PositionPicker
    kind="chat"
    label="Chat position"
    options={POSITIONS}
    value={g.chatPosition}
    onchange={(v) => settings.patch('general', { chatPosition: v })}
  />
</Row>

<SectionHead title="App" />
<Row setting="general.tooltips" label="Tooltips" hint="Short explanations when you hover buttons.">
  <Toggle label="Tooltips" checked={g.tooltips} onchange={(v) => settings.patch('general', { tooltips: v })} />
</Row>
<!-- V103-11 The wheel glides Chatterino's way (lib/smoothScroll.ts); switches at once. -->
<Row setting="general.smoothScroll" label="Smooth scrolling" hint="The mouse wheel glides instead of jumping, everywhere in the app.">
  <Toggle label="Smooth scrolling" checked={g.smoothScroll} onchange={(v) => settings.patch('general', { smoothScroll: v })} />
</Row>
<Row setting="general.minimizeToTray" label="Minimize to tray" hint="Minimizing hides cyex deck in the tray instead of the taskbar.">
  <Toggle
    label="Minimize to tray"
    checked={g.minimizeToTray}
    onchange={(v) => settings.patch('general', { minimizeToTray: v })}
  />
</Row>

<!-- V108-N05 Hidden in Home and Discover. Categories are hidden from their right-click menu. -->
<SectionHead title="Home and Discover" />
<Row
  setting="browse.hiddenCategories"
  label="Hidden categories"
  hint={b.hiddenCategories.length ? 'Left out of Home and Discover.' : 'None. Right-click a category in Discover and pick Hide category.'}
  stacked={b.hiddenCategories.length > 0}
>
  {#if b.hiddenCategories.length}
    <div class="hid">
      {#each b.hiddenCategories as c (c.id)}
        <button
          class="hide"
          aria-label="Show {c.name} again"
          onclick={() => void settings.patch('browse', { hiddenCategories: withoutHidden(b.hiddenCategories, c.id) })}
        >
          {c.name}<Icon name="x" size={12} />
        </button>
      {/each}
    </div>
  {/if}
</Row>
<Row
  setting="browse.hiddenTags"
  label="Hidden tags"
  hint="Live streams with one of these tags are left out of Home and Discover."
  stacked={b.hiddenTags.length > 0}
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      addTag();
    }}
  >
    <input
      class="field"
      maxlength="40"
      aria-label="Add a tag to hide"
      placeholder={b.hiddenTags.length >= MAX_HIDDEN_TAGS ? 'List is full' : 'Add a tag'}
      disabled={b.hiddenTags.length >= MAX_HIDDEN_TAGS}
      bind:value={newTag} />
  </form>
  {#if b.hiddenTags.length}
    <div class="hid">
      {#each b.hiddenTags as t (t)}
        <button
          class="hide"
          aria-label="Show {t} again"
          onclick={() => void settings.patch('browse', { hiddenTags: b.hiddenTags.filter((x) => x !== t) })}
        >
          {t}<Icon name="x" size={12} />
        </button>
      {/each}
    </div>
  {/if}
</Row>

<SectionHead title="Updates" />
<Row setting="general.updates.check" label="Check for updates" hint="At startup and every 8 hours.">
  <Toggle
    label="Check for updates"
    checked={g.updates.check}
    onchange={(v) => settings.patch('general', { updates: { ...g.updates, check: v } })}
  />
</Row>
<Row setting="general.updates.prereleases" label="Include pre-releases" hint="Test builds that come before a release.">
  <Toggle
    label="Include pre-releases"
    checked={g.updates.prereleases}
    onchange={(v) => settings.patch('general', { updates: { ...g.updates, prereleases: v } })}
  />
</Row>
<ResetSection partition="general" page="general" />

<style>
  .hid {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }
  .hide {
    display: flex;
    align-items: center;
    gap: 5px;
    min-height: 24px;
    padding: 1px 8px;
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.78rem;
  }
  .hide:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }
  .preview {
    margin: 4px 0 2px;
    padding: 10px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-family: var(--chat-font);
    font-size: var(--chat-size);
    line-height: 1.55;
  }
  .preview b {
    color: var(--accent-text);
  }
  .time {
    margin-right: 6px;
    font: 0.85em var(--font-mono);
    color: var(--dim);
  }
</style>
