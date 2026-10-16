// V105-20 What the update UI says. Pure, so it's tested.

import type { CheckResult } from '../../ipc/bindings/CheckResult';
import type { UpdateProgress } from '../../ipc/bindings/UpdateProgress';

/** `current` is the running version, named when it is the newest. */
export function checkText(r: CheckResult, current?: string): string {
  switch (r.kind) {
    case 'newest':
      return current ? `You have the newest version, ${current}.` : 'You have the newest version.';
    case 'available':
      return `${r.update.version} is out.`;
    case 'failed':
      return "Couldn’t check. Try again later.";
  }
}

export function progressText(p: UpdateProgress): string {
  switch (p.kind) {
    case 'downloading':
      return `Downloading… ${p.percent} %`;
    case 'verifying':
      return 'Checking the file…';
    case 'restarting':
      return 'Restarting…';
  }
}
