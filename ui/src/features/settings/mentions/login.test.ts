import assert from 'node:assert/strict';
import { test } from 'node:test';
import { loginProblem } from './login.ts';

test('names, @names, lists and blanks pass', () => {
  for (const t of ['', 'nightbot', '@Nightbot', 'a_b9', 'a, b', 'x'.repeat(25)]) assert.equal(loginProblem(t), null);
});
test('what is not a login is flagged', () => {
  for (const t of ['bad name!', 'a-b', 'x'.repeat(26), 'ünï', 'a, b?']) assert.equal(loginProblem(t), 'Not a Twitch name.');
});
