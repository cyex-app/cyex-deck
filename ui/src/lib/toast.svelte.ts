// Short notices. Info and success stack bottom right, errors bottom left (same as cyex logs).

export interface Toast {
  id: number;
  kind: 'info' | 'success' | 'error';
  text: string;
}

const LIFETIME_MS = { info: 3500, success: 3500, error: 7000 };

let nextId = 0;

class ToastStore {
  items = $state<Toast[]>([]);

  info(text: string): void {
    this.push('info', text);
  }

  /** V105-16 Green edge: something you asked for worked. */
  success(text: string): void {
    this.push('success', text);
  }

  error(text: string): void {
    this.push('error', text);
  }

  dismiss(id: number): void {
    this.items = this.items.filter((t) => t.id !== id);
  }

  private push(kind: Toast['kind'], text: string): void {
    const id = nextId++;
    this.items = [...this.items, { id, kind, text }];
    setTimeout(() => this.dismiss(id), LIFETIME_MS[kind]);
  }
}

export const toast = new ToastStore();
