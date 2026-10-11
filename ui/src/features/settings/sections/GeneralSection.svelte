<!-- General (2.1): chat text, where chat sits, and the app itself. Chat fonts come with the
     app, or are font files the user added, or any font on this PC by name (V101-10). -->
<script lang="ts">
  import Icon from '../../../components/Icon.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Stepper from '../../../components/Stepper.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import { BUNDLED_FONTS } from '../../../design/userFonts';
  import type { ChatPosition } from '../../../ipc/bindings/ChatPosition';
  import { fontsIpc } from '../../../ipc/theme';
  import { formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import ResetSection from '../ResetSection.svelte';

  const g = $derived(settings.v.general);

  /** Common Windows fonts to suggest; Helvetica only shows if it's installed. */
  const INSTALLED = ['Segoe UI', 'Arial', 'Verdana', 'Tahoma', 'Consolas', 'Cascadia Code', 'Helvetica'];
  const POSITIONS: { value: ChatPosition; label: string }[] = [
    { value: 'left', label: 'Left' },
    { value: 'right', label: 'Right' },
    { value: 'top', label: 'Top' },
    { value: 'bottom', label: 'Bottom' },
  ];

  const added = $derived(g.fonts.map((f) => f.name));
  /** Typed by name: not the app font, not bundled, not added. */
  const typed = $derived(!!g.chatFontFamily && !BUNDLED_FONTS.includes(g.chatFontFamily) && !added.includes(g.chatFontFamily));

  const pick = (family: string) => void settings.patch('general', { chatFontFamily: family });

  async function addFont() {
    try {
      const font = await fontsIpc.add();
      if (font) toast.info(`Added ${font.name}.`);
    } catch (e) {
      toast.error(String(e));
    }
  }

  function removeFont(name: string) {
    fontsIpc.remove(name).catch((e) => toast.error(String(e)));
  }
</script>

<h2>General</h2>

<h3 class="label">Chat text</h3>
<Row label="Font" hint="These come with the app. Add your own from a font file." stacked>
  <div class="fonts" role="radiogroup" aria-label="Chat font">
    <button class="font" role="radio" aria-checked={!g.chatFontFamily} onclick={() => pick('')}>App font</button>
    {#each BUNDLED_FONTS as f (f)}
      <button class="font" role="radio" aria-checked={g.chatFontFamily === f} style:font-family="'{f}'" onclick={() => pick(f)}
        >{f}</button
      >
    {/each}
    {#each added as f (f)}
      <span class="mine">
        <button class="font" role="radio" aria-checked={g.chatFontFamily === f} style:font-family={`"${f}"`} onclick={() => pick(f)}
          >{f}</button
        >
        <button class="remove" aria-label="Remove {f}" title="Remove" onclick={() => removeFont(f)}><Icon name="x" size={12} /></button>
      </span>
    {/each}
    <button class="font add" onclick={addFont}><Icon name="plus" size={13} />Add a font file</button>
  </div>
</Row>
<Row label="Installed font" hint="Any other font on this PC, by name. Helvetica only works if it's installed.">
  <input
    class="field"
    list="chat-fonts"
    placeholder="Font name"
    value={typed ? g.chatFontFamily : ''}
    onchange={(e) => pick(e.currentTarget.value.trim())}
  />
  <datalist id="chat-fonts">
    {#each INSTALLED as font (font)}<option value={font}></option>{/each}
  </datalist>
</Row>
<Row label="Size">
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

<h3 class="label">Layout</h3>
<Row label="Chat position" hint="Which side of the player or a clip the chat sits on.">
  <Segmented
    label="Chat position"
    options={POSITIONS}
    value={g.chatPosition}
    onchange={(v) => settings.patch('general', { chatPosition: v })}
  />
</Row>

<h3 class="label">App</h3>
<Row label="Tooltips" hint="Short explanations when you hover buttons.">
  <Toggle label="Tooltips" checked={g.tooltips} onchange={(v) => settings.patch('general', { tooltips: v })} />
</Row>
<Row label="Minimize to tray" hint="Minimizing hides cyex deck in the tray instead of the taskbar.">
  <Toggle
    label="Minimize to tray"
    checked={g.minimizeToTray}
    onchange={(v) => settings.patch('general', { minimizeToTray: v })}
  />
</Row>

<ResetSection partition="general" />

<style>
  h3 {
    margin: 22px 0 2px;
  }
  .field {
    width: 220px;
  }
  .fonts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
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
  .font:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .font[aria-checked='true'] {
    border-color: var(--accent-border);
    color: var(--text);
  }
  .add {
    border-style: dashed;
    font-family: var(--font-ui);
    font-size: 0.82rem;
  }
  .mine {
    position: relative;
    display: inline-flex;
  }
  .mine .font {
    padding-right: 26px;
  }
  .remove {
    position: absolute;
    top: 50%;
    right: 4px;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
    transform: translateY(-50%);
  }
  .remove:hover {
    background: var(--surface-hover);
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
