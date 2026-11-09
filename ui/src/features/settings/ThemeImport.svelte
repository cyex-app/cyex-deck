<!-- V109-T01 Adding a theme from a cyex.app link (or an old code): a look at it first, then
     Add. A link's preview has Report. Renders two flex items for ThemePicker's tools row:
     the field, and the preview on a line of its own. -->
<script lang="ts">
  import { enterKey } from '../../lib/keys';
  import Button from '../../components/Button.svelte';
  import type { ThemePreview } from '../../ipc/bindings/ThemePreview';
  import { themeIpc } from '../../ipc/theme';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { PLAIN_PREVIEW } from './themes';
  import { MAX_REASON } from './themeLink';

  let text = $state('');
  let busy = $state(false);
  let preview = $state.raw<ThemePreview | null>(null);
  let reporting = $state(false);
  let reason = $state('');

  const look = $derived.by(() => {
    if (!preview) return null;
    const t = preview.theme;
    const b = PLAIN_PREVIEW[t.base];
    return { bg: t.colors.bg ?? b.bg, surface: t.colors.surface ?? b.surface, text: t.colors.text ?? b.text, accent: t.accent };
  });
  const meta = $derived.by(() => {
    if (!preview) return '';
    if (!preview.sourceId) return 'From a theme code';
    return [`v${preview.version}`, preview.date].filter(Boolean).join(' · ');
  });

  async function work(task: () => Promise<void>) {
    if (busy) return;
    busy = true;
    try {
      await task();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  const show = () =>
    work(async () => {
      preview = await themeIpc.preview(text);
      reporting = false;
    });

  function close() {
    preview = null;
    reporting = false;
    reason = '';
  }

  const add = () =>
    work(async () => {
      if (!preview) return;
      await themeIpc.addPreview(preview);
      text = '';
      close();
      toast.success('Theme added.');
    });

  const report = () =>
    work(async () => {
      if (!preview?.sourceId) return;
      await themeIpc.report(preview.sourceId, reason);
      toast.success('Reported. Thanks.');
      close();
    });
</script>

<span class="import">
  <span class="code-label">Theme link</span>
  <input
    class="field code"
    placeholder="Paste a link or code"
    aria-label="Theme link or code"
    bind:value={text}
    onkeydown={(e) => {
      if (enterKey(e) && text.trim()) void show();
    }}
  />
  <Button small disabled={!text.trim() || busy} onclick={() => void show()}>Add</Button>
</span>

{#if preview && look}
  <div class="preview" role="group" aria-label="Theme to add">
    <span class="pic" style:background={look.bg}
      ><i style:background={look.surface}></i><b style:background={look.text}></b><u style:background={look.accent}></u></span
    >
    <span class="about">
      <span class="name">{preview.theme.name}</span>
      <span class="meta">{meta}</span>
    </span>
    {#if reporting}
      <input
        class="field reason"
        maxlength={MAX_REASON}
        placeholder="What’s wrong with it?"
        aria-label="Reason"
        bind:value={reason}
      />
      <Button small variant="danger" disabled={!reason.trim() || busy} onclick={() => void report()}>Send report</Button>
      <Button small variant="ghost" onclick={() => (reporting = false)}>Cancel</Button>
    {:else}
      <Button small variant="primary" disabled={busy} onclick={() => void add()}>Add theme</Button>
      <Button small variant="ghost" onclick={close}>Cancel</Button>
      {#if preview.sourceId}
        <span class="report"><Button small variant="link" onclick={() => (reporting = true)}>Report</Button></span>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .import {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    min-width: 0;
  }
  .code-label {
    color: var(--muted);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .code {
    flex: 0 1 200px;
    min-width: 0;
    height: 28px;
  }
  .preview {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    flex-basis: 100%;
    padding: 8px 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .pic {
    position: relative;
    flex: none;
    width: 52px;
    height: 32px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .pic i {
    position: absolute;
    inset: 0 auto 0 0;
    width: 16px;
  }
  .pic b {
    position: absolute;
    left: 22px;
    right: 6px;
    top: 10px;
    height: 3px;
    border-radius: var(--radius-sm);
  }
  .pic u {
    position: absolute;
    left: 22px;
    width: 12px;
    top: 18px;
    height: 5px;
    border-radius: var(--radius-sm);
  }
  .about {
    display: flex;
    flex-direction: column;
    flex: 1 1 140px;
    min-width: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .reason {
    flex: 1 1 200px;
    min-width: 0;
    height: 28px;
  }
  .report {
    margin-left: auto;
  }
  @media (forced-colors: active) {
    .pic,
    .pic * {
      forced-color-adjust: none;
    }
  }
</style>
