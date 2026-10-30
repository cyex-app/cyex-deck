// V108-N17 "Open in logs": a user's logs.cyex.app page for a channel, in the browser. It
// runs `/logs <user>` in the chat, so the URL and the open-link check stay in Rust.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { usercardIpc } from '../../ipc/rules';
import { systemIpc } from '../../ipc/system';
import { errorText } from '../../lib/errors';
import { toast } from '../../lib/toast.svelte';
import { logsMessageUrl } from './logsLink';

/** "Jump to in logs": the message itself on logs.cyex.app, or no entry for a local line. */
export function jumpInLogs(channel: string, m: ChatMessage) {
  const url = logsMessageUrl(channel, m);
  if (!url) return [];
  return [
    {
      label: 'Jump to in logs',
      icon: 'scroll' as const,
      run: () => systemIpc.openLink(url).catch((e) => toast.error(errorText(e))),
    },
  ];
}

export function openInLogs(channel: string, login: string): void {
  usercardIpc
    .run(channel, `/logs ${login}`, { login, userId: null, display: null, msgId: null, msg: null, msgTime: null })
    .catch((e) => toast.error(errorText(e)));
}
