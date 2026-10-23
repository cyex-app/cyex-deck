<!-- Highlights (2.6, V106-33): mentions live here now. Built-in rows first (your name and
     replies, V104-01, then the rest), then your own by words, people, badges and app (C26),
     global mentions (V104-02) and what stays quiet (C27).
     Each row can color the line, play a sound or its own file (V105-17), flash the taskbar,
     go to the Mentions tab, show a pop-up and copy the message into a custom feed (F4).
     Look-alike letters are per word row; the global switch is on the Ignores page (C32). -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { AppRule } from '../../../ipc/bindings/AppRule';
  import type { BadgeRule } from '../../../ipc/bindings/BadgeRule';
  import type { BuiltIn } from '../../../ipc/bindings/BuiltIn';
  import type { GlobalStatus } from '../../../ipc/bindings/GlobalStatus';
  import type { Highlights } from '../../../ipc/bindings/Highlights';
  import type { MentionCheck } from '../../../ipc/bindings/MentionCheck';
  import type { PhraseRule } from '../../../ipc/bindings/PhraseRule';
  import type { RegexFlavor } from '../../../ipc/bindings/RegexFlavor';
  import type { UserRule } from '../../../ipc/bindings/UserRule';
  import { mentionsIpc } from '../../../ipc/mentions';
  import { accounts } from '../../../lib/accounts.svelte';
  import { settings } from '../../../lib/settings.svelte';
  import { mentionsTab } from '../../chat/mentionsTab.svelte';
  import HighlightImport from '../chatterino/HighlightImport.svelte';
  import { loginProblem } from '../mentions/login';
  import { goesNowhere, statusText } from '../mentions/status';
  import type { Field } from '../rules/fields';
  import { OPT } from '../rules/options';
  import RuleList from '../rules/RuleList.svelte';
  import RuleRow from '../rules/RuleRow.svelte';

  const h = $derived(settings.v.highlights);
  const save = (patch: Partial<Highlights>) => void settings.patch('highlights', patch);
  const inFront = $derived(settings.v.general.sound.playInFront);

  type Alert = { color: string; sound: boolean; soundFile: string; flash: boolean; show: boolean; notify: boolean; route: string };
  const ALERT: Field<Alert>[] = [
    { key: 'color', kind: 'color', alert: true, ...OPT.color },
    { key: 'sound', kind: 'check', alert: true, ...OPT.sound },
    { key: 'soundFile', kind: 'sound', alert: true, ...OPT.soundFile },
    { key: 'flash', kind: 'check', alert: true, ...OPT.flash },
    { key: 'show', kind: 'check', alert: true, ...OPT.show },
    { key: 'notify', kind: 'check', alert: true, ...OPT.notify },
    { key: 'route', kind: 'text', placeholder: 'Custom feed', width: '100px', alert: true, ...OPT.route },
  ];
  const alert = <T extends Alert>() => ALERT as unknown as Field<T>[];
  const on = <T,>(): Field<T & { enabled: boolean }> => ({ key: 'enabled', label: 'On', kind: 'check' });
  const blank = { enabled: true, color: '', sound: false, soundFile: '', flash: false, show: false, notify: false, route: '' };

  /** Q2 Fixed rows. Your name and replies always go to the Mentions tab (Q4, O1), so no
   * option for it; off turns them off everywhere (O2). Colors match the chat's (Line.svelte). */
  const BUILT_IN: { key: keyof Highlights; label: string; color: string; mention?: true }[] = [
    { key: 'me', label: 'Your name', color: 'var(--accent-line)', mention: true },
    { key: 'replies', label: 'Replies to you', color: 'var(--accent-line)', mention: true },
    { key: 'firstMessage', label: 'First messages', color: 'var(--green)' },
    { key: 'redeemed', label: 'Highlighted with points', color: 'var(--twitch)' },
    { key: 'subs', label: 'Subs and gifts', color: 'var(--twitch)' },
    { key: 'announcements', label: 'Announcements', color: 'var(--twitch)' },
    { key: 'watchStreaks', label: 'Watch streaks', color: 'var(--cyan)' },
    { key: 'own', label: 'Your own messages', color: 'var(--accent-hover)' },
  ];
  const builtInFields: Field<BuiltIn>[] = [on<BuiltIn>(), ...alert<BuiltIn>()];
  // No "show" option here, but its slot stays so the swatch lines up with the other rows.
  const mentionFields = builtInFields.map((f) => (f.key === 'show' ? { ...f, spacer: true } : f));

  const FLAVORS: { value: RegexFlavor; label: string }[] = [
    { value: 'rust', label: 'Rust' },
    { value: 'javascript', label: 'JS' },
    { value: 'python', label: 'Python' },
  ];
  const wordFields = (r: PhraseRule): Field<PhraseRule>[] => [
    on<PhraseRule>(),
    { key: 'pattern', label: 'Words or a regex', kind: 'text', placeholder: r.regex ? 'A regex' : 'Words', main: true, mono: r.regex },
    { key: 'wholeWord', kind: 'check', ...OPT.wholeWord },
    { key: 'caseSensitive', kind: 'check', ...OPT.caseSensitive },
    { key: 'lookalikes', kind: 'check', ...OPT.lookalikes },
    { key: 'regex', kind: 'check', ...OPT.regex },
    { key: 'flavor', kind: 'select', options: FLAVORS, width: '96px', when: (x) => x.regex, ...OPT.flavor },
    ...alert<PhraseRule>(),
  ];
  /** Q12 New word rows go to the Mentions tab: what most people mean by a mention. */
  const newWord = (): PhraseRule => ({
    ...blank,
    pattern: '',
    regex: false,
    flavor: 'rust',
    caseSensitive: false,
    wholeWord: true,
    lookalikes: false,
    show: true,
  });

  const userFields: Field<UserRule>[] = [
    on<UserRule>(),
    { key: 'login', label: 'Name', kind: 'text', placeholder: 'Username', main: true },
    { key: 'regex', kind: 'check', ...OPT.regex, hint: 'The name is a regular expression.' },
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
      main: true,
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
    { key: 'app', label: 'Sent from', kind: 'select', options: APPS, main: true },
    ...alert<AppRule>(),
  ];
  const appName = (a: AppRule) => APPS.find((x) => x.value === a.app)?.label ?? a.app;

  /** Patterns are checked in Rust (one engine for every flavor): errors, flavor notes, repeats. */
  function checked(list: () => PhraseRule[]) {
    let out = $state.raw<MentionCheck[]>([]);
    $effect(() => {
      const l = list();
      let live = true;
      mentionsIpc
        .check(l)
        .then((c) => live && (out = c))
        .catch(() => {});
      return () => (live = false);
    });
    return {
      problem: (_: PhraseRule, i: number) => out[i]?.error ?? null,
      note: (_: PhraseRule, i: number) => {
        const c = out[i];
        return c?.note ?? (c?.sameAs ? `Same as rule ${c.sameAs}.` : null);
      },
    };
  }
  const words = checked(() => h.phrases);
  const globals = checked(() => h.globalList);
  const ruleName = (r: PhraseRule, i: number) => r.pattern || `Rule ${i + 1}`;

  let status = $state<GlobalStatus>({ state: 'off' });
  $effect(() => {
    void mentionsIpc.globalStatus().then((s) => (status = s));
    const off = mentionsIpc.onGlobalStatus((s) => (status = s));
    return () => void off.then((f) => f());
  });
  /** V107-Mq One login per row; Rust lowercases, splits "a, b" and drops repeats. */
  type Login = { login: string };
  const rows = (l: string[]): Login[] => l.map((login) => ({ login }));
  const logins = (r: Login[]) => r.map((x) => x.login);
  const neverFields: Field<Login>[] = [{ key: 'login', label: 'Name', kind: 'text', placeholder: 'Username, like nightbot', main: true }];
  const mutedFields: Field<Login>[] = [{ key: 'login', label: 'Channel', kind: 'text', placeholder: 'Channel name', main: true }];
  /** V107-H16 Global rows pick the Mentions tab and/or a feed, like word rows; a row
   * with neither says so. Your names are looked for too (V107-Mq). */
  const globalProblem = (r: PhraseRule, i: number) => globals.problem(r, i) ?? goesNowhere(r);
  const hasGlobal = $derived(h.globalList.some((x) => x.enabled && x.pattern.trim()) || (h.me.enabled && accounts.list.length > 0));
</script>

<SectionHead title="Highlights" level={2} setting="highlights:import">
  Make messages stand out, ping you, or copy them to the Mentions tab or a feed. For a feed, type its name.
  {#snippet actions()}
    <Button small icon="list" onclick={() => mentionsTab.open()}>Open the Mentions tab</Button>
    <HighlightImport />
  {/snippet}
</SectionHead>
{#if !inFront}
  <p class="intro more">Sounds only play while you’re in another app. Settings → Sound can change that.</p>
{/if}

<SectionHead title="Built in" />
{#each BUILT_IN as b (b.key)}
  <RuleRow
    setting={`highlights.${b.key}`}
    title={b.label}
    item={h[b.key] as BuiltIn}
    fields={b.mention ? mentionFields : builtInFields}
    layout="split"
    gripSlot
    fallback={b.color}
    note={b.mention ? 'Always goes to the Mentions tab while on.' : null}
    onchange={(v) => save({ [b.key]: v } as Partial<Highlights>)}
  />
{/each}

<SectionHead title="Words">Highlight messages that contain these.</SectionHead>
<RuleList
  setting="highlights.phrases"
  items={h.phrases}
  fields={wordFields}
  layout="split"
  mainMin="110px"
  fallback="var(--accent)"
  make={newWord}
  problem={words.problem}
  note={words.note}
  reorder={{ key: 'highlights.phrases', label: ruleName }}
  onchange={(phrases) => save({ phrases })}
  empty="No words yet."
  emptyIcon="star"
/>

<SectionHead title="People">Highlight everything these users write.</SectionHead>
<RuleList
  setting="highlights.users"
  items={h.users}
  fields={userFields}
  slots={2}
  layout="split"
  make={() => ({ ...blank, login: '', regex: false })}
  reorder={{ key: 'highlights.users', label: (r, i) => r.login || `Rule ${i + 1}` }}
  onchange={(users) => save({ users })}
  empty="No people yet."
  emptyIcon="user"
/>

<SectionHead title="Badges">Highlight messages from people with these badges.</SectionHead>
<RuleList
  setting="highlights.badges"
  items={h.badges}
  fields={badgeFields}
  layout="split"
  make={() => ({ ...blank, badge: '' })}
  reorder={{ key: 'highlights.badges', label: (r, i) => r.badge || `Rule ${i + 1}` }}
  onchange={(badges) => save({ badges })}
  empty="No badges yet."
  emptyIcon="badge"
/>

<SectionHead title="Sent from">Highlight messages sent from these apps.</SectionHead>
<RuleList
  setting="highlights.apps"
  items={h.apps}
  fields={appFields}
  layout="split"
  make={(): AppRule => ({ ...blank, app: 'mobile' })}
  reorder={{ key: 'highlights.apps', label: appName }}
  onchange={(apps) => save({ apps })}
  empty="No apps yet."
  emptyIcon="smartphone"
/>

<SectionHead title="Global mentions">Find out when you’re mentioned in chats you aren’t in. Your names count while Your name is on. Each row goes to the Mentions tab, a feed, or both.</SectionHead>
<Row setting="highlights.global" label="Global mentions" hint="Searches every chat logs.cyex.app keeps. Uses about 0,8 GB an hour while on. Your list stays on this PC.">
  <Toggle label="Global mentions" checked={h.global} onchange={(v) => save({ global: v })} />
</Row>
{#if h.global}
  <p class="status" class:live={status.state === 'connected'}>
    <span class="dot"></span>{statusText(status, hasGlobal)}
  </p>
  <RuleList
    setting="highlights.globalList"
    items={h.globalList}
    fields={wordFields}
    layout="split"
    mainMin="110px"
    fallback="var(--accent)"
    make={newWord}
    problem={globalProblem}
    note={globals.note}
    reorder={{ key: 'highlights.globalList', label: ruleName }}
    onchange={(globalList) => save({ globalList })}
    empty="More words to find in any chat, open or not."
    emptyIcon="at"
  />
{/if}

<SectionHead title="Never highlight" setting="highlights.never">These names never light up or play sounds, e.g. bots.</SectionHead>
<RuleList
  items={rows(h.never)}
  fields={neverFields}
  layout="split"
  make={() => ({ login: '' })}
  problem={(r) => loginProblem(r.login)}
  deleted="Name removed."
  onchange={(r) => save({ never: logins(r) })}
  empty="No names yet."
  emptyIcon="user"
/>
<SectionHead title="Silent chats" setting="highlights.muted">These chats never play sounds or flash. Also in a chat’s menu.</SectionHead>
<RuleList
  items={rows(h.muted)}
  fields={mutedFields}
  layout="split"
  make={() => ({ login: '' })}
  problem={(r) => loginProblem(r.login)}
  deleted="Chat removed."
  onchange={(r) => save({ muted: logins(r) })}
  empty="No chats yet."
  emptyIcon="volumeOff"
/>

<style>
  .intro.more {
    margin: -2px 0 6px;
    color: var(--dim);
    font-size: var(--text-sub);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 10px 0 2px;
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--dim);
  }
  .live .dot {
    background: var(--green);
  }
</style>
