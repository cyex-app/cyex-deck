// V105-11 Images pasted or dropped into a chat box go to the user's upload host; the links come back into the box.

import { Channel } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';

import type { UploadEvent } from '../../ipc/bindings/UploadEvent';
import type { UploadItem } from '../../ipc/bindings/UploadItem';
import { settings } from '../../lib/settings.svelte';
import { uploadIpc } from '../../ipc/upload';
import { extOfMime, hostOf, imagePaths } from './upload';

/** The image files on the clipboard (image data or copied files). */
export function imageFiles(e: ClipboardEvent): File[] {
  return [...(e.clipboardData?.files ?? [])].filter((f) => f.type.startsWith('image/'));
}

/** Sends pasted image data to Rust; types we don't upload are skipped. */
export async function stageFiles(files: File[]): Promise<UploadItem[]> {
  const items: UploadItem[] = [];
  for (const f of files) {
    const ext = extOfMime(f.type);
    if (!ext) continue;
    items.push({ kind: 'staged', value: await uploadIpc.stage(new Uint8Array(await f.arrayBuffer()), ext) });
  }
  return items;
}

/** One chat box's running upload. */
export class Upload {
  status = $state('');
  busy = $state(false);

  async start(channel: string, items: UploadItem[], insert: (link: string) => void, fail: (msg: string) => void) {
    this.busy = true;
    const on = new Channel<UploadEvent>();
    on.onmessage = (ev) => {
      if (ev.kind === 'started') this.status = `Uploading ${ev.index} of ${ev.total}…`;
      else if (ev.kind === 'done') insert(ev.link);
      else if (ev.kind === 'failed') fail(ev.message);
      else {
        this.status = '';
        this.busy = false;
      }
    };
    try {
      await uploadIpc.images(channel, items, on);
    } catch (e) {
      fail(String(e));
      this.status = '';
      this.busy = false;
    }
  }
}

interface Hooks {
  login: () => string;
  /** The box with its buttons: drops outside it are not ours. */
  row: () => HTMLElement | undefined;
  insert: (link: string) => void;
  fail: (msg: string) => void;
}

/** A chat box's paste and drop handling. Call while the component starts (it sets up an effect). */
export function useUploads(hooks: Hooks) {
  const state = new Uploads(hooks);
  return state;
}

class Uploads {
  readonly upload = new Upload();
  /** With "Ask before uploading" on, the batch waits for a yes. */
  pending = $state<{ host: string; count: number; go: () => void; pictures: File[]; paths: string[]; names: string[] } | null>(null);

  constructor(private hooks: Hooks) {
    $effect(() => {
      let off: (() => void) | undefined;
      let gone = false;
      void getCurrentWebview()
        .onDragDropEvent((ev) => this.dropped(ev.payload))
        .then((f) => (gone ? f() : (off = f)));
      return () => {
        gone = true;
        off?.();
      };
    });
  }

  /** `pictures` are the pasted files, for the question's thumbnails (dropped ones get a preview from Rust by path). */
  private begin(items: UploadItem[], pictures: File[] = []) {
    const cfg = settings.v.uploader;
    const host = hostOf(cfg.url);
    const { login, insert, fail } = this.hooks;
    const go = () => void this.upload.start(login(), items, insert, fail);
    const paths = pictures.length ? [] : items.filter((i) => i.kind === 'file').map((i) => i.value);
    const names = pictures.length ? [] : items.map((i) => i.value.split(/[\/]/).pop() ?? '');
    if (cfg.confirm) this.pending = { host, count: items.length, go, pictures, paths, names };
    else go();
  }

  private dropped(p: { type: string; paths?: string[]; position?: { x: number; y: number } }) {
    const row = this.hooks.row();
    if (p.type !== 'drop' || !settings.v.uploader.enabled || !row || !p.position) return;
    const r = row.getBoundingClientRect();
    const x = p.position.x / devicePixelRatio;
    const y = p.position.y / devicePixelRatio;
    if (x < r.left || x > r.right || y < r.top || y > r.bottom) return;
    const paths = imagePaths(p.paths ?? []);
    if (!paths.length) return;
    if (this.upload.busy) return this.hooks.fail('Wait for the upload to finish.');
    if (this.pending) return this.hooks.fail('Answer the upload question first.');
    this.begin(paths.map((value) => ({ kind: 'file', value })));
  }

  onpaste = async (e: ClipboardEvent) => {
    if (!settings.v.uploader.enabled) return;
    const files = imageFiles(e);
    if (!files.length) return;
    e.preventDefault();
    if (this.upload.busy) return this.hooks.fail('Wait for the upload to finish.');
    // A second batch would replace the first one's question and strand its staged files.
    if (this.pending) return this.hooks.fail('Answer the upload question first.');
    try {
      const items = await stageFiles(files);
      if (items.length) this.begin(items, files);
    } catch {
      this.hooks.fail('Couldn’t read that image.');
    }
  };
}
