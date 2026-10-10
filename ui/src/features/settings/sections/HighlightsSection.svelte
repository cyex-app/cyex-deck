<!-- Highlights (2.6): built-in ones, then your own by phrase, user, badge and app (C26).
     Each can color the line, play a sound, flash the taskbar and copy the message into a
     feed tab (F4). Also: who never highlights, chats that stay quiet (C27), look-alike
     letters (C32). -->
<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { AppRule } from '../../../ipc/bindings/AppRule';
  import type { BadgeRule } from '../../../ipc/bindings/BadgeRule';
  import type { BuiltIn } from '../../../ipc/bindings/BuiltIn';
  import type { Highlights } from '../../../ipc/bindings/Highlights';
  import type { PhraseRule } from '../../../ipc/bindings/PhraseRule';
  import type { UserRule } from '../../../ipc/bindings/UserRule';
  import { settings } from '../../../lib/settings.svelte';
  import { splitList, type Field } from '../rules/fields';
  import RuleList from '../rules/RuleList.svelte';
  import RuleRow from '../rules/RuleRow.svelte';

  const h = $derived(settings.v.highlights);
  const save = (patch: Partial<Highlights>) => void settings.patch('highlights', patch);

  type Alert = { color: string; sound: boolean; flash: boolean; route: string };
  const ALERT: Field<Alert>[] = [
    { key: 'color', label: 'Color', kind: 'color' },
    { key: 'sound', label: 'Sound', kind: 'check' },
    { key: 'flash', label: 'Flash', kind: 'check' },
    { key: 'route', label: 'Also show in feed', kind: 'text', placeholder: 'Feed', width: '90px' },
  ];
  const alert = <T extends Alert>() => ALERT as unknown as Field<T>[];
  const on = <T,>(): Field<T & { enabled: boolean }> => ({ key: 'enabled', label: 'On', kind: 'check' });
  const blank = { enabled: true, color: '', sound: false, flash: false, route: '' };

  /** Colors match the chat's own (Line.svelte). */
  const BUILT_IN: { key: keyof Highlights; label: string; color: string }[] = [
    { key: 'mention', label: 'Your name', color: 'var(--accent)' },
    { key: 'firstMessage', label: 'First messages', color: 'var(--green)' },
    { key: 'redeemed', label: 'Highlighted with points', color: 'var(--twitch)' },
    { key: 'subs', label: 'Subs and gifts', color: 'var(--twitch)' },
    { key: 'announcements', label: 'Announcements', color: 'var(--twitch)' },
    { key: 'watchStreaks', label: 'Watch streaks', color: 'var(--cyan)' },
    { key: 'own', label: 'Your own messages', color: 'var(--accent-hover)' },
  ];
  const builtInFields: Field<BuiltIn>[] = [on<BuiltIn>(), ...alert<BuiltIn>()];

  const phraseFields: Field<PhraseRule>[] = [
    on<PhraseRule>(),
    { key: 'pattern', label: 'Words', kind: 'text', placeholder: 'Words or a regex' },
    { key: 'regex', label: 'Regex', kind: 'check' },
    { key: 'caseSensitive', label: 'Aa', kind: 'check' },
    ...alert<PhraseRule>(),
  ];
  const userFields: Field<UserRule>[] = [
    on<UserRule>(),
    { key: 'login', label: 'Name', kind: 'text', placeholder: 'Login' },
    { key: 'regex', label: 'Regex', kind: 'check' },
    ...alert<UserRule>(),
  ];
  const badgeFields: Field<BadgeRule>[] = [
    on<BadgeRule>(),
    { key: 'badge', label: 'Badge', kind: 'text', placeholder: 'moderator, subscriber/3000…' },
    ...alert<BadgeRule>(),
  ];
  const APPS = [
    { value: 'web', label: 'Browser' },
    { value: 'android', label: 'Android' },
    { value: 'ios', label: 'iPhone' },
    { value: 'mobile', label: 'Phone app' },
    { value: 'odd', label: 'Odd client' },
  ] as const;
  const appFields: Field<AppRule>[] = [
    on<AppRule>(),
    { key: 'app', label: 'Sent from', kind: 'select', options: APPS },
    ...alert<AppRule>(),
  ];
</script>

<h2>Highlights</h2>

<h3 class="label">Built in</h3>
{#each BUILT_IN as b (b.key)}
  <RuleRow
    title={b.label}
    item={h[b.key] as BuiltIn}
    fields={builtInFields}
    fallback={b.color}
    onchange={(v) => save({ [b.key]: v } as Partial<Highlights>)}
  />
{/each}

<h3 class="label">Words</h3>
<RuleList
  items={h.phrases}
  fields={phraseFields}
  make={() => ({ ...blank, pattern: '', regex: false, caseSensitive: false })}
  onchange={(phrases) => save({ phrases })}
  empty="Messages with these words light up."
/>

<h3 class="label">People</h3>
<RuleList
  items={h.users}
  fields={userFields}
  make={() => ({ ...blank, login: '', regex: false })}
  onchange={(users) => save({ users })}
  empty="Everything these people say lights up."
/>

<h3 class="label">Badges</h3>
<RuleList
  items={h.badges}
  fields={badgeFields}
  make={() => ({ ...blank, badge: '' })}
  onchange={(badges) => save({ badges })}
/>

<h3 class="label">Sent from</h3>
<RuleList
  items={h.apps}
  fields={appFields}
  make={(): AppRule => ({ ...blank, app: 'mobile' })}
  onchange={(apps) => save({ apps })}
  empty="Messages sent from a phone or an odd client."
/>

<h3 class="label">Quiet</h3>
<Row label="Never highlight" hint="These names never light up or play sounds, e.g. bots." stacked>
  <input
    class="field"
    placeholder="nightbot, streamelements"
    value={h.never.join(', ')}
    onchange={(e) => save({ never: splitList(e.currentTarget.value.toLowerCase()) })}
  />
</Row>
<Row label="Silent chats" hint="These chats never play sounds or flash. Also in a chat's menu." stacked>
  <input
    class="field"
    placeholder="Channel names"
    value={h.muted.join(', ')}
    onchange={(e) => save({ muted: splitList(e.currentTarget.value.toLowerCase()) })}
  />
</Row>
<Row label="Look-alike letters" hint="Words match even when written with look-alike letters, like Cyrillic а or 𝐟𝐚𝐧𝐜𝐲 text.">
  <Toggle label="Look-alike letters" checked={h.lookalikes} onchange={(v) => save({ lookalikes: v })} />
</Row>

<style>
  .field {
    width: 100%;
  }
</style>
