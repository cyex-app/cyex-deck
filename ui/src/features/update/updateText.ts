// V105-20 What the update UI says. Pure, so it's tested.

import type { CheckResult } from '../../ipc/bindings/CheckResult';
import type { UpdateProgress } from '../../ipc/bindings/UpdateProgress';

export function checkText(r: CheckResult): string {
  switch (r.kind) {
    case 'newest':
      return 'You have the newest version.';
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
