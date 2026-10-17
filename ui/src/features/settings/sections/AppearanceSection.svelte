<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Stepper from '../../../components/Stepper.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { CustomTheme } from '../../../ipc/bindings/CustomTheme';
  import type { ThemeLook } from '../../../ipc/bindings/ThemeLook';
  import type { TabBarSide } from '../../../ipc/bindings/TabBarSide';
  import type { TabDensity } from '../../../ipc/bindings/TabDensity';
  import type { TabRows } from '../../../ipc/bindings/TabRows';
  import type { TabUnread } from '../../../ipc/bindings/TabUnread';
  import { themeIpc } from '../../../ipc/theme';
  import { settings } from '../../../lib/settings.svelte';
  import { ZOOMS } from '../../../lib/zoom';
  import { toast } from '../../../lib/toast.svelte';
  import ColorField from '../ColorField.svelte';
  import ResetSection from '../ResetSection.svelte';
  import { errorText } from '../../../lib/errors';
  import ThemeEditor from '../ThemeEditor.svelte';
  import ThemePicker from '../ThemePicker.svelte';
  import { accentOf, choiceOf, defaultAccentOf } from '../themes';

  const a = $derived(settings.v.appearance);
  /** V106-02 The built-in themes, for the accent row (shared with ThemePicker). */
  let builtins = $state.raw<CustomTheme[]>([]);
  $effect(() => {
    void themeIpc
      .builtins()
      .then((list) => (builtins = list))
      .catch((e) => toast.error(errorText(e)));
  });
  const choice = $derived(choiceOf(a, builtins));
  /** U8 The custom theme in use, if any. */
  const active = $derived(choice.kind === 'custom' ? choice.theme : null);
  const accent = $derived(accentOf(a, choice));
  const defaultAccent = $derived(defaultAccentOf(choice));

  /** The theme as painted, for the editor and the accent note. */
  let look = $state<ThemeLook | null>(null);
  $effect(() => {
    void settings.v.appearance;
    void themeIpc.look().then((l) => (look = l));
  });

  function saveTheme(next: CustomTheme) {
    void settings.patch('appearance', { themes: a.themes.map((t) => (t.id === next.id ? next : t)) });
  }

  /** Deletes at once; Undo puts the theme back where it was, and switches to it again if it was in use. */
  function deleteTheme(id: string) {
    const at = a.themes.findIndex((t) => t.id === id);
    const gone = a.themes[at];
    if (!gone) return;
    const wasUsed = a.themeId === id;
    void settings.patch('appearance', {
      themes: a.themes.filter((t) => t.id !== id),
      themeId: wasUsed ? '' : a.themeId,
    });
    toast.success(`Deleted “${gone.name}”.`, {
      action: {
        label: 'Undo',
        run: () => {
          const now = settings.v.appearance.themes.filter((t) => t.id !== id);
          void settings.patch('appearance', {
            themes: [...now.slice(0, at), gone, ...now.slice(at)],
            ...(wasUsed ? { themeId: id } : {}),
          });
        },
      },
    });
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
  /** F16 */
  const ROWS: { value: TabRows; label: string }[] = [
    { value: 'limit', label: 'Up to' },
    { value: 'all', label: 'All' },
  ];

  /** Zoom steps; the house style puts a space before the percent sign. */
  const ZOOM_STEPS = ZOOMS.map((z) => ({ value: z as number, label: `${z} %` }));

  const UNREAD: { value: TabUnread; label: string }[] = [
    { value: 'all', label: 'Messages' },
    { value: 'mentions', label: 'Mentions' },
    { value: 'off', label: 'Off' },
  ];

  // 3.3: tell the user when their color was nudged to stay visible.
  const adjusted = $derived(!!look && look.accent.accent !== accent);

  /** Sets the accent of the theme in use. Default (`null`) gives a built-in its own back. */
  function writeAccent(v: string | null) {
    if (choice.kind === 'custom') saveTheme({ ...choice.theme, accent: v ?? defaultAccent });
    else if (choice.kind === 'builtin') void settings.patch('appearance', { accents: { [choice.theme.id]: v === defaultAccent ? null : v } });
    else void settings.patch('appearance', { accent: v ?? defaultAccent });
  }
</script>

<SectionHead title="Appearance" level={2}>Colors, themes and how tabs look.</SectionHead>

<SectionHead title="Colors">The theme and accent color.</SectionHead>
<Row setting="appearance.theme appearance.themes" label="Theme" hint="Make your own under Custom themes, or paste a code." stacked>
  <ThemePicker {a} {builtins} {choice} />
</Row>

{#if active}
  <ThemeEditor theme={active} {look} onchange={saveTheme} ondelete={() => deleteTheme(active.id)} />
{/if}

<Row setting="appearance.accent" label="Accent color" hint={adjusted ? 'Shown a bit lighter or darker here so it stays visible.' : undefined}>
  <ColorField
    value={accent}
    label="Accent color"
    width="92px"
    onchange={writeAccent}
    oninvalid={() => toast.error('Use a hex color like #145aff.')}
  />
  <Button variant="ghost" small disabled={accent === defaultAccent} onclick={() => writeAccent(null)}
    >Default</Button
  >
</Row>

<!-- T4 A picture of the accent, not controls: nothing here takes focus. -->
<div class="sample" inert>
  <span class="cap">Preview</span>
  <Button variant="primary" small>Button</Button>
  <span class="link">Accent text</span>
  <Toggle label="Sample" checked={true} onchange={() => {}} />
</div>

<Row setting="appearance.zoom" label="Zoom" hint="Pop-outs follow. Overlays keep their own size.">
  <Segmented
    label="Zoom"
    options={ZOOM_STEPS}
    value={a.zoom}
    onchange={(v) => settings.patch('appearance', { zoom: v })}
  />
</Row>

<SectionHead title="Tabs">The tab bar at the top or side.</SectionHead>
<Row setting="appearance.tabDensity" label="Tab size" hint="Compact fits more tabs.">
  <Segmented
    label="Tab size"
    options={DENSITIES}
    value={a.tabDensity}
    onchange={(v) => settings.patch('appearance', { tabDensity: v })}
  />
</Row>

<Row setting="appearance.tabCloseButton" label="Close button on tabs" hint="Middle click still closes a tab when this is off.">
  <Toggle
    label="Close button on tabs"
    checked={a.tabCloseButton}
    onchange={(v) => settings.patch('appearance', { tabCloseButton: v })}
  />
</Row>

<Row setting="appearance.tabBarSide" label="Bar position">
  <Segmented
    label="Bar position"
    options={SIDES}
    value={a.tabBarSide}
    onchange={(v) => settings.patch('appearance', { tabBarSide: v })}
  />
</Row>

<Row setting="appearance.tabRows" label="Rows of tabs" hint="For a bar at the top or bottom. Past that it scrolls.">
  <Segmented label="Rows of tabs" options={ROWS} value={a.tabRows} onchange={(v) => settings.patch('appearance', { tabRows: v })} />
  {#if a.tabRows === 'limit'}
    <Stepper
      label="Most rows"
      value={a.tabMaxRows}
      min={1}
      max={10}
      onchange={(v) => settings.patch('appearance', { tabMaxRows: v })}
    />
  {/if}
</Row>

<Row setting="appearance.tabOnlyLive" label="Only live tabs" hint="Hides tabs whose channels are offline. The one you’re on stays.">
  <Toggle
    label="Only live tabs"
    checked={a.tabOnlyLive}
    onchange={(v) => settings.patch('appearance', { tabOnlyLive: v })}
  />
</Row>

<Row setting="appearance.tabUnread" label="Light up tabs" hint="Background tabs light up on new chat, brighter for mentions.">
  <Segmented
    label="Light up tabs"
    options={UNREAD}
    value={a.tabUnread}
    onchange={(v) => settings.patch('appearance', { tabUnread: v })}
  />
</Row>

<ResetSection partition="appearance" />

<style>
  .cap {
    color: var(--dim);
    font-size: 0.78rem;
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
