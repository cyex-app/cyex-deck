// V104-02 The firehose connection in a few words, and a fresh mention.

import type { GlobalStatus } from '../../../ipc/bindings/GlobalStatus';
import type { MentionRule } from '../../../ipc/bindings/MentionRule';

export function statusText(s: GlobalStatus, hasMentions: boolean): string {
  switch (s.state) {
    case 'connecting':
      return 'Connecting…';
    case 'connected':
      return 'Connected';
    case 'offline':
      return `Offline. Trying again in ${s.retrySecs}s.`;
    default:
      return hasMentions ? 'Off' : 'Add a mention to start.';
  }
}

/** Whole word, any case, shown in the Mentions tab: what most people mean by a mention. */
export function newMention(): MentionRule {
  return {
    enabled: true,
    pattern: '',
    wholeWord: true,
    caseSensitive: false,
    lookalikes: false,
    regex: false,
    flavor: 'rust',
    show: true,
    flash: false,
    notify: false,
    sound: false,
    soundFile: '',
    color: '',
  };
}

/** "C:\Sounds\ping.wav" → "ping.wav". */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
