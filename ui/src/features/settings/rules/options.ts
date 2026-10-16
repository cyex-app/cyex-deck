// V105-18 The options Highlights and Mentions rows share: icon, name and what each does
// (the tooltip). Spread into a Field with its key and kind.

import type { IconName } from '../../../components/icons';

interface Option {
  label: string;
  hint: string;
  icon?: IconName;
}

export const OPT = {
  wholeWord: { label: 'Whole word', hint: 'Only matches it on its own, not inside a longer word.', icon: 'wholeWord' },
  caseSensitive: { label: 'Match case', hint: 'Only matches the same capital and small letters.', icon: 'matchCase' },
  lookalikes: {
    label: 'Look-alikes',
    hint: 'Also matches letters that only look the same, like Cyrillic а or 𝐟𝐚𝐧𝐜𝐲 text.',
    icon: 'languages',
  },
  regex: { label: 'Regex', hint: 'The pattern is a regular expression.', icon: 'regex' },
  flavor: { label: 'Regex flavor', hint: 'Which regex rules the pattern follows.' },
  color: { label: 'Color', hint: 'Tints the message. × goes back to the default.' },
  sound: { label: 'Sound', hint: 'Plays the chime, or your sound file.', icon: 'volume' },
  soundFile: { label: 'Sound file', hint: 'Plays a file of your own instead of the chime.', icon: 'music' },
  flash: { label: 'Flash', hint: 'Flashes the taskbar button while the app isn’t in front.', icon: 'zap' },
  show: { label: 'Mentions tab', hint: 'Also copies the message into the Mentions tab.', icon: 'at' },
  notify: { label: 'Pop-up', hint: 'Shows a Windows notification while the app isn’t in front.', icon: 'bell' },
  route: { label: 'Feed tab', hint: 'Copies the message into the feed tab with this name.' },
} satisfies Record<string, Option>;
