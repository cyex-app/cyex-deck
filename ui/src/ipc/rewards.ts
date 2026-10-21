// V107-R37 Channel points rewards on your own channel and the redemption queue (GQL).

import { invoke } from '@tauri-apps/api/core';

import type { SmQueue } from './bindings/SmQueue';
import type { SmRedemptionStatus } from './bindings/SmRedemptionStatus';
import type { SmReward } from './bindings/SmReward';
import type { SmBulk } from './bindings/SmBulk';
import type { SmRewardEdit } from './bindings/SmRewardEdit';
import type { SmRewardSaved } from './bindings/SmRewardSaved';
import type { SmRewards } from './bindings/SmRewards';
import type { SmRewardSwitch } from './bindings/SmRewardSwitch';

export const rewardsIpc = {
  list: () => invoke<SmRewards>('sm_rewards'),
  /** What's wrong with it, one line each; empty when it can be saved. */
  check: (edit: SmRewardEdit) => invoke<string[]>('sm_reward_check', { edit }),
  /** `warning`: made, but a second step failed. */
  create: (edit: SmRewardEdit) => invoke<SmRewardSaved>('sm_reward_create', { edit }),
  update: (id: string, edit: SmRewardEdit) => invoke<SmReward>('sm_reward_update', { id, edit }),
  switch: (id: string, change: SmRewardSwitch) => invoke<SmReward>('sm_reward_switch', { id, switch: change }),
  /** `waiting`: complete or refund its waiting redemptions. */
  remove: (id: string, waiting: SmRedemptionStatus | null) => invoke<void>('sm_reward_delete', { id, waiting }),
  /** Newest first; `reward` narrows it to one. */
  queue: (reward: string | null, cursor: string | null) => invoke<SmQueue>('sm_redemptions', { reward, cursor }),
  /** Which went through, and why the rest didn't. */
  setStatus: (ids: string[], status: SmRedemptionStatus) => invoke<SmBulk>('sm_redemptions_set', { ids, status }),
};
