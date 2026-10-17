<!-- Automations (S4): when someone says a trigger, run a command as if you typed it.
     Off as a whole by default, with a limit per chat. -->
<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { Automation } from '../../../ipc/bindings/Automation';
  import { settings } from '../../../lib/settings.svelte';
  import type { Field } from '../rules/fields';
  import { OPT } from '../rules/options';
  import RuleList from '../rules/RuleList.svelte';
  import SectionHead from '../SectionHead.svelte';
  import NumberField from '../../../components/NumberField.svelte';

  const c = $derived(settings.v.commands);

  const fields: Field<Automation>[] = [
    { key: 'enabled', label: 'On', kind: 'check' },
    { key: 'trigger', label: 'Trigger', kind: 'text', placeholder: 'e.g. !discord', main: true },
    {
      key: 'matching',
      label: 'Match',
      hint: 'How the trigger has to appear in the message.',
      kind: 'select',
      width: '124px',
      options: [
        { value: 'command', label: 'Command' },
        { value: 'exact', label: 'Exactly' },
        { value: 'startsWith', label: 'Starts with' },
        { value: 'contains', label: 'Contains' },
        { value: 'regex', label: 'Regex' },
      ],
    },
    {
      key: 'who',
      label: 'Who',
      hint: 'Who can set it off.',
      kind: 'select',
      width: '110px',
      alert: true,
      options: [
        { value: 'everyone', label: 'Everyone' },
        { value: 'subscribers', label: 'Subs' },
        { value: 'vips', label: 'VIPs' },
        { value: 'moderators', label: 'Mods' },
        { value: 'broadcaster', label: 'Streamer' },
      ],
    },
    { key: 'reply', kind: 'check', alert: true, ...OPT.reply },
    { key: 'liveOnly', kind: 'check', alert: true, ...OPT.liveOnly },
    {
      key: 'cooldownSecs',
      label: 'Cooldown in seconds',
      hint: 'Least time between two answers in a chat.',
      caption: 'Cooldown',
      unit: 's',
      kind: 'number',
      alert: true,
    },
    {
      key: 'userCooldownSecs',
      label: 'Cooldown per person in seconds',
      hint: 'Least time before the same person can set it off again.',
      caption: 'Per person',
      unit: 's',
      kind: 'number',
      alert: true,
    },
    { key: 'name', label: 'Name', kind: 'text', placeholder: 'Name', width: '130px', wide: true },
    { key: 'channels', label: 'Chats', kind: 'list', placeholder: 'Chats it works in (all if empty)', wide: true },
    {
      key: 'response',
      label: 'Response',
      kind: 'area',
      placeholder: 'e.g. Join us: discord.gg/…   $(nick) is who said it, $1- is what followed',
      wide: true,
    },
  ];

  const make = (): Automation => ({
    enabled: true,
    name: '',
    channels: [],
    trigger: '',
    matching: 'command',
    response: '',
    reply: false,
    who: 'everyone',
    liveOnly: false,
    cooldownSecs: 30,
    userCooldownSecs: 0,
    count: 0,
  });
</script>

<SectionHead title="Automations" level={2} docs="automations">When someone says the trigger, the response runs as if you typed it.</SectionHead>

<Row setting="commands.automationsOn" label="Run automations" hint="They answer as your account, so keep them friendly and slow.">
  <Toggle
    label="Run automations"
    checked={c.automationsOn}
    onchange={(v) => void settings.patch('commands', { automationsOn: v })}
  />
</Row>
<Row setting="commands.automationLimit" label="At most" hint="Automatic messages per chat in 30 seconds.">
  <NumberField label="At most" min={1} max={10} width="80px" value={c.automationLimit} onchange={(n) => void settings.patch('commands', { automationLimit: n })} />
</Row>

<RuleList
  setting="commands.automations"
  items={c.automations}
  {fields}
  layout="split"
  emptyIcon="zap"
  {make}
  onchange={(automations) => void settings.patch('commands', { automations })}
  addLabel="Add automation"
  empty="No automations yet."
  deleted="Automation deleted."
  reorder={{ key: 'commands.automations', label: (a, i) => a.name || a.trigger || `Automation ${i + 1}` }}
/>
