// V1061-03 Several players of one channel on the VOD: how their entries fold into one.

export type Rewound = { behind: number; ended: boolean };

/** The nearest to live counts; over only when all are. */
export function fold(list: Rewound[]): Rewound | undefined {
  if (list.length === 0) return undefined;
  return { behind: Math.min(...list.map((x) => x.behind)), ended: list.every((x) => x.ended) };
}
