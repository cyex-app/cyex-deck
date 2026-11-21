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

const RELEASES = 'https://github.com/cyex-app/cyex-deck/releases/tag/';

/** V112-22 The tag a version is published under: the patch is zero-padded, 0.1.7 → v0.1.07. */
export function releaseTag(version: string): string {
  const m = /^v?(\d+)\.(\d+)\.(\d+)(.*)$/.exec(version.trim());
  if (!m) return `v${version.replace(/^v/, '')}`;
  return `v${m[1]}.${m[2]}.${m[3].padStart(2, '0')}${m[4]}`;
}

/** The release notes of a version on GitHub. */
export function releaseNotesUrl(version: string): string {
  return RELEASES + releaseTag(version);
}

/** A version as people read it: 0.1.7 → 0.1.07. */
export function displayVersion(version: string): string {
  return releaseTag(version).slice(1);
}
