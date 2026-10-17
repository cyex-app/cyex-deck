import assert from 'node:assert/strict';
import { test } from 'node:test';

import { DRIFT_MS, pictureBar, videoBar } from './storyBar.ts';

interface FakeAnim {
  keys: Keyframe[];
  opts: KeyframeAnimationOptions;
  currentTime: number;
  playbackRate: number;
  onfinish: (() => void) | null;
  calls: string[];
  play(): void;
  pause(): void;
  cancel(): void;
}

function fakeEl() {
  const anims: FakeAnim[] = [];
  const el = {
    animate(keys: Keyframe[], opts: KeyframeAnimationOptions) {
      const a: FakeAnim = {
        keys,
        opts,
        currentTime: 0,
        playbackRate: 1,
        onfinish: null,
        calls: [],
        play() {
          this.calls.push('play');
        },
        pause() {
          this.calls.push('pause');
        },
        cancel() {
          this.calls.push('cancel');
        },
      };
      anims.push(a);
      return a;
    },
  };
  return { el: el as unknown as Element, anims };
}

class FakeVideo extends EventTarget {
  duration = NaN;
  currentTime = 0;
  paused = true;
  readyState = 0;
  playbackRate = 1;
  listeners = 0;
  addEventListener(...args: Parameters<EventTarget['addEventListener']>) {
    this.listeners++;
    super.addEventListener(...args);
  }
  removeEventListener(...args: Parameters<EventTarget['removeEventListener']>) {
    this.listeners--;
    super.removeEventListener(...args);
  }
  fire(type: string) {
    this.dispatchEvent(new Event(type));
  }
}

function video(t: { el: Element }, v = new FakeVideo()) {
  const stop = videoBar(t.el, v as unknown as HTMLVideoElement);
  return { v, stop };
}

test('picture bar animates transform only, 6 s linear (V106-20)', () => {
  const { el, anims } = fakeEl();
  pictureBar(el, 6000, () => {});
  assert.equal(anims.length, 1);
  const [a] = anims;
  assert.equal(a.opts.duration, 6000);
  assert.equal(a.opts.easing, 'linear');
  for (const k of a.keys) {
    assert.deepEqual(Object.keys(k), ['transform']);
    assert.ok(!('width' in k));
  }
});

test('picture bar end calls onend; stop cancels and clears onfinish (V106-20)', () => {
  const { el, anims } = fakeEl();
  let ended = 0;
  const stop = pictureBar(el, 6000, () => ended++);
  anims[0].onfinish?.();
  assert.equal(ended, 1);
  stop();
  assert.equal(anims[0].onfinish, null);
  assert.deepEqual(anims[0].calls, ['cancel']);
});

test('video bar waits for a duration (V106-20)', () => {
  const t = fakeEl();
  const { v } = video(t);
  assert.equal(t.anims.length, 0);
  v.fire('playing');
  assert.equal(t.anims.length, 0);
});

test('video bar follows the video clock (V106-20)', () => {
  const t = fakeEl();
  const { v } = video(t);
  v.duration = 12;
  v.currentTime = 1;
  v.fire('loadedmetadata');
  assert.equal(t.anims.length, 1);
  const a = t.anims[0];
  assert.equal(a.opts.duration, 12000);
  assert.equal(a.currentTime, 1000);
  assert.equal(a.calls.at(-1), 'pause');

  v.paused = false;
  v.readyState = 4;
  v.fire('playing');
  assert.equal(a.calls.at(-1), 'play');

  v.paused = true;
  v.fire('pause');
  assert.equal(a.calls.at(-1), 'pause');

  v.paused = false;
  v.fire('playing');
  v.currentTime = 5;
  v.fire('seeked');
  assert.equal(a.currentTime, 5000);
  assert.equal(a.calls.at(-1), 'play');

  v.playbackRate = 2;
  v.fire('ratechange');
  assert.equal(a.playbackRate, 2);

  v.readyState = 2;
  v.fire('waiting');
  assert.equal(a.calls.at(-1), 'pause');
  assert.equal(t.anims.length, 1);
});

test('video bar re-syncs only past the drift limit (V106-20)', () => {
  const t = fakeEl();
  const v = new FakeVideo();
  v.duration = 10;
  v.paused = false;
  v.readyState = 4;
  video(t, v);
  const a = t.anims[0];
  const before = a.calls.length;
  a.currentTime = 2000;
  v.currentTime = 2 + (DRIFT_MS - 50) / 1000;
  v.fire('timeupdate');
  assert.equal(a.calls.length, before);
  assert.equal(a.currentTime, 2000);
  v.currentTime = 2 + (DRIFT_MS + 50) / 1000;
  v.fire('timeupdate');
  assert.equal(a.calls.length, before + 1);
  assert.equal(a.currentTime, v.currentTime * 1000);
});

test('video bar stop removes every listener and cancels (V106-20)', () => {
  const t = fakeEl();
  const v = new FakeVideo();
  v.duration = 10;
  const { stop } = video(t, v);
  const a = t.anims[0];
  stop();
  assert.equal(v.listeners, 0);
  assert.equal(a.calls.at(-1), 'cancel');
  const n = a.calls.length;
  v.currentTime = 7;
  for (const e of ['loadedmetadata', 'durationchange', 'playing', 'pause', 'waiting', 'seeked', 'ratechange', 'timeupdate']) v.fire(e);
  assert.equal(a.calls.length, n);
  assert.equal(a.currentTime, 0);
  assert.equal(t.anims.length, 1);
});
