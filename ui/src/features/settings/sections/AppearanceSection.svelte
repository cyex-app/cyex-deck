<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import { openMenu } from '../../../components/menu.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { CustomTheme } from '../../../ipc/bindings/CustomTheme';
  import type { ThemeLook } from '../../../ipc/bindings/ThemeLook';
  import type { TabBarSide } from '../../../ipc/bindings/TabBarSide';
  import type { TabDensity } from '../../../ipc/bindings/TabDensity';
  import type { TabUnread } from '../../../ipc/bindings/TabUnread';
  import type { Theme } from '../../../ipc/bindings/Theme';
  import { themeIpc } from '../../../ipc/theme';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import ResetSection from '../ResetSection.svelte';
  import ThemeEditor from '../ThemeEditor.svelte';

  const DEFAULT_ACCENT = '#145aff';
  const HEX = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i;

  const a = $derived(settings.v.appearance);
  /** U8 The custom theme in use, if any. */
  const active = $derived(a.themes.find((t) => t.id === a.themeId) ?? null);
  const accent = $derived(active?.accent ?? a.accent);

  /** Colors for the little previews; keep in sync with tokens.css. */
  const BUILT_IN: Record<Theme, { label: string; bg: string; surface: string; text: string }> = {
    dark: { label: 'Dark', bg: '#1e1f23', surface: '#26272c', text: '#e8e8ea' },
    light: { label: 'Light', bg: '#f3f3f4', surface: '#ffffff', text: '#1b1c20' },
  };
  const BASES: Theme[] = ['dark', 'light'];
  const preview = (t: CustomTheme) => {
    const b = BUILT_IN[t.base];
    return { bg: t.colors.bg ?? b.bg, surface: t.colors.surface ?? b.surface, text: t.colors.text ?? b.text };
  };

  /** The theme as painted, for the editor and the accent note. */
  let look = $state<ThemeLook | null>(null);
  $effect(() => {
    void settings.v.appearance;
    void themeIpc.look().then((l) => (look = l));
  });

  const pickBuiltIn = (theme: Theme) => settings.patch('appearance', { theme, themeId: '' });
  const pickCustom = (id: string) => settings.patch('appearance', { themeId: id });

  function saveTheme(next: CustomTheme) {
    void settings.patch('appearance', { themes: a.themes.map((t) => (t.id === next.id ? next : t)) });
  }

  function deleteTheme(id: string) {
    void settings.patch('appearance', {
      themes: a.themes.filter((t) => t.id !== id),
      themeId: a.themeId === id ? '' : a.themeId,
    });
  }

  async function add(theme: CustomTheme) {
    try {
      await themeIpc.add(theme);
    } catch (e) {
      toast.error(String(e));
    }
  }

  /** A new theme starts as a copy of what's showing now. */
  function newTheme() {
    void add({
      id: '',
      name: active ? `${active.name} copy` : 'My theme',
      base: a.theme,
      colors: active ? { ...active.colors } : {},
      accent,
      wallpaper: active ? { ...active.wallpaper } : { source: '', area: 'window', dim: 70, blur: 0, fit: 'cover' },
    });
  }

  async function presets(e: MouseEvent) {
    // Under the button (V101-30); the event's target is gone after the wait.
    const at = e.currentTarget as HTMLElement;
    const list = await themeIpc.presets();
    openMenu(
      at,
      list.map((p) => ({ label: p.name, run: () => void add(p) })),
      'end',
    );
  }

  let code = $state('');
  async function importCode() {
    try {
      await themeIpc.import(code);
      code = '';
      toast.info('Theme added.');
    } catch (e) {
      toast.error(String(e));
    }
  }
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

  const UNREAD: { value: TabUnread; label: string }[] = [
    { value: 'all', label: 'Messages' },
    { value: 'mentions', label: 'Mentions' },
    { value: 'off', label: 'Off' },
  ];

  // 3.3: tell the user when their color was nudged to stay visible.
  const adjusted = $derived(!!look && look.accent.accent !== accent);

  /** Sets the accent of the theme in use: a custom theme's own, else the plain one. */
  function writeAccent(v: string) {
    if (active) saveTheme({ ...active, accent: v });
    else void settings.patch('appearance', { accent: v });
  }

  function setAccent(input: HTMLInputElement) {
    const v = input.value.trim();
    if (!HEX.test(v)) {
      toast.error('Use a hex color like #145aff.');
      input.value = accent;
      return;
    }
    writeAccent(v.startsWith('#') ? v : `#${v}`);
  }
</script>

<h2>Appearance</h2>

<h3 class="label">Colors</h3>
<Row label="Theme" hint="Make your own with New, or paste a code someone shared." stacked>
  <div class="themes" role="radiogroup" aria-label="Theme">
    {#each BASES as base (base)}
      {@const b = BUILT_IN[base]}
      <button class="theme" role="radio" aria-checked={!active && a.theme === base} onclick={() => pickBuiltIn(base)}>
        <span class="pic" style:background={b.bg}><i style:background={b.surface}></i><b style:background={b.text}></b></span>
        {b.label}
      </button>
    {/each}
    {#each a.themes as t (t.id)}
      {@const p = preview(t)}
      <button class="theme" role="radio" aria-checked={active?.id === t.id} onclick={() => pickCustom(t.id)}>
        <span class="pic" style:background={p.bg}
          ><i style:background={p.surface}></i><b style:background={p.text}></b><u style:background={t.accent}></u></span
        >
        {t.name}
      </button>
    {/each}
    <button class="theme add" onclick={newTheme}><span class="pic">+</span>New</button>
  </div>
  <div class="tools">
    <Button small variant="ghost" onclick={presets}>Start from a preset</Button>
    <input class="field code" placeholder="Paste a theme code" aria-label="Theme code" bind:value={code} />
    <Button small disabled={!code.trim()} onclick={importCode}>Add</Button>
  </div>
</Row>

{#if active}
  <ThemeEditor theme={active} {look} onchange={saveTheme} ondelete={() => deleteTheme(active.id)} />
{/if}

<Row label="Accent color" hint={adjusted ? 'Shown a bit lighter or darker here so it stays visible.' : undefined}>
  <input
    type="color"
    class="swatch"
    aria-label="Pick accent color"
    value={accent}
    oninput={(e) => setAccent(e.currentTarget)}
  />
  <input class="field hex" aria-label="Accent hex code" value={accent} onchange={(e) => setAccent(e.currentTarget)} />
  <Button variant="ghost" small disabled={accent === DEFAULT_ACCENT} onclick={() => writeAccent(DEFAULT_ACCENT)}
    >Default</Button
  >
</Row>

<div class="sample">
  <Button variant="primary" small>Button</Button>
  <span class="link">Accent text</span>
  <Toggle label="Sample" checked={true} onchange={() => {}} />
</div>

<h3 class="label">Tabs</h3>
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

<Row label="Bar position">
  <Segmented
    label="Bar position"
    options={SIDES}
    value={a.tabBarSide}
    onchange={(v) => settings.patch('appearance', { tabBarSide: v })}
  />
</Row>

<Row label="Light up tabs" hint="Background tabs light up on new chat, brighter for mentions.">
  <Segmented
    label="Light up tabs"
    options={UNREAD}
    value={a.tabUnread}
    onchange={(v) => settings.patch('appearance', { tabUnread: v })}
  />
</Row>

<ResetSection partition="appearance" />

<style>
  h3 {
    margin: 22px 0 2px;
  }
  .themes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .theme {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    width: 92px;
    padding: 6px;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.78rem;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .theme:hover {
    background: var(--row-hover);
  }
  .theme[aria-checked='true'] {
    border-color: var(--accent-border);
    color: var(--text);
  }
  .pic {
    position: relative;
    display: grid;
    place-items: center;
    width: 78px;
    height: 46px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .pic i {
    position: absolute;
    inset: 0 auto 0 0;
    width: 24px;
  }
  .pic b {
    position: absolute;
    left: 32px;
    right: 10px;
    top: 14px;
    height: 4px;
    border-radius: 2px;
  }
  .pic u {
    position: absolute;
    left: 32px;
    width: 18px;
    top: 26px;
    height: 6px;
    border-radius: 2px;
  }
  .add .pic {
    border-style: dashed;
    font-size: 1.2rem;
    color: var(--dim);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .code {
    flex: 1;
    min-width: 0;
    font-family: var(--font-mono);
  }
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
