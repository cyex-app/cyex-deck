<!-- V107-R38 Your channel's moderation settings, as on Twitch's Settings › Moderation:
     AutoMod (a preset or per category), chat delay, email and phone verification, chat
     rules, unban requests, blocked and allowed terms. Each change is saved right away and
     the panel shows what Twitch read back. Slow, followers, sub and emote modes are in Chat
     settings (1.35). GQL only. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import Card from '../../components/PanelCard.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import Select from '../../components/Select.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import type { BlockedTerm } from '../../ipc/bindings/BlockedTerm';
  import type { ModsetChange } from '../../ipc/bindings/ModsetChange';
  import type { ModsetSettings } from '../../ipc/bindings/ModsetSettings';
  import type { ModsetVerification } from '../../ipc/bindings/ModsetVerification';
  import type { ModsetVerifyMode } from '../../ipc/bindings/ModsetVerifyMode';
  import { modsetIpc } from '../../ipc/modset';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { termProblem } from '../modview/terms';
  import {
    ACCOUNT_AGES,
    CATEGORIES,
    COOLDOWNS,
    DELAYS,
    EXEMPT,
    FOLLOW_AGES,
    LEVELS,
    levelsFor,
    MAX_RULE,
    minuteChoices,
    MODES,
    ruleProblem,
    type Who,
    withMode,
    withoutRule,
    withPartial,
    withRule,
  } from './modset';

  let s = $state<ModsetSettings | null>(null);
  let failed = $state(false);
  let busy = $state(false);
  let showCategories = $state(false);
  let rule = $state('');
  let termList = $state<'blocked' | 'permitted'>('blocked');
  let term = $state('');

  function load() {
    failed = false;
    modsetIpc
      .read()
      .then((r) => (s = r))
      .catch((e) => {
        failed = true;
        toast.error(errorText(e));
      });
  }
  load();

  async function run(call: () => Promise<ModsetSettings>): Promise<boolean> {
    busy = true;
    try {
      s = await call();
      return true;
    } catch (e) {
      toast.error(errorText(e));
      return false;
    } finally {
      busy = false;
    }
  }

  const set = (change: ModsetChange) => run(() => modsetIpc.set(change));
  const verify = (v: ModsetVerification) => set({ kind: 'verification', verification: v });

  async function addRule() {
    if (!s) return;
    const why = ruleProblem(rule, s.rules);
    if (why) return toast.error(why);
    if (await set({ kind: 'rules', rules: withRule(s.rules, rule) })) rule = '';
  }

  const terms = $derived<BlockedTerm[]>(s ? (termList === 'permitted' ? s.permitted : s.blocked) : []);

  async function addTerm() {
    const why = termProblem(term, termList === 'blocked' ? terms : []);
    if (why) return toast.error(why);
    if (await run(() => modsetIpc.addTerm(termList === 'permitted', term.trim()))) term = '';
  }
</script>

{#snippet who(label: string, key: Who, v: ModsetVerification)}
  <div class="row">
    <span>{label}</span>
    <Select
      label="{label}: who must verify"
      width="112px"
      value={v[key].mode}
      disabled={busy}
      options={MODES}
      onchange={(m) => void verify(withMode(v, key, m as ModsetVerifyMode))}
    />
  </div>
  {#if v[key].mode === 'PARTIAL'}
    {@const p = v[key].partial}
    <div class="sub">
      <Checkbox label="First-time chatters" checked={p.firstTime} disabled={busy} onchange={(on) => void verify(withPartial(v, key, { firstTime: on }))} />
      <div class="row">
        <Checkbox
          label="Accounts younger than"
          checked={p.accountYoungerThan != null}
          disabled={busy}
          onchange={(on) => void verify(withPartial(v, key, { accountYoungerThan: on ? ACCOUNT_AGES[1] : null }))}
        />
        {#if p.accountYoungerThan != null}
          <Select
            label="Account age"
            width="88px"
            value={String(p.accountYoungerThan)}
            disabled={busy}
            options={minuteChoices(ACCOUNT_AGES, p.accountYoungerThan)}
            onchange={(m) => void verify(withPartial(v, key, { accountYoungerThan: Number(m) }))}
          />
        {/if}
      </div>
      <div class="row">
        <Checkbox
          label="Following less than"
          checked={p.followingLessThan != null}
          disabled={busy}
          onchange={(on) => void verify(withPartial(v, key, { followingLessThan: on ? FOLLOW_AGES[0] : null }))}
        />
        {#if p.followingLessThan != null}
          <Select
            label="Follow age"
            width="88px"
            value={String(p.followingLessThan)}
            disabled={busy}
            options={minuteChoices(FOLLOW_AGES, p.followingLessThan)}
            onchange={(m) => void verify(withPartial(v, key, { followingLessThan: Number(m) }))}
          />
        {/if}
      </div>
    </div>
  {/if}
{/snippet}

<Card title="Moderation settings">
  {#snippet tools()}
    {#if s}<IconButton icon="refresh" label="Refresh" size={14} disabled={busy} onclick={load} />{/if}
  {/snippet}
  {#if s}
    {@const v = s.verification}
    <section>
      <h4 class="label">AutoMod</h4>
      <Segmented label="AutoMod level" options={LEVELS} value={s.automod.overall ?? -1} onchange={(level) => !busy && void set({ kind: 'autoMod', level })} />
      {#if s.automod.overall == null}<p class="hint">Custom levels.</p>{/if}
      <button class="more" aria-expanded={showCategories} onclick={() => (showCategories = !showCategories)}>
        <Icon name={showCategories ? 'chevronUp' : 'chevronDown'} size={12} />
        By category
      </button>
      {#if showCategories}
        {#each CATEGORIES as name, i (name)}
          <div class="row">
            <span>{name}</span>
            <Select
              label={name}
              width="96px"
              value={String(s.automod.levels[i] ?? 0)}
              disabled={busy}
              options={levelsFor(i)}
              onchange={(l) => void set({ kind: 'category', index: i, level: Number(l) })}
            />
          </div>
        {/each}
      {/if}
    </section>

    <section>
      <h4 class="label">Chat delay</h4>
      <Segmented label="Chat delay" options={DELAYS} value={s.delay} onchange={(seconds) => !busy && void set({ kind: 'delay', seconds })} />
      <p class="hint">Messages show this much later for everyone but mods.</p>
    </section>

    <section>
      <h4 class="label">Verified accounts</h4>
      {@render who('Email', 'email', v)}
      {@render who('Phone', 'phone', v)}
      {#if v.email.mode !== 'NONE' || v.phone.mode !== 'NONE'}
        <p class="hint">Not needed for</p>
        <div class="exempt">
          {#each EXEMPT as e (e.key)}
            <Checkbox label={e.label} checked={v[e.key]} disabled={busy} onchange={(on) => void verify({ ...v, [e.key]: on })} />
          {/each}
        </div>
      {/if}
    </section>

    <section>
      <h4 class="label">Chat rules</h4>
      {#if s.rules.length}
        <ol>
          {#each s.rules as r, i (i)}
            <li>
              <span class="text">{r}</span>
              <IconButton icon="x" label="Remove rule" size={12} disabled={busy} onclick={() => s && void set({ kind: 'rules', rules: withoutRule(s.rules, i) })} />
            </li>
          {/each}
        </ol>
      {:else}
        <p class="hint">No rules. New chatters see them before their first message.</p>
      {/if}
      <form
        class="add"
        onsubmit={(e) => {
          e.preventDefault();
          void addRule();
        }}
      >
        <input class="field grow" placeholder="Add a rule" maxlength={MAX_RULE} bind:value={rule} />
        <Button small type="submit" disabled={busy || !rule.trim()}>Add</Button>
      </form>
    </section>

    <section>
      <h4 class="label">Unban requests</h4>
      <div class="row">
        <span>Banned chatters can ask</span>
        <Toggle label="Unban requests" checked={s.unbanRequests} disabled={busy} onchange={(on) => s && void set({ kind: 'unbanRequests', on, minutes: s.unbanCooldownMinutes })} />
      </div>
      {#if s.unbanRequests}
        <div class="row">
          <span>Wait after the ban</span>
          <Select
            label="Wait after the ban"
            width="96px"
            value={String(s.unbanCooldownMinutes)}
            disabled={busy}
            options={minuteChoices(COOLDOWNS, s.unbanCooldownMinutes)}
            onchange={(m) => void set({ kind: 'unbanRequests', on: true, minutes: Number(m) })}
          />
        </div>
      {/if}
    </section>

    <section>
      <h4 class="label">Terms</h4>
      <Segmented
        label="Terms"
        options={[
          { value: 'blocked', label: `Blocked ${s.blocked.length || ''}`.trim() },
          { value: 'permitted', label: `Allowed ${s.permitted.length || ''}`.trim() },
        ]}
        value={termList}
        onchange={(l) => (termList = l)}
      />
      <p class="hint">{termList === 'blocked' ? 'Never in chat.' : 'Never caught by AutoMod.'}</p>
      <form
        class="add"
        onsubmit={(e) => {
          e.preventDefault();
          void addTerm();
        }}
      >
        <input class="field grow" placeholder={termList === 'blocked' ? 'Block a term' : 'Allow a term'} maxlength="500" bind:value={term} />
        <Button small type="submit" disabled={busy || !term.trim()}>Add</Button>
      </form>
      {#if terms.length}
        <ul>
          {#each terms as t (t.id)}
            <li>
              <span class="text">{t.text}</span>
              <ConfirmButton label="Remove" confirm="Sure?" variant="ghost" disabled={busy} onconfirm={() => void run(() => modsetIpc.removeTerm(termList === 'permitted', t.id))} />
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {:else if failed}
    <p class="hint">Couldn’t load the moderation settings. <Button small variant="link" onclick={load}>Try again</Button></p>
  {:else}
    <Loading />
  {/if}
</Card>

<style>
  section {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 6px;
    padding-bottom: 12px;
  }
  section + section {
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  h4 {
    margin: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
  }
  .row > span {
    flex: 1;
    min-width: 0;
    font-size: 0.85rem;
  }
  .exempt {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
  }
  .sub {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-left: 12px;
    border-left: 2px solid var(--border);
  }
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.78rem;
  }
  .more {
    display: inline-flex;
    align-self: flex-start;
    align-items: center;
    gap: 4px;
    padding: 2px 0;
    color: var(--muted);
    font: inherit;
    font-size: 0.78rem;
    background: none;
    border: 0;
    cursor: pointer;
  }
  .more:hover {
    color: var(--text);
  }
  .add {
    display: flex;
    gap: 6px;
  }
  .add .field {
    flex: 1;
    min-width: 0;
  }
  ol,
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  ol {
    counter-reset: rule;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 0 2px 0 6px;
    border-radius: var(--radius);
  }
  ol li::before {
    counter-increment: rule;
    content: counter(rule) '.';
    flex: none;
    min-width: 1.2em;
    color: var(--muted);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
  }
  li:hover {
    background: var(--surface-hover);
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
    font-size: 0.82rem;
  }
</style>
