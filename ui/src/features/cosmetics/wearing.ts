// The status line after a pick: names what changed ("Wearing Sunset", "No paint").

export function wearingText(kind: 'paint' | 'badge' | null, id: string | null, name: string | undefined): string {
  if (!kind) return 'Wearing it';
  if (id === null) return kind === 'paint' ? 'No paint' : 'No badge';
  return name ? `Wearing ${name}` : 'Wearing it';
}
