// Reactive copy of the signed-in accounts and their joined-chat counts. Rust owns both;
// changes arrive as events.

import { accountsIpc } from '../ipc/accounts';
import type { AccountView } from '../ipc/bindings/AccountView';
import type { ConnectionCount } from '../ipc/bindings/ConnectionCount';
import type { AuthSnapshot } from '../ipc/bindings/AuthSnapshot';
import type { Feature } from '../ipc/bindings/Feature';

class AccountsStore {
  list = $state<AccountView[]>([]);
  connections = $state<ConnectionCount[]>([]);

  /** The default account, used for new tabs and the top bar. */
  get current(): AccountView | undefined {
    return this.list.find((a) => a.isDefault);
  }

  /** 1.54: whether the account (the default one if not given) can use a tool. */
  can(feature: Feature, userId?: string): boolean {
    const account = userId ? this.list.find((a) => a.userId === userId) : this.current;
    return account?.features.includes(feature) ?? false;
  }

  /** Whether any signed-in account can use a tool. */
  any(feature: Feature): boolean {
    return this.list.some((a) => a.features.includes(feature));
  }

  connectionsOf(userId: string): ConnectionCount | undefined {
    return this.connections.find((c) => c.userId === userId);
  }

  async load(): Promise<void> {
    const [snapshot, connections] = await Promise.all([accountsIpc.snapshot(), accountsIpc.connections()]);
    this.apply(snapshot);
    this.connections = connections;
    await accountsIpc.onChanged((s) => this.apply(s));
    await accountsIpc.onConnections((c) => (this.connections = c));
  }

  private apply(snapshot: AuthSnapshot): void {
    this.list = snapshot.accounts;
  }
}

export const accounts = new AccountsStore();
