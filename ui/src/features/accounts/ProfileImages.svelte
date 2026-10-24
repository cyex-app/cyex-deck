<!-- V107-R19 Profile picture, banner and offline image: preview, Change, and Restore original
     while the app keeps the image its first change replaced. Twitch's internal API; when Twitch refuses,
     its own settings page opens in the app's Twitch window. V107-R19c Under each, the last ones changed here.
     V107-Mh Remove (asks first) puts Twitch's default back; the removed image stays in Recent. No banner Remove:
     Twitch has no way for that. -->
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
  import ProfileImageHistory from './ProfileImageHistory.svelte';

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
  const REMOVED: Record<ProfileImageKind, string> = {
    avatar: 'Profile picture removed.',
    banner: 'Banner removed.',
    offline: 'Offline image removed.',
  };
  const ASK: Record<ProfileImageKind, string> = {
    avatar: 'Remove your profile picture? It stays in Recent.',
    banner: 'Remove the banner? It stays in Recent.',
    offline: 'Remove the offline image? It stays in Recent.',
  };

  let view = $state.raw<ProfileImagesView | null>(null);
  let loadError = $state('');
  let busy = $state<ProfileImageKind | null>(null);
  let removing = $state(false);
  let asking = $state<ProfileImageKind | null>(null);
  let failed = $state<{ kind: ProfileImageKind; error: ImageError } | null>(null);
  let cropping = $state.raw<File | null>(null);
  const inputs: Partial<Record<ProfileImageKind, HTMLInputElement>> = {};
  let askEl = $state<HTMLElement>();

  $effect(() => {
    if (asking && askEl) askEl.querySelector<HTMLButtonElement>('button:last-of-type')?.focus();
  });

  function cancelAsk(kind: ProfileImageKind) {
    asking = null;
    queueMicrotask(() => {
      const btns = document.querySelectorAll<HTMLButtonElement>(`.row[data-kind="${kind}"] .actions button`);
      [...btns].find((b) => b.textContent?.trim().startsWith('Remove') && !b.textContent.includes('original'))?.focus();
    });
  }

  $effect(() => {
    profileImagesIpc
      .get(account)
      .then((v) => (view = v))
      .catch((e) => (loadError = imageError(e)?.message ?? errorText(e)));
  });

  /** V107-Mz4 Twitch can take minutes to show a new offline image: the editor showed the one
   * before, and the next upload brought it out. After an upload Twitch isn't showing yet, the
   * editor checks again until it does (or gives up after a few minutes). */
  const FOLLOW_EVERY = 5000;
  const FOLLOW_FOR = 180_000;
  let pending = $state<ProfileImageKind | null>(null);
  let follow = 0;
  let followTimer: ReturnType<typeof setTimeout> | undefined;

  function stopFollow() {
    follow++;
    clearTimeout(followTimer);
    pending = null;
  }

  function followUp(kind: ProfileImageKind, stale: string, remove: boolean) {
    stopFollow();
    const mine = follow;
    const until = Date.now() + FOLLOW_FOR;
    pending = kind;
    const tick = async () => {
      if (mine !== follow) return;
      if (Date.now() > until) {
        pending = null;
        return;
      }
      try {
        const v = await profileImagesIpc.get(account);
        if (mine !== follow) return;
        if ((v.slots.find((sl) => sl.kind === kind)?.url ?? '') !== stale) {
          view = v;
          pending = null;
          toast.success((remove ? REMOVED : DONE)[kind]);
          return;
        }
      } catch {
        // Try again on the next tick.
      }
      followTimer = setTimeout(tick, FOLLOW_EVERY);
    };
    followTimer = setTimeout(tick, FOLLOW_EVERY);
  }

  $effect(() => () => stopFollow());

  async function run(kind: ProfileImageKind, call: () => Promise<ProfileImagesView>, remove = false) {
    stopFollow();
    busy = kind;
    removing = remove;
    asking = null;
    failed = null;
    try {
      view = await call();
      const waiting = remove ? 'Removed. Twitch may take a minute to show it.' : 'Uploaded. Twitch may take a minute to show it.';
      toast.success(view.waiting ? waiting : (remove ? REMOVED : DONE)[kind]);
      if (view.waiting) followUp(kind, view.slots.find((sl) => sl.kind === kind)?.url ?? '', remove);
    } catch (e) {
      const error = imageError(e) ?? { kind: 'failed', message: errorText(e) };
      failed = { kind, error };
      // The image it replaced may be saved already: Restore original shows.
      profileImagesIpc.get(account).then((v) => (view = v), () => {});
    } finally {
      busy = null;
      removing = false;
    }
  }

  function remove(kind: ProfileImageKind) {
    void run(kind, () => profileImagesIpc.remove(account, kind), true);
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

  async function forget(kind: ProfileImageKind, id: string) {
    try {
      await profileImagesIpc.forget(account, kind, id);
    } catch (e) {
      toast.error(imageError(e)?.message ?? errorText(e));
    }
    profileImagesIpc.get(account).then((v) => (view = v), () => {});
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
          {#if slot.url}<img src={slot.url} alt="" />{:else}<span>{slot.accent ? 'Default' : 'None'}</span>{/if}
        </div>
        <span class="name">
          {LABEL[slot.kind]}
          {#if pending === slot.kind}<span class="pending">Waiting for Twitch…</span>{/if}
        </span>
        <div class="actions">
          {#if slot.backup}
            <Button small variant="ghost" disabled={busy !== null} onclick={() => run(slot.kind, () => profileImagesIpc.restore(account, slot.kind))}>
              Restore original
            </Button>
          {/if}
          {#if slot.removable}
            <Button small variant="ghost" disabled={busy !== null} onclick={() => (asking = asking === slot.kind ? null : slot.kind)}>
              {busy === slot.kind && removing ? 'Removing…' : 'Remove'}
            </Button>
          {/if}
          <Button small disabled={busy !== null} onclick={() => inputs[slot.kind]?.click()}>
            {busy === slot.kind && !removing ? 'Uploading…' : 'Change'}
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
      {#if asking === slot.kind && slot.removable}
        <div class="ask" bind:this={askEl}>
          <span>{ASK[slot.kind]}</span>
          <Button small variant="danger" disabled={busy !== null} onclick={() => remove(slot.kind)}>Remove</Button>
          <Button small variant="ghost" onclick={() => cancelAsk(slot.kind)}>Cancel</Button>
        </div>
      {/if}
      {#if slot.history.length}
        <ProfileImageHistory
          kind={slot.kind}
          items={slot.history}
          disabled={busy !== null}
          onuse={(id) => run(slot.kind, () => profileImagesIpc.use(account, slot.kind, id))}
          onforget={(id) => forget(slot.kind, id)}
        />
      {/if}
      {#if failed?.kind === slot.kind}
        <div class="fail">
          <span>{failed.error.message}</span>
          {#if failed.error.kind === 'refused'}
            <Button small variant="link" onclick={onTwitch}>Change it on Twitch</Button>
          {/if}
        </div>
      {/if}
    {/each}
    <small>PNG, JPG or GIF, up to 10 MB. The app keeps your original until you restore it, and your last few changes.</small>
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
  .pending {
    display: block;
    font-weight: 400;
    font-size: var(--text-sm, 12px);
    color: var(--dim);
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .ask {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 0 10px 108px;
    font-size: 0.82rem;
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
