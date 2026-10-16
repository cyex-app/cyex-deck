<!-- Ignores (F2): words that hide a message or get swapped, people your account blocked
     on Twitch, and people hidden only for you (C28). -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import EmptyState from '../../../components/EmptyState.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { BlockedShow } from '../../../ipc/bindings/BlockedShow';
  import type { Highlights } from '../../../ipc/bindings/Highlights';
  import type { IgnoreRule } from '../../../ipc/bindings/IgnoreRule';
  import { settings } from '../../../lib/settings.svelte';
  import type { Field } from '../rules/fields';
  import { OPT } from '../rules/options';
  import RuleList from '../rules/RuleList.svelte';

  const h = $derived(settings.v.highlights);
  const save = (patch: Partial<Highlights>) => void settings.patch('highlights', patch);

  const fields: Field<IgnoreRule>[] = [
    { key: 'enabled', label: 'On', kind: 'check' },
    { key: 'pattern', label: 'Words', kind: 'text', placeholder: 'Words or a regex', main: true },
    { key: 'caseSensitive', kind: 'check', ...OPT.caseSensitive },
    { key: 'regex', kind: 'check', ...OPT.regex },
    { key: 'block', kind: 'check', alert: true, ...OPT.block },
    { key: 'replace', kind: 'text', placeholder: 'Swap for', width: '110px', alert: true, ...OPT.swap },
    {
      key: 'route',
      kind: 'text',
      placeholder: 'Feed tab',
      width: '88px',
      alert: true,
      label: 'Still show in a feed tab',
      hint: 'A hidden message still goes into the feed tab with this name.',
    },
  ];

  const SHOW: { value: BlockedShow; label: string }[] = [
    { value: 'never', label: 'Never' },
    { value: 'moderator', label: 'Where I mod' },
    { value: 'broadcaster', label: 'In my chat' },
  ];
</script>

<h2>Ignores</h2>

<h3 class="label">Words</h3>
<RuleList
  setting="highlights.ignores"
  items={h.ignores}
  {fields}
  layout="split"
  slots={2}
  emptyIcon="eyeOff"
  make={() => ({
    enabled: true,
    pattern: '',
    regex: false,
    caseSensitive: false,
    block: true,
    replace: '',
    route: '',
  })}
  onchange={(ignores) => save({ ignores })}
  empty="Hide messages with these words, or swap the words for something else."
/>

<h3 class="label">Blocked on Twitch</h3>
<Row setting="highlights.blocked.enabled" label="Hide people you blocked" hint="Read from each signed-in account.">
  <Toggle
    label="Hide people you blocked"
    checked={h.blocked.enabled}
    onchange={(v) => save({ blocked: { ...h.blocked, enabled: v } })}
  />
</Row>
<Row setting="highlights.blocked.show" label="Still show them" hint="Where you might need to act on what they say.">
  <Segmented
    label="Still show blocked people"
    options={SHOW}
    value={h.blocked.show}
    onchange={(v) => save({ blocked: { ...h.blocked, show: v } })}
  />
</Row>

<h3 class="label" data-setting="highlights.hiddenUsers">Hidden for you</h3>
{#each h.hiddenUsers as u (u.id)}
  <Row label={u.login}>
    <Button small variant="ghost" onclick={() => save({ hiddenUsers: h.hiddenUsers.filter((x) => x.id !== u.id) })}>
      Show again
    </Button>
  </Row>
{:else}
  <EmptyState icon="user" text="Nobody. Hide someone from their name’s menu or with /hideuser." />
{/each}
