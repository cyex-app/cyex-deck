<!-- Highlights (2.6): built-in ones, then your own by phrase, user, badge and app (C26).
     Your name moved to Mentions (V104-01).
     Each can color the line, play a sound, flash the taskbar and copy the message into a
     feed tab (F4). Also: who never highlights, chats that stay quiet (C27), look-alike
     letters (C32).
     V105-17 Each can also go to the Mentions tab and play its own sound file. -->
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
  import HighlightImport from '../chatterino/HighlightImport.svelte';
  import RuleList from '../rules/RuleList.svelte';
  import RuleRow from '../rules/RuleRow.svelte';

  const h = $derived(settings.v.highlights);
  const save = (patch: Partial<Highlights>) => void settings.patch('highlights', patch);

  type Alert = { color: string; sound: boolean; soundFile: string; flash: boolean; show: boolean; route: string };
  const ALERT: Field<Alert>[] = [
    { key: 'color', label: 'Color', kind: 'color' },
    { key: 'sound', label: 'Sound', kind: 'check' },
    { key: 'soundFile', label: 'Sound file', kind: 'sound' },
    { key: 'flash', label: 'Flash', kind: 'check' },
    { key: 'show', label: 'Also show in Mentions', short: 'Mentions', kind: 'check' },
    { key: 'route', label: 'Also show in feed', kind: 'text', placeholder: 'Feed', width: '56px' },
  ];
  const alert = <T extends Alert>() => ALERT as unknown as Field<T>[];
  const on = <T,>(): Field<T & { enabled: boolean }> => ({ key: 'enabled', label: 'On', kind: 'check' });
  const blank = { enabled: true, color: '', sound: false, soundFile: '', flash: false, show: false, route: '' };

  /** Colors match the chat's own (Line.svelte). */
  const BUILT_IN: { key: keyof Highlights; label: string; color: string }[] = [
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
    { key: 'pattern', label: 'Words', kind: 'text', placeholder: 'Words or regex' },
    { key: 'regex', label: 'Regex', kind: 'check' },
    { key: 'caseSensitive', label: 'Match case', short: 'Aa', kind: 'check' },
    ...alert<PhraseRule>(),
  ];
  const userFields: Field<UserRule>[] = [
    on<UserRule>(),
    { key: 'login', label: 'Name', kind: 'text', placeholder: 'Username' },
    { key: 'regex', label: 'Regex', kind: 'check' },
    ...alert<UserRule>(),
  ];
  const badgeFields: Field<BadgeRule>[] = [
    on<BadgeRule>(),
    {
      key: 'badge',
      label: 'Badge',
      hint: 'A badge name. subscriber1, subscriber2 and subscriber3 match that tier; subscriber matches any tier; subscriber/3012 one exact version.',
      kind: 'text',
      placeholder: 'Badge, like subscriber2 or vip',
    },
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
    setting={`highlights.${b.key}`}
    title={b.label}
    item={h[b.key] as BuiltIn}
    fields={builtInFields}
    fallback={b.color}
    onchange={(v) => save({ [b.key]: v } as Partial<Highlights>)}
  />
{/each}

<div class="list-head" data-setting="highlights:import">
  <h3 class="label">Words</h3>
  <HighlightImport />
</div>
<RuleList
  setting="highlights.phrases"
  items={h.phrases}
  fields={phraseFields}
  make={() => ({ ...blank, pattern: '', regex: false, caseSensitive: false })}
  onchange={(phrases) => save({ phrases })}
  empty="Messages with these words light up."
/>

<h3 class="label">People</h3>
<RuleList
  setting="highlights.users"
  items={h.users}
  fields={userFields}
  make={() => ({ ...blank, login: '', regex: false })}
  onchange={(users) => save({ users })}
  empty="Everything these people say lights up."
/>

<h3 class="label">Badges</h3>
<RuleList
  setting="highlights.badges"
  items={h.badges}
  fields={badgeFields}
  make={() => ({ ...blank, badge: '' })}
  onchange={(badges) => save({ badges })}
/>

<h3 class="label">Sent from</h3>
<RuleList
  setting="highlights.apps"
  items={h.apps}
  fields={appFields}
  make={(): AppRule => ({ ...blank, app: 'mobile' })}
  onchange={(apps) => save({ apps })}
  empty="Messages sent from a phone or an odd client."
/>

<h3 class="label">Quiet</h3>
<Row setting="highlights.never" label="Never highlight" hint="These names never light up or play sounds, e.g. bots." stacked>
  <input
    class="field"
    placeholder="nightbot, streamelements"
    value={h.never.join(', ')}
    onchange={(e) => save({ never: splitList(e.currentTarget.value.toLowerCase()) })}
  />
</Row>
<Row setting="highlights.muted" label="Silent chats" hint="These chats never play sounds or flash. Also in a chat's menu." stacked>
  <input
    class="field"
    placeholder="Channel names"
    value={h.muted.join(', ')}
    onchange={(e) => save({ muted: splitList(e.currentTarget.value.toLowerCase()) })}
  />
</Row>
<Row setting="highlights.lookalikes" label="Look-alike letters" hint="Words match even when written with look-alike letters, like Cyrillic а or 𝐟𝐚𝐧𝐜𝐲 text.">
  <Toggle label="Look-alike letters" checked={h.lookalikes} onchange={(v) => save({ lookalikes: v })} />
</Row>

<style>
  .list-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin: 0.68rem 0;
  }
  .list-head h3 {
    margin: 0;
  }
  .field {
    width: 100%;
  }
</style>
