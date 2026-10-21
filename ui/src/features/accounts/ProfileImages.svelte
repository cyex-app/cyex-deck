<!-- V107-R19 Profile picture, banner and offline image: preview, Change, and Restore original
     while the app keeps the image its first change replaced. Twitch's internal API; when Twitch refuses,
     its own settings page opens in the app's Twitch window. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Loading from '../../components/Loading.svelte';
  import type { ImageError } from '../../ipc/bindings/ImageError';
  import type { ProfileImageKind } from '../../ipc/bindings/ProfileImageKind';
  import type { ProfileImagesView } from '../../ipc/bindings/ProfileImagesView';
  import { imageError, profileImagesIpc } from '../../ipc/profileImages';
  import { subsIpc } from '../../ipc/subs';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import AvatarCrop from './AvatarCrop.svelte';

  let { account }: { account: string } = $props();

  const MAX = 10 * 1024 * 1024;
  const TYPES = ['image/png', 'image/jpeg', 'image/gif'];
  const LABEL: Record<ProfileImageKind, string> = {
    avatar: 'Profile picture',
    banner: 'Banner',
    offline: 'Offline image',
  };
  const DONE: Record<ProfileImageKind, string> = {
    avatar: 'Profile picture changed.',
    banner: 'Banner changed.',
    offline: 'Offline image changed.',
  };

  let view = $state.raw<ProfileImagesView | null>(null);
  let loadError = $state('');
  let busy = $state<ProfileImageKind | null>(null);
  let failed = $state<{ kind: ProfileImageKind; error: ImageError } | null>(null);
  let cropping = $state.raw<File | null>(null);
  const inputs: Partial<Record<ProfileImageKind, HTMLInputElement>> = {};

  $effect(() => {
    profileImagesIpc
      .get(account)
      .then((v) => (view = v))
      .catch((e) => (loadError = imageError(e)?.message ?? errorText(e)));
  });

  async function run(kind: ProfileImageKind, call: () => Promise<ProfileImagesView>) {
    busy = kind;
    failed = null;
    try {
      view = await call();
      toast.success(view.waiting ? 'Uploaded. Twitch may take a minute to show it.' : DONE[kind]);
    } catch (e) {
      const error = imageError(e) ?? { kind: 'failed', message: errorText(e) };
      failed = { kind, error };
      // The image it replaced may be saved already: Restore original shows.
      profileImagesIpc.get(account).then((v) => (view = v), () => {});
    } finally {
      busy = null;
    }
  }

  function picked(kind: ProfileImageKind, input: HTMLInputElement) {
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    if (!TYPES.includes(file.type)) failed = { kind, error: { kind: 'format', message: 'Only PNG, JPG or GIF.' } };
    else if (file.size > MAX) failed = { kind, error: { kind: 'tooBig', message: 'That file is over 10 MB.' } };
    else if (kind === 'avatar') cropping = file;
    else void file.arrayBuffer().then((b) => run(kind, () => profileImagesIpc.change(account, kind, new Uint8Array(b))));
  }

  function cropped(png: Uint8Array) {
    cropping = null;
    void run('avatar', () => profileImagesIpc.change(account, 'avatar', png));
  }

  function onTwitch() {
    subsIpc.open(account, 'https://www.twitch.tv/settings/profile').catch((e) => toast.error(errorText(e)));
  }
</script>

<div class="images">
  {#if loadError}
    <p class="err">{loadError}</p>
  {:else if !view}
    <Loading />
  {:else}
    {#each view.slots as slot (slot.kind)}
      <div class="row" data-kind={slot.kind}>
        <div class="preview {slot.kind}">
          {#if slot.url}<img src={slot.url} alt="" />{:else}<span>None</span>{/if}
        </div>
        <span class="name">{LABEL[slot.kind]}</span>
        <div class="actions">
          {#if slot.backup}
            <Button small variant="ghost" disabled={busy !== null} onclick={() => run(slot.kind, () => profileImagesIpc.restore(account, slot.kind))}>
              Restore original
            </Button>
          {/if}
          <Button small disabled={busy !== null} onclick={() => inputs[slot.kind]?.click()}>
            {busy === slot.kind ? 'Uploading…' : 'Change'}
          </Button>
          <input
            bind:this={inputs[slot.kind]}
            type="file"
            accept={TYPES.join(',')}
            hidden
            onchange={(e) => picked(slot.kind, e.currentTarget)}
          />
        </div>
      </div>
      {#if failed?.kind === slot.kind}
        <div class="fail">
          <span>{failed.error.message}</span>
          {#if failed.error.kind === 'refused'}
            <Button small variant="link" onclick={onTwitch}>Change it on Twitch</Button>
          {/if}
        </div>
      {/if}
    {/each}
    <small>PNG, JPG or GIF, up to 10 MB. The app keeps your original until you restore it.</small>
  {/if}
</div>

{#if cropping}<AvatarCrop file={cropping} ondone={cropped} onclose={() => (cropping = null)} />{/if}

<style>
  .images {
    display: flex;
    flex-direction: column;
    padding: 4px 0 8px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 52px;
    border-top: 1px solid var(--border);
  }
  .preview {
    flex: none;
    display: grid;
    place-items: center;
    overflow: hidden;
    width: 96px;
    height: 40px;
    border-radius: var(--radius);
    background: var(--bg);
    border: 1px solid var(--border);
  }
  .preview.avatar {
    width: 40px;
    border-radius: 50%;
    margin-right: 56px;
  }
  .preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .preview span {
    font-size: 0.72rem;
    color: var(--dim);
  }
  .name {
    flex: 1;
    min-width: 0;
    font-weight: 600;
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .fail {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 0 8px 108px;
    color: var(--red);
    font-size: 0.82rem;
  }
  small {
    padding-top: 6px;
    border-top: 1px solid var(--border);
    color: var(--dim);
    font-size: 0.74rem;
  }
  .err {
    color: var(--red);
  }
</style>
