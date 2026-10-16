// What the players' right-click menus (V101-17) share: hotkeys as shown, copying, links
// out and picture in picture.

import { systemIpc } from '../../ipc/system';
import { bindingOf } from '../../lib/hotkeys.svelte';
import { toast } from '../../lib/toast.svelte';
import type { PlayerMenu } from './menu';
import { errorText } from '../../lib/errors';
import { copy } from '../../lib/clipboard';

export function playerKeys(): PlayerMenu['keys'] {
  return { play: bindingOf('player.playPause'), mute: bindingOf('player.mute'), fullscreen: bindingOf('player.fullscreen') };
}

export function copyText(text: string, what = 'Link'): void {
  void copy(text, what);
}

export function openLink(url: string): void {
  systemIpc.openLink(url).catch((e) => toast.error(errorText(e)));
}

/** The menu entry, when the player can float in its own small window. */
export function pipFor(video: HTMLVideoElement | undefined): PlayerMenu['pip'] {
  if (!video || !document.pictureInPictureEnabled) return undefined;
  const on = document.pictureInPictureElement === video;
  return {
    on,
    toggle: () => {
      const done = on ? document.exitPictureInPicture() : video.requestPictureInPicture();
      done.catch(() => toast.error('Picture in picture didn’t start.'));
    },
  };
}

/** Tells `set` when the video goes into or out of picture in picture; returns the cleanup. */
export function watchPip(video: HTMLVideoElement, set: (on: boolean) => void): () => void {
  const on = () => set(true);
  const off = () => set(false);
  video.addEventListener('enterpictureinpicture', on);
  video.addEventListener('leavepictureinpicture', off);
  return () => {
    video.removeEventListener('enterpictureinpicture', on);
    video.removeEventListener('leavepictureinpicture', off);
  };
}
