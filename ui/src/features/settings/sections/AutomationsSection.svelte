<!-- Automations (S4): when someone says a trigger, run a command as if you typed it.
     Off as a whole by default, with a limit per chat. -->
<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { Automation } from '../../../ipc/bindings/Automation';
  import { settings } from '../../../lib/settings.svelte';
  import type { Field } from '../rules/fields';
  import RuleList from '../rules/RuleList.svelte';

  const c = $derived(settings.v.commands);

  const fields: Field<Automation>[] = [
    { key: 'enabled', label: 'On', kind: 'check' },
    { key: 'name', label: 'Name', kind: 'text', placeholder: 'Name', width: '120px' },
    { key: 'channels', label: 'Chats', kind: 'list', placeholder: 'Chats it works in' },
    {
      key: 'matching',
      label: 'Match',
      kind: 'select',
      options: [
        { value: 'command', label: 'Command' },
        { value: 'exact', label: 'Exactly' },
        { value: 'startsWith', label: 'Starts with' },
        { value: 'contains', label: 'Contains' },
        { value: 'regex', label: 'Regex' },
      ],
    },
    { key: 'trigger', label: 'Trigger', kind: 'text', placeholder: '!discord' },
    { key: 'response', label: 'Response', kind: 'area', placeholder: 'Join us: discord.gg/… · $(nick) who said it · $1- what followed' },
    {
      key: 'who',
      label: 'Who',
      kind: 'select',
      options: [
        { value: 'everyone', label: 'Everyone' },
        { value: 'subscribers', label: 'Subs' },
        { value: 'vips', label: 'VIPs' },
        { value: 'moderators', label: 'Mods' },
        { value: 'broadcaster', label: 'Streamer' },
      ],
    },
    { key: 'reply', label: 'As a reply', kind: 'check' },
    { key: 'liveOnly', label: 'Only live', kind: 'check' },
    { key: 'cooldownSecs', label: 'Cooldown, seconds', kind: 'number' },
    { key: 'userCooldownSecs', label: 'Per person, seconds', kind: 'number' },
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

<h2>Automations</h2>

<Row setting="commands.automationsOn" label="Run automations" hint="They answer as your account, so keep them friendly and slow.">
  <Toggle
    label="Run automations"
    checked={c.automationsOn}
    onchange={(v) => void settings.patch('commands', { automationsOn: v })}
  />
</Row>
<Row setting="commands.automationLimit" label="At most" hint="Automatic messages per chat in 30 seconds.">
  <input
    class="field num"
    type="number"
    aria-label="At most"
    min="1"
    max="10"
    value={c.automationLimit}
    onchange={(e) =>
      void settings.patch('commands', {
        automationLimit: Math.min(10, Math.max(1, Math.round(Number(e.currentTarget.value) || 1))),
      })}
  />
</Row>

<RuleList
  setting="commands.automations"
  items={c.automations}
  {fields}
  {make}
  onchange={(automations) => void settings.patch('commands', { automations })}
  addLabel="Add automation"
  empty="When someone says the trigger, the response runs as if you typed it."
/>

<style>
  .num {
    width: 80px;
  }
</style>
