import assert from 'node:assert/strict';
import { test } from 'node:test';

import { customEntry, fill, kindsFor, makeAction, placeholdersFor, type Runners, validateCustom } from './custom.ts';
import type { CustomNode } from './types.ts';

const values = { nick: 'a b&c', 'display-nick': 'Ä b', 'user-id': '42', chan: 'xqc', msg: 'hi there?', 'msg-id': 'm-1' };

test('placeholders fill as text or as URL-encoded values', () => {
  assert.equal(fill('/me waves at $(display-nick) in $(chan)', values), '/me waves at Ä b in xqc');
  assert.equal(fill('https://x.tv/?q=$(nick)&m=$(msg)', values, 'url'), 'https://x.tv/?q=a%20b%26c&m=hi%20there%3F');
});

test('unknown names stay, missing values are empty, values are not expanded again', () => {
  assert.equal(fill('$(foo) $(nick)', {}), '$(foo) ');
  assert.equal(fill('$(nick) $(chan)', { nick: '$(chan)', chan: 'x' }), '$(chan) x');
});

test('each menu offers its own placeholders', () => {
  assert.deepEqual(placeholdersFor('chat.message'), ['nick', 'display-nick', 'user-id', 'msg-id', 'msg', 'chan']);
  assert.ok(placeholdersFor('chat.name').includes('msg'));
  assert.deepEqual(placeholdersFor('chat.name', { viewerList: true }), ['nick', 'display-nick', 'user-id', 'chan']);
  assert.deepEqual(placeholdersFor('tabs.bar'), []);
  assert.deepEqual(placeholdersFor('nope'), []);
  assert.ok(!placeholdersFor('whispers.thread').includes('chan'));
});

test('only chat menus can run a command or fill the message box', () => {
  assert.deepEqual(kindsFor('chat.name'), ['link', 'chat', 'prefill', 'copy']);
  assert.deepEqual(kindsFor('tabs.tab'), ['link', 'copy']);
});

test('validation says what is wrong', () => {
  const link = (url: string) => makeAction('link', url);
  assert.equal(validateCustom('chat.name', 'Look up', link('https://x.tv/$(nick)')), null);
  assert.equal(validateCustom('chat.name', ' ', link('https://x.tv')), 'Give it a name.');
  assert.equal(validateCustom('chat.name', 'x', link('')), 'Add a link.');
  assert.match(validateCustom('chat.name', 'x', link('javascript:alert(1)')) ?? '', /http/);
  assert.match(validateCustom('chat.name', 'x', link('file:///c:/a')) ?? '', /http/);
  assert.match(validateCustom('chat.name', 'x', link('https://$(nick).tv')) ?? '', /after the address/);
  assert.match(validateCustom('chat.name', 'x', link('https://x.tv/$(nope)')) ?? '', /isn’t a placeholder/);
  assert.match(validateCustom('chat.name', 'x', link('https://x.tv/$(msg)'), { viewerList: true }) ?? '', /isn’t available/);
  assert.match(validateCustom('tabs.tab', 'x', makeAction('chat', '/me hi')) ?? '', /can’t do that/);
  assert.equal(validateCustom('chat.name', 'x', makeAction('chat', '/me hi')), null);
  assert.equal(validateCustom('chat.name', 'x', makeAction('copy', '  ')), 'Add the text.');
});

function runners(): Runners & { calls: string[] } {
  const calls: string[] = [];
  return {
    calls,
    openLink: (u) => void calls.push(`link ${u}`),
    chat: (t) => void calls.push(`chat ${t}`),
    prefill: (t) => void calls.push(`prefill ${t}`),
    copy: (t) => void calls.push(`copy ${t}`),
  };
}

const node = (action: CustomNode['action'], icon: string | null = null): CustomNode => ({ t: 'custom', id: 'custom.1', label: 'Go', icon, hidden: false, action });

test('picking an entry does its action with the values filled', () => {
  const r = runners();
  for (const a of [
    makeAction('link', 'https://x.tv/u/$(nick)'),
    makeAction('chat', '/shoutout $(nick)'),
    makeAction('prefill', '@$(display-nick) '),
    makeAction('copy', '$(chan)/$(msg-id)'),
  ]) {
    const e = customEntry(node(a), values, r);
    assert.ok(e !== 'separator');
    (e as { run: () => void }).run();
  }
  assert.deepEqual(r.calls, ['link https://x.tv/u/a%20b%26c', 'chat /shoutout a b&c', 'prefill @Ä b ', 'copy xqc/m-1']);
});

test('a link never opens anything but the web', () => {
  const r = runners();
  const e = customEntry(node({ kind: 'link', url: 'javascript:alert(1)' }), values, r);
  (e as { run: () => void }).run();
  assert.deepEqual(r.calls, []);
});

test('entries have an icon, their own or one for the kind', () => {
  const r = runners();
  assert.equal((customEntry(node(makeAction('copy', 'x')), values, r) as { icon?: string }).icon, 'copy');
  assert.equal((customEntry(node(makeAction('copy', 'x'), 'star'), values, r) as { icon?: string }).icon, 'star');
  assert.equal((customEntry(node(makeAction('chat', 'x')), values, r) as { id?: string }).id, 'custom.1');
});
