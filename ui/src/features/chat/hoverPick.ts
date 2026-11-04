// V108-F57d The keyboard pick of a chat line belongs to the list that made it: a pick in
// one chat must not hide the hover bars of another (tiles side by side), and a list that
// goes away lets go of its own pick only. Pure, so it is tested apart from the pointer code.

export type Owner = symbol;

export class Pick {
  owner: Owner | null = null;
  id: string | null = null;

  /** Picks `id` for `owner` (null id: none). Returns the ids whose state changed. */
  set(owner: Owner, id: string | null): string[] {
    if (id === null && this.owner !== owner) return [];
    if (this.owner === owner && this.id === id) return [];
    const was = this.id;
    this.owner = id === null ? null : owner;
    this.id = id;
    return [was, id].filter((x): x is string => x !== null);
  }

  /** The pick gives way to the pointer only for lines of its own list. */
  hot(id: string, owner: Owner | undefined, under: string | null): boolean {
    return this.id !== null && owner !== undefined && owner === this.owner ? this.id === id : under === id;
  }

  /** Whether a pick of this owner is held (the line under the pointer gives way). */
  holds(owner: Owner): boolean {
    return this.id !== null && this.owner === owner;
  }
}
