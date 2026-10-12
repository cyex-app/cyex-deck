// Languages Twitch filters streams by (H4), checked 2026/10/10. Codes are its enum values.

export const LANGUAGES: [code: string, name: string][] = [
  ['EN', 'English'],
  ['DE', 'German'],
  ['ES', 'Spanish'],
  ['FR', 'French'],
  ['PT', 'Portuguese'],
  ['RU', 'Russian'],
  ['IT', 'Italian'],
  ['JA', 'Japanese'],
  ['KO', 'Korean'],
  ['ZH', 'Chinese'],
  ['ZH_HK', 'Cantonese'],
  ['AR', 'Arabic'],
  ['TR', 'Turkish'],
  ['PL', 'Polish'],
  ['NL', 'Dutch'],
  ['SV', 'Swedish'],
  ['NO', 'Norwegian'],
  ['DA', 'Danish'],
  ['FI', 'Finnish'],
  ['CS', 'Czech'],
  ['SK', 'Slovak'],
  ['HU', 'Hungarian'],
  ['RO', 'Romanian'],
  ['BG', 'Bulgarian'],
  ['EL', 'Greek'],
  ['UK', 'Ukrainian'],
  ['CA', 'Catalan'],
  ['HI', 'Hindi'],
  ['ID', 'Indonesian'],
  ['MS', 'Malay'],
  ['TH', 'Thai'],
  ['TL', 'Tagalog'],
  ['VI', 'Vietnamese'],
  ['ASL', 'Sign language'],
  ['OTHER', 'Other'],
];

/** "All languages", "German", "German, English", "German and 2 more". */
export function languageLabel(codes: string[]): string {
  const names = codes.map((c) => LANGUAGES.find(([code]) => code === c)?.[1] ?? c);
  if (!names.length) return 'All languages';
  if (names.length <= 2) return names.join(', ');
  return `${names[0]} and ${names.length - 1} more`;
}
