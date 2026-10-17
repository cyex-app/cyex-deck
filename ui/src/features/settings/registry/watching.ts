// V103-13 Settings in the Watching group: Player and Miner.

import type { Entry } from './entry';

/** A prediction setting, shown once betting is on. */
const bet = (field: string, label: string, description?: string, keys?: string[]): Entry => ({
  id: `miner.bets.${field}`,
  page: 'miner',
  label,
  description,
  keywords: ['predictions', 'bet'],
  keys,
  anchor: 'miner.bets.enabled',
});

export const WATCHING: Entry[] = [
  // Player
  {
    id: 'player.blockAds',
    page: 'player',
    label: 'Block ads',
    description: 'Cuts Twitch’s ads out of streams, or plays a clean copy while they run.',
    keywords: ['ads', 'adblock', 'commercials'],
  },
  {
    id: 'player.lowLatency',
    page: 'player',
    label: 'Low latency',
    description: 'A few seconds behind the stream instead of about ten.',
    keywords: ['delay'],
  },
  {
    id: 'player.liveRewind',
    page: 'player',
    label: 'Live VODs',
    description: 'Go back in live streams where Twitch allows it.',
    keywords: ['rewind', 'dvr', 'live vod', 'stream rewind', 'go back'],
  },
  {
    id: 'player.seekStep',
    page: 'player',
    label: 'Skip step',
    description: 'How far Left and Right, or J and L, jump.',
    keywords: ['arrow', 'seek', 'skip', 'keys', 'j', 'l'],
  },
  {
    id: 'player.quality',
    page: 'player',
    label: 'Quality',
    description: 'What players start in.',
    keywords: ['resolution', 'source', '1080p', '720p'],
  },
  {
    id: 'player:proxy',
    page: 'player',
    label: 'Ad-block proxy',
    description: 'When no clean copy is found, streams load through this HTTP proxy.',
    keywords: ['ads', 'adblock', 'vpn', 'country'],
  },

  // Miner
  {
    id: 'miner.order',
    page: 'miner',
    label: 'What goes first',
    description: 'Which channels the miner picks after the two you star.',
    keywords: ['priority', 'order', 'channel points'],
  },
  {
    id: 'miner.claimInTabs',
    page: 'miner',
    label: 'Bonus chests in open tabs',
    description: 'With each tab’s account, not only in mined channels.',
    keywords: ['claim', 'channel points'],
  },
  { id: 'miner.claimMoments', page: 'miner', label: 'Claim moments', keywords: ['moments'] },
  {
    id: 'miner.joinRaids',
    page: 'miner',
    label: 'Join raids',
    description: 'Goes along with a raid for its points.',
    keywords: ['raid'],
  },
  {
    id: 'miner.claimDrops',
    page: 'miner',
    label: 'Claim drops',
    description: 'Twitch wants the game account linked first.',
    keywords: ['drops', 'rewards'],
  },
  {
    id: 'miner.watchingEarns',
    page: 'miner',
    label: 'Watching here earns points',
    description: 'Streams you watch in the app count like on twitch.tv, drops included.',
    keywords: ['channel points', 'watch time'],
  },
  {
    id: 'miner.goals',
    page: 'miner',
    label: 'Community goals',
    description: 'Puts points toward a channel’s goals on its own.',
    keywords: ['channel points'],
  },
  {
    id: 'miner.chat',
    page: 'miner',
    label: 'Sit in their chats',
    description: 'Some channel bots reward people in chat.',
    keywords: ['presence', 'join chat'],
  },
  {
    id: 'miner.bets.enabled',
    page: 'miner',
    label: 'Bet on predictions',
    description: 'Spends points.',
    keywords: ['predictions', 'gamble'],
  },
  bet('strategy', 'Prediction pick', 'Which side the miner bets on.'),
  bet('outcome', 'Prediction side', '1 is the first (blue) side.'),
  bet('gap', 'Close bets', 'Gap between the two sides’ voters. Close bets go to the better payout.'),
  bet('percent', 'Bet size', 'Percent of your balance.'),
  bet('maxPoints', 'Most points per bet'),
  bet('keep', 'Points to keep', 'No bets while the balance is at or under this.'),
  bet('stealth', 'Stay small', 'Never bet more than the biggest bet on that side.'),
  bet('delay', 'When to bet', undefined, ['miner.bets.delayMode']),
  bet('filter', 'Only bet if', undefined, ['miner.bets.filter.by', 'miner.bets.filter.cmp', 'miner.bets.filter.value']),
];
