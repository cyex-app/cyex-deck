<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import { previewAccent } from '../../../design/theme';
  import type { TabBarSide } from '../../../ipc/bindings/TabBarSide';
  import type { TabDensity } from '../../../ipc/bindings/TabDensity';
  import type { Theme } from '../../../ipc/bindings/Theme';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import ResetSection from '../ResetSection.svelte';

  const DEFAULT_ACCENT = '#145aff';
  const HEX = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i;

  const a = $derived(settings.v.appearance);

  const THEMES: { value: Theme; label: string }[] = [
    { value: 'dark', label: 'Dark' },
    { value: 'light', label: 'Light' },
  ];
  const DENSITIES: { value: TabDensity; label: string }[] = [
    { value: 'comfortable', label: 'Roomy' },
    { value: 'normal', label: 'Normal' },
    { value: 'compact', label: 'Compact' },
  ];
  const SIDES: { value: TabBarSide; label: string }[] = [
    { value: 'top', label: 'Top' },
    { value: 'bottom', label: 'Bottom' },
    { value: 'left', label: 'Left' },
    { value: 'right', label: 'Right' },
  ];

  // 3.3: tell the user when their color was nudged to stay visible.
  let adjusted = $state(false);
  $effect(() => {
    const { accent, theme } = a;
    previewAccent(accent, theme).then((t) => (adjusted = t.accent !== accent));
  });

  function setAccent(input: HTMLInputElement) {
    const v = input.value.trim();
    if (!HEX.test(v)) {
      toast.error('Use a hex color like #145aff.');
      input.value = a.accent;
      return;
    }
    void settings.patch('appearance', { accent: v.startsWith('#') ? v : `#${v}` });
  }
</script>

<h2>Appearance</h2>

<Row label="Theme">
  <Segmented label="Theme" options={THEMES} value={a.theme} onchange={(v) => settings.patch('appearance', { theme: v })} />
</Row>

<Row label="Accent color" hint={adjusted ? 'Shown a bit lighter or darker here so it stays visible.' : undefined}>
  <input
    type="color"
    class="swatch"
    aria-label="Pick accent color"
    value={a.accent}
    oninput={(e) => setAccent(e.currentTarget)}
  />
  <input
    class="field hex"
    aria-label="Accent hex code"
    value={a.accent}
    onchange={(e) => setAccent(e.currentTarget)}
  />
  <Button
    variant="ghost"
    small
    disabled={a.accent === DEFAULT_ACCENT}
    onclick={() => settings.patch('appearance', { accent: DEFAULT_ACCENT })}>Default</Button
  >
</Row>

<div class="sample">
  <Button variant="primary" small>Button</Button>
  <span class="link">Accent text</span>
  <Toggle label="Sample" checked={true} onchange={() => {}} />
</div>

<Row label="Tab size" hint="Compact fits more tabs.">
  <Segmented
    label="Tab size"
    options={DENSITIES}
    value={a.tabDensity}
    onchange={(v) => settings.patch('appearance', { tabDensity: v })}
  />
</Row>

<Row label="Close button on tabs" hint="Middle click still closes a tab when this is off.">
  <Toggle
    label="Close button on tabs"
    checked={a.tabCloseButton}
    onchange={(v) => settings.patch('appearance', { tabCloseButton: v })}
  />
</Row>

<Row label="Tab bar position">
  <Segmented
    label="Tab bar position"
    options={SIDES}
    value={a.tabBarSide}
    onchange={(v) => settings.patch('appearance', { tabBarSide: v })}
  />
</Row>

<ResetSection partition="appearance" />

<style>
  .swatch {
    width: 34px;
    height: 30px;
    padding: 2px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .swatch::-webkit-color-swatch-wrapper {
    padding: 0;
  }
  .swatch::-webkit-color-swatch {
    border: 0;
    border-radius: 2px;
  }
  .hex {
    width: 92px;
    font-family: var(--font-mono);
  }
  .sample {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .link {
    color: var(--accent-text);
    font-weight: 600;
  }
</style>
