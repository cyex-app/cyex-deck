// Stream manager (10, 1.35, X4): the broadcaster's own channel. Every action runs GQL
// first, Helix otherwise (D30), on the manager account's channel.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { AdSchedule } from './bindings/AdSchedule';
import type { BroadcastInfo } from './bindings/BroadcastInfo';
import type { CategoryHit } from './bindings/CategoryHit';
import type { ChannelEdit } from './bindings/ChannelEdit';
import type { ChatModes } from './bindings/ChatModes';
import type { SmActivity } from './bindings/SmActivity';
import type { SmBroadcaster } from './bindings/SmBroadcaster';
import type { SmMode } from './bindings/SmMode';
import type { SmOpen } from './bindings/SmOpen';
import type { SmPoll } from './bindings/SmPoll';
import type { SmPrediction } from './bindings/SmPrediction';
import type { SmPredictionEnd } from './bindings/SmPredictionEnd';
import type { SmStats } from './bindings/SmStats';
import type { SmTogether } from './bindings/SmTogether';
import type { Snoozed } from './bindings/Snoozed';
import type { VotesUpdate } from './bindings/VotesUpdate';

export const smIpc = {
  /** The account that runs the manager, or null when none can. */
  manager: () => invoke<SmBroadcaster | null>('sm_manager'),
  /** Ref-counted: starts the feed and stats on the first open. */
  open: () => invoke<SmOpen>('sm_open'),
  close: () => invoke<void>('sm_close'),
  info: () => invoke<BroadcastInfo>('sm_info'),
  setInfo: (edit: ChannelEdit) => invoke<void>('sm_set_info', { edit }),
  categories: (query: string) => invoke<CategoryHit[]>('sm_categories', { query }),
  marker: (description: string) => invoke<void>('sm_marker', { description }),
  markers: () => invoke<string[]>('sm_markers'),
  modes: () => invoke<ChatModes>('sm_modes'),
  setMode: (mode: SmMode) => invoke<ChatModes>('sm_set_mode', { mode }),
  commercial: (secs: number) => invoke<number>('sm_commercial', { secs }),
  adSchedule: () => invoke<AdSchedule>('sm_ad_schedule'),
  snooze: () => invoke<Snoozed>('sm_snooze'),
  raid: (login: string) => invoke<void>('sm_raid', { login }),
  unraid: () => invoke<void>('sm_unraid'),
  votes: () => invoke<VotesUpdate | null>('sm_votes'),
  createPoll: (poll: SmPoll) => invoke<void>('sm_create_poll', { poll }),
  endPoll: (id: string, archive: boolean) => invoke<void>('sm_end_poll', { id, archive }),
  createPrediction: (prediction: SmPrediction) => invoke<void>('sm_create_prediction', { prediction }),
  endPrediction: (id: string, end: SmPredictionEnd) => invoke<void>('sm_end_prediction', { id, end }),
  together: () => invoke<SmTogether>('sm_together'),
  togetherStart: () => invoke<void>('sm_together_start'),
  togetherEnd: (session: string) => invoke<void>('sm_together_end', { session }),
  togetherInvite: (session: string, login: string) => invoke<void>('sm_together_invite', { session, login }),
  togetherUninvite: (session: string, guest: string) => invoke<void>('sm_together_uninvite', { session, guest }),
  togetherSlot: (session: string, guest: string, slot: string, clear: boolean) =>
    invoke<void>('sm_together_slot', { session, guest, slot, clear }),
  obsFolder: () => invoke<string | null>('sm_obs_folder'),
  obsOpen: () => invoke<void>('sm_obs_open'),
  onStats: (f: (s: SmStats) => void): Promise<UnlistenFn> => listen<SmStats>('stream:stats', (e) => f(e.payload)),
  onActivity: (f: (a: SmActivity) => void): Promise<UnlistenFn> =>
    listen<SmActivity>('stream:activity', (e) => f(e.payload)),
};
