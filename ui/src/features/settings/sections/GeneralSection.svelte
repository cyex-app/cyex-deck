<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Stepper from '../../../components/Stepper.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { ChatPosition } from '../../../ipc/bindings/ChatPosition';
  import { formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import ResetSection from '../ResetSection.svelte';

  const g = $derived(settings.v.general);

  const FONTS = ['IBM Plex Sans', 'Inter', 'Segoe UI', 'Roboto', 'Arial', 'Verdana', 'Tahoma', 'Consolas', 'Cascadia Code'];
  const POSITIONS: { value: ChatPosition; label: string }[] = [
    { value: 'left', label: 'Left' },
    { value: 'right', label: 'Right' },
    { value: 'top', label: 'Top' },
    { value: 'bottom', label: 'Bottom' },
  ];
</script>

<h2>General</h2>

<Row label="Chat font" hint="Any font installed on this PC. Leave empty for the app font.">
  <input
    class="field"
    list="chat-fonts"
    placeholder="App font"
    value={g.chatFontFamily}
    onchange={(e) => settings.patch('general', { chatFontFamily: e.currentTarget.value })}
  />
  <datalist id="chat-fonts">
    {#each FONTS as font (font)}<option value={font}></option>{/each}
  </datalist>
</Row>

<Row label="Chat font size">
  <Stepper
    label="Chat font size"
    value={g.chatFontSize}
    min={9}
    max={32}
    unit="px"
    onchange={(v) => settings.patch('general', { chatFontSize: v })}
  />
</Row>

<div class="preview">
  <span class="time">{formatTime(new Date())}</span>
  <b>seryxx</b>: this is how chat will look
</div>

<Row label="Chat position" hint="Which side of the player or a clip the chat sits on.">
  <Segmented
    label="Chat position"
    options={POSITIONS}
    value={g.chatPosition}
    onchange={(v) => settings.patch('general', { chatPosition: v })}
  />
</Row>

<Row label="Tooltips" hint="Short explanations when you hover buttons.">
  <Toggle label="Tooltips" checked={g.tooltips} onchange={(v) => settings.patch('general', { tooltips: v })} />
</Row>

<Row label="Minimize to tray" hint="Minimizing hides cyex in the tray instead of the taskbar.">
  <Toggle
    label="Minimize to tray"
    checked={g.minimizeToTray}
    onchange={(v) => settings.patch('general', { minimizeToTray: v })}
  />
</Row>

<ResetSection partition="general" />

<style>
  .field {
    width: 220px;
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
