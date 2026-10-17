// V1061-03 Live players on the broadcast's VOD now, by channel login: chat says it's live (Q23), the stream side keeps them after the stream ends (Q21).
import { SvelteMap } from 'svelte/reactivity';

export const rewound = new SvelteMap<string, { behind: number; ended: boolean }>();
