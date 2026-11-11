<!-- V109-T01 Sharing your own theme by link: Share (uploads, copies the link), then Copy link,
     Update shared theme (when it changed since) and Stop sharing. Copy code stays for
     sharing without the server. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import ConfirmChoices from '../../components/ConfirmChoices.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
  import { themeIpc } from '../../ipc/theme';
  import { copy } from '../../lib/clipboard';
  import { copied } from '../../lib/copied.svelte';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { asksBeforeUpload, shareState } from './themeLink';

  // V110-A-idea A check mark on the button for a moment, not a toast.
  const cp = copied();

  interface Props {
    theme: CustomTheme;
    /** Saves edits still waiting for the next frame, so the upload has them. */
    flush: () => void;
  }

  let { theme, flush }: Props = $props();

  const s = $derived(shareState(theme));
  let busy = $state(false);
  /** Shared from another PC (settings brought over): the link works, the key isn't here. */
  let canEdit = $state(true);
  $effect(() => {
    if (s.kind !== 'shared') return;
    const id = theme.id;
    void themeIpc.canEdit(id).then((v) => {
      if (theme.id === id) canEdit = v;
    });
  });

  async function run(task: () => Promise<void>) {
    if (busy) return;
    busy = true;
    flush();
    try {
      await task();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  const share = () =>
    run(async () => {
      const link = await themeIpc.share(theme.id);
      await copy(link, 'Link');
    });

  const update = () =>
    run(async () => {
      const v = await themeIpc.shareUpdate(theme.id);
      toast.success(`Shared theme updated to v${v}.`);
    });

  const stop = () =>
    run(async () => {
      await themeIpc.shareStop(theme.id);
      toast.success('Sharing stopped. The link no longer works.');
    });

  /** Copy code on a theme with a wallpaper file: asks before the picture goes up. */
  let asking = $state(false);
  // A2-9 ThemeEditor doesn't remount this when another theme is picked: the question belongs to one theme.
  $effect(() => {
    void theme.id;
    asking = false;
  });

  function copyCode() {
    flush();
    if (asksBeforeUpload(theme)) asking = true;
    else void doCopy(true);
  }

  async function doCopy(withWallpaper: boolean) {
    asking = false;
    try {
      const { code, note } = await themeIpc.code(theme.id, withWallpaper);
      const ok = await copy(code, 'Theme code');
      if (ok && note) toast.info(note);
    } catch (e) {
      toast.error(errorText(e));
    }
  }
</script>

<div class="share">
  {#if s.kind === 'none'}
    <Button small icon="link" disabled={busy} onclick={share}>{busy ? 'Sharing…' : 'Share'}</Button>
  {:else}
    <Button small icon={cp.is('link') ? 'check' : 'copy'} onclick={() => void cp.run(s.link, 'Link', 'link')}>Copy link</Button>
    <span class="sr-only" aria-live="polite">{cp.said}</span>
    {#if canEdit}
      <Button small icon="upload" disabled={busy || !s.changed} onclick={update}>Update shared theme</Button>
      <ConfirmButton label="Stop sharing" confirm="Click to stop sharing" variant="ghost" disabled={busy} onconfirm={stop} />
    {/if}
    <span class="state">{s.changed ? 'Changed since you shared it' : `Shared · v${s.version}`}</span>
  {/if}
  {#if asking}
    <ConfirmChoices
      ask="Your wallpaper is on this PC. To put it in the code, it’s uploaded to cyex.app, and anyone with the code can open it."
      choices={[
        { label: 'Upload and copy', variant: 'primary', run: () => void doCopy(true) },
        { label: 'Copy without wallpaper', run: () => void doCopy(false) },
      ]}
      oncancel={() => (asking = false)}
    />
  {:else}
    <Button small variant="ghost" onclick={copyCode}>Copy code</Button>
  {/if}
</div>

<style>
  .share {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .state {
    color: var(--dim);
    font-size: var(--text-hint);
    white-space: nowrap;
  }
</style>
