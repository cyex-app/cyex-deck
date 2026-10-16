<!-- V104-01 One mention: what it looks for and how (whole word, case, look-alikes, regex
     and its flavor), then what it does (color, sound or its own sound file, flash, the
     Mentions tab, pop-up). "Your name" has only the second part. V105-18 The same row
     as Highlights (RuleRow, split), options as icons. Errors and notes on the pattern
     show underneath. -->
<script lang="ts">
  import type { MentionAlert } from '../../../ipc/bindings/MentionAlert';
  import type { MentionCheck } from '../../../ipc/bindings/MentionCheck';
  import type { MentionRule } from '../../../ipc/bindings/MentionRule';
  import type { RegexFlavor } from '../../../ipc/bindings/RegexFlavor';
  import type { Field } from '../rules/fields';
  import { OPT } from '../rules/options';
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
    { value: 'javascript', label: 'JS' },
    { value: 'python', label: 'Python' },
  ];

  const ALERT: Field<Item>[] = [
    { key: 'color', kind: 'color', alert: true, ...OPT.color },
    { key: 'sound', kind: 'check', alert: true, ...OPT.sound },
    { key: 'soundFile', kind: 'sound', alert: true, ...OPT.soundFile },
    { key: 'flash', kind: 'check', alert: true, ...OPT.flash },
    { key: 'show', kind: 'check', alert: true, ...OPT.show },
    { key: 'notify', kind: 'check', alert: true, ...OPT.notify },
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
          { key: 'wholeWord', kind: 'check', ...OPT.wholeWord },
          { key: 'caseSensitive', kind: 'check', ...OPT.caseSensitive },
          { key: 'lookalikes', kind: 'check', ...OPT.lookalikes },
          { key: 'regex', kind: 'check', ...OPT.regex },
          { key: 'flavor', kind: 'select', options: FLAVORS, width: '64px', when: (i) => !!i.regex, ...OPT.flavor },
          ...ALERT,
        ],
  );
</script>

<RuleRow
  layout="split"
  mainMin="110px"
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
