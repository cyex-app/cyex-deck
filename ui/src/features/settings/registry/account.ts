// V103-13 Settings in the Account group: Accounts and Notifications.

import type { Entry } from './entry';

export const ACCOUNT: Entry[] = [
  // Accounts
  {
    id: 'accounts.list',
    page: 'accounts',
    label: 'Accounts',
    description: 'Everyone you signed in as. Remove one or sign in again.',
    keywords: ['sign out', 'log out', 'remove account', 'users'],
  },
  {
    id: 'accounts.defaultId',
    page: 'accounts',
    label: 'Default account',
    description: 'Used wherever no other account is picked.',
    keywords: ['make default', 'main account'],
  },
  {
    id: 'accounts:twitch',
    page: 'accounts',
    label: 'Sign in with Twitch',
    description: 'Chat, mod tools, your follows, payment methods and subscriptions.',
    keywords: ['log in', 'login', 'add account'],
  },
  {
    id: 'accounts:code',
    page: 'accounts',
    label: 'Sign in with a code',
    description: 'Approve a short code on Twitch. Chat and the GQL tools.',
    keywords: ['gql', 'token', 'log in', 'login', 'add account'],
  },
  {
    id: 'accounts:profile',
    page: 'accounts',
    label: 'Twitch profile',
    description: 'Display name, bio and name color, saved on Twitch.',
    keywords: ['display name', 'bio', 'about', 'name color', 'username color'],
  },

  // Notifications
  {
    id: 'notifications.live.enabled',
    page: 'notifications',
    label: 'Live alerts',
    description: 'When channels the default account follows go live.',
    keywords: ['goes live', 'stream started', 'online', 'notify'],
  },
  {
    id: 'notifications.live.toast',
    page: 'notifications',
    label: 'Live alert pop-up',
    description: 'A Windows notification. Click it to open the channel.',
    keywords: ['toast', 'windows notification'],
    anchor: 'notifications.live.enabled',
  },
  {
    id: 'notifications.live.sound',
    page: 'notifications',
    label: 'Live alert sound',
    anchor: 'notifications.live.enabled',
  },
  {
    id: 'notifications.live.skipAtStart',
    page: 'notifications',
    label: 'Quiet at start',
    description: 'No alerts for channels that were already live when the app opened.',
    anchor: 'notifications.live.enabled',
  },
  {
    id: 'notifications.live.channels',
    page: 'notifications',
    label: 'Live alerts per channel',
    description: 'Pick which followed channels alert you.',
    keywords: ['followed channels', 'bell'],
    anchor: 'notifications.live.enabled',
  },
  {
    id: 'notifications.whispers.toast',
    page: 'notifications',
    label: 'Whisper pop-up',
    description: 'While the app isn’t in front.',
    keywords: ['toast', 'dm', 'private message'],
  },
  { id: 'notifications.whispers.sound', page: 'notifications', label: 'Whisper sound', keywords: ['dm'] },
  {
    id: 'notifications.whispers.flash',
    page: 'notifications',
    label: 'Flash the taskbar',
    description: 'When a whisper comes in.',
    keywords: ['dm', 'blink'],
  },
  {
    id: 'notifications.whispers.inline',
    page: 'notifications',
    label: 'Show whispers in chat',
    description: 'Whispers also show as lines in the chats on screen.',
    keywords: ['dm'],
  },
  {
    id: 'notifications.bell.toast',
    page: 'notifications',
    label: 'Bell pop-ups',
    description: 'When Twitch marks a notification as a pop-up, show it as a Windows notification too.',
    keywords: ['toast', 'twitch notifications'],
  },
  {
    id: 'notifications:twitch',
    page: 'notifications',
    label: 'What Twitch sends',
    description: 'Twitch’s own email, phone and on-site notifications, saved on Twitch.',
    keywords: ['email', 'phone', 'push', 'mobile', 'smart delivery'],
  },
];
