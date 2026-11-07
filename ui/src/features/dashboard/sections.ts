// V109-C04 The creator dashboard's sections, grouped like Twitch's dashboard menu. The last
// one opened is kept on this device and opens next time. Pure, so it's tested.

export const GROUPS = [
  {
    label: 'Content',
    sections: [
      { id: 'clips', label: 'Clips', icon: 'scissors', hint: 'Clips of your channel and the ones you made.' },
      { id: 'videos', label: 'Videos', icon: 'film', hint: 'Past broadcasts, highlights and uploads.' },
    ],
  },
  {
    label: 'Community',
    sections: [
      { id: 'followers', label: 'Followers', icon: 'heart', hint: 'Who follows you, and since when.' },
      { id: 'roles', label: 'Roles Manager', icon: 'users', hint: 'Mods, VIPs, editors and more in your channel.' },
    ],
  },
  {
    label: 'Viewer rewards',
    sections: [{ id: 'rewards', label: 'Channel points', icon: 'points', hint: 'Rewards and the redemption queue.' }],
  },
  {
    label: 'Settings',
    sections: [{ id: 'modset', label: 'Moderation', icon: 'shield', hint: 'AutoMod, verification, chat rules, terms.' }],
  },
] as const;

export type SectionId = (typeof GROUPS)[number]['sections'][number]['id'];
export type Section = (typeof GROUPS)[number]['sections'][number];

export const SECTIONS: readonly Section[] = GROUPS.flatMap((g): readonly Section[] => g.sections);

export const FIRST: SectionId = 'clips';

/** A kept section if it still exists, else the first. */
export function sectionOr(saved: string | null | undefined): SectionId {
  return SECTIONS.find((s) => s.id === saved)?.id ?? FIRST;
}

const KEY = 'cyex.dashboard.section';

/** The last section opened here; the first when none was kept or storage is off. */
export function lastSection(): SectionId {
  try {
    return sectionOr(localStorage.getItem(KEY));
  } catch {
    return FIRST;
  }
}

export function keepSection(id: SectionId): void {
  try {
    localStorage.setItem(KEY, id);
  } catch {
    // Not kept; it opens on the first next time.
  }
}
