import assert from 'node:assert/strict';
import { test } from 'node:test';
import { lineBadges } from './sharedBadges.ts';

const b = (set: string, version = '1') => ({ set, version });

test('no source: the sender list as it is', () => {
  const s = [b('premium'), b('subscriber', '3')];
  assert.equal(lineBadges(undefined, s), s);
});

test('a global set on both lists shows once', () => {
  const out = lineBadges({ roomId: '9', badges: [b('premium'), b('subscriber', '12')] }, [b('premium'), b('turbo')]);
  assert.deepEqual(
    out.map((x) => x.set),
    ['premium', 'subscriber', 'turbo'],
  );
});

test('source badges keep the source room, sender ones none', () => {
  const out = lineBadges({ roomId: '9', badges: [b('subscriber', '12')] }, [b('subscriber', '3'), b('turbo')]);
  assert.deepEqual(out, [{ set: 'subscriber', version: '12', room: '9' }, b('turbo')]);
});

test('a set twice inside one list shows once', () => {
  const out = lineBadges({ roomId: '9', badges: [b('bits', '100'), b('bits', '1000')] }, []);
  assert.equal(out.length, 1);
  assert.equal(out[0].version, '100');
});

test('live record: host subscriber/0 gives way to the source subscriber/3, shared sets once', () => {
  const sender = [b('subscriber', '0'), b('winston-overwatch')];
  const source = { roomId: '58662950', badges: [b('moderator'), b('subscriber', '3'), b('winston-overwatch')] };
  const out = lineBadges(source, sender).map((x) => `${x.set}/${x.version}`);
  assert.deepEqual(out, ['moderator/1', 'subscriber/3', 'winston-overwatch/1']);
});
