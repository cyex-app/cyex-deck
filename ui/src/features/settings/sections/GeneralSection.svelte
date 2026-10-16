<!-- General (2.1): chat text, where chat sits, and the app itself. One chat font list: bundled
     fonts, font files the user added and fonts installed on this PC (V101-10, V105-13). -->
<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';

  import Icon from '../../../components/Icon.svelte';
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Stepper from '../../../components/Stepper.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { ChatPosition } from '../../../ipc/bindings/ChatPosition';
  import { fontsIpc } from '../../../ipc/theme';
  import { updateIpc } from '../../../ipc/update';
  import { formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { checkText } from '../../update/updateText';
  import FontPicker from '../FontPicker.svelte';
  import ResetSection from '../ResetSection.svelte';
  import { errorText } from '../../../lib/errors';

  const g = $derived(settings.v.general);

  const POSITIONS: { value: ChatPosition; label: string }[] = [
    { value: 'left', label: 'Left' },
    { value: 'right', label: 'Right' },
    { value: 'top', label: 'Top' },
    { value: 'bottom', label: 'Bottom' },
  ];

  const pick = (family: string) => void settings.patch('general', { chatFontFamily: family });

  async function addFont() {
    try {
      const font = await fontsIpc.add();
      if (font) toast.success(`Added ${font.name}.`);
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  function removeFont(name: string) {
    fontsIpc.remove(name).catch((e) => toast.error(errorText(e)));
  }

  // V105-20 "Check now": the answer sits next to the button.
  let checking = $state(false);
  let checked = $state('');
  let version = $state('');
  $effect(() => {
    void getVersion()
      .then((v) => (version = v))
      .catch(() => {});
  });

  async function checkNow() {
    checking = true;
    checked = '';
    try {
      checked = checkText(await updateIpc.check(), version);
    } catch {
      checked = checkText({ kind: 'failed' });
    } finally {
      checking = false;
    }
  }
</script>

<h2>General</h2>

<h3 class="label">Chat text</h3>
<Row setting="general.chatFontFamily general.fonts" label="Font" hint="Fonts that come with the app, your font files, and fonts on this PC." stacked>
  <div class="font-row">
    <FontPicker value={g.chatFontFamily} onpick={pick} onremove={removeFont} />
    <button class="font add" onclick={addFont}><Icon name="plus" size={13} />Add a font file</button>
  </div>
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

<h3 class="label">Layout</h3>
<Row setting="general.chatPosition" label="Chat position" hint="Which side of the player or a clip the chat sits on.">
  <Segmented
    label="Chat position"
    options={POSITIONS}
    value={g.chatPosition}
    onchange={(v) => settings.patch('general', { chatPosition: v })}
  />
</Row>

<h3 class="label">App</h3>
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

<h3 class="label">Updates</h3>
<Row setting="general.updates.check" label="Check for updates" hint="At startup and every 12 hours.">
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
<Row setting="general:checkNow" label={version ? `Version ${version}` : 'Version'} hint="Look for a newer one right away.">
  {#if checked}<span class="checked">{checked}</span>{/if}
  <Button small disabled={checking} onclick={() => void checkNow()}>{checking ? 'Checking…' : 'Check now'}</Button>
</Row>
<p class="privacy">Checks GitHub for new versions. Sends nothing about you or your accounts.</p>

<ResetSection partition="general" />

<style>
  h3 {
    margin: 22px 0 2px;
  }
  .font-row {
    display: flex;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 8px;
  }
  .font {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    padding: 0 11px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.9rem;
    white-space: nowrap;
  }
  .add {
    border-style: dashed;
    font-family: var(--font-ui);
    font-size: 0.82rem;
  }
  .add:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .checked {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .privacy {
    margin: 8px 0 0;
    color: var(--dim);
    font-size: 0.8rem;
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
