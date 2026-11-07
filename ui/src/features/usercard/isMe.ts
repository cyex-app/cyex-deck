/** V109-F70 Whether a card is one of your own accounts: by user id, across every signed-in
 *  account (not just the default or the one moderating here). */
export function isOwnCard(profileId: string | undefined, accountIds: readonly string[], modAccount?: string | null): boolean {
  if (!profileId) return false;
  return accountIds.includes(profileId) || profileId === modAccount;
}
