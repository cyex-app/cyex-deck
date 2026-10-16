<!-- V104-01 One mention: what it looks for and how (whole word, case, look-alikes, regex
     and its flavor), then what it does (color, sound or its own sound file, flash, the
     Mentions tab, pop-up). "Your name" has only the second part. V105-18 The same row
     controls as Highlights (RuleRow), in its two-line layout. Errors and notes on the
     pattern show underneath. -->
<script lang="ts">
  import type { MentionAlert } from '../../../ipc/bindings/MentionAlert';
  import type { MentionCheck } from '../../../ipc/bindings/MentionCheck';
  import type { MentionRule } from '../../../ipc/bindings/MentionRule';
  import type { RegexFlavor } from '../../../ipc/bindings/RegexFlavor';
  import type { Field } from '../rules/fields';
  import RuleRow from '../rules/RuleRow.svelte';

  type Item = MentionAlert & { enabled: boolean } & Partial<MentionRule>;

  interface Props {
    item: Item;
    /** "Your name": no pattern or match options. */
    title?: string;
    check?: MentionCheck;
    onchange: (item: Item) => void;
    ondelete?: () => void;
    /** V103-13 Registry id: settings search jumps here. */
    setting?: string;
  }

  let { item, title, check, onchange, ondelete, setting }: Props = $props();

  const FLAVORS: { value: RegexFlavor; label: string }[] = [
    { value: 'rust', label: 'Rust' },
    { value: 'javascript', label: 'JavaScript' },
    { value: 'python', label: 'Python' },
  ];

  const ALERT: Field<Item>[] = [
    { key: 'color', label: 'Color', kind: 'color', alert: true },
    { key: 'sound', label: 'Sound', kind: 'check', alert: true },
    { key: 'soundFile', label: 'Sound file', kind: 'sound', alert: true },
    { key: 'flash', label: 'Flash', kind: 'check', alert: true },
    { key: 'show', label: 'Also show in Mentions', short: 'Mentions', kind: 'check', alert: true },
    { key: 'notify', label: 'Pop-up', kind: 'check', alert: true },
  ];
  const fields = $derived<Field<Item>[]>(
    title
      ? [{ key: 'enabled', label: 'On', kind: 'check' }, ...ALERT]
      : [
          { key: 'enabled', label: 'On', kind: 'check' },
          {
            key: 'pattern',
            label: 'Words or a regex',
            kind: 'text',
            placeholder: item.regex ? 'A regex' : 'Words',
            main: true,
            mono: !!item.regex,
          },
          { key: 'wholeWord', label: 'Whole word', kind: 'check' },
          { key: 'caseSensitive', label: 'Match case', short: 'Aa', kind: 'check' },
          { key: 'lookalikes', label: 'Look-alikes', kind: 'check' },
          { key: 'regex', label: 'Regex', kind: 'check' },
          {
            key: 'flavor',
            label: 'Regex flavor',
            kind: 'select',
            options: FLAVORS,
            when: (i) => !!i.regex,
          },
          ...ALERT,
        ],
  );
</script>

<RuleRow
  layout="stack"
  {item}
  {fields}
  {title}
  {setting}
  {ondelete}
  {onchange}
  fallback="var(--accent)"
  problem={check?.error}
  note={check?.note}
/>
