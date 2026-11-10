// V110-F89 Test-only generator for generated-input tests: a seeded xorshift so a failure
// reproduces, and strings weighted towards what breaks text code (surrogate pairs, combining
// marks, bidi controls, zero-width characters, case mappings that change length).

export class Rng {
  private s: number;
  constructor(seed: number) {
    this.s = seed >>> 0 || 1;
  }
  next(): number {
    let x = this.s;
    x ^= x << 13;
    x >>>= 0;
    x ^= x >>> 17;
    x ^= x << 5;
    x >>>= 0;
    this.s = x;
    return x;
  }
  below(n: number): number {
    return n <= 0 ? 0 : this.next() % n;
  }
  chance(percent: number): boolean {
    return this.below(100) < percent;
  }
  pick<T>(items: readonly T[]): T {
    return items[this.below(items.length)];
  }
}

const TROUBLE = [
  0x130, 0x212a, 0x1e9e, 0xdf, 0x1c5, 0x149, 0xfb03, 0x3c2, 0x3a3, 0x300, 0x301, 0x34f, 0x200b, 0x200c, 0x200d,
  0x2060, 0xfeff, 0x200e, 0x200f, 0x202a, 0x202b, 0x202c, 0x202d, 0x202e, 0x2066, 0x2067, 0x2068, 0x2069, 0xfffd,
  0xe000, 0xd7ff, 0x1f600, 0x1f44b, 0x1f3fd, 0x1f468, 0x1f469, 0xfe0f, 0x20e3, 0x1f1e9, 0x1f1ea, 0x1d54f, 0x20bb7,
  0xe0001, 0x0, 0x1, 0x7, 0x8, 0x9, 0xb, 0xc, 0xd, 0xa, 0x1b, 0x7f, 0x80, 0x85, 0x9f, 0xa0, 0x2028, 0x2029, 0x3000,
  0x20, 0x20, 0x20, 0xa9, 0xae, 0x2122, 0x23, 0x2a, 0x30, 0x39, 0x5c, 0x3b, 0x3a, 0x40, 0x21, 0x24, 0x28, 0x29,
];

export function scalar(r: Rng): string {
  switch (r.below(10)) {
    case 0:
    case 1:
    case 2:
    case 3:
    case 4:
      return String.fromCodePoint(r.pick(TROUBLE));
    case 5:
    case 6:
      return String.fromCharCode(97 + r.below(26));
    case 7:
      return String.fromCharCode(48 + r.below(10));
    case 8:
      return String.fromCodePoint(0x80 + r.below(0x3000));
    default: {
      let cp = r.next() % 0x110000;
      if (cp >= 0xd800 && cp < 0xe000) cp = 0x41;
      return String.fromCodePoint(cp);
    }
  }
}

export function text(r: Rng, max: number): string {
  const n = r.below(max + 1);
  let out = '';
  for (let i = 0; i < n; i++) out += scalar(r);
  return out;
}

export function words(r: Rng, maxWords: number): string {
  const n = r.below(maxWords + 1);
  const out: string[] = [];
  for (let i = 0; i < n; i++) out.push(text(r, 8));
  return out.join(' ');
}

/** A string with lone surrogates in it (what a bad decode or a cut can leave). */
export function broken(r: Rng, max: number): string {
  const n = r.below(max + 1);
  let out = '';
  for (let i = 0; i < n; i++) out += r.chance(30) ? String.fromCharCode(0xd800 + r.below(0x800)) : scalar(r);
  return out;
}

export const NUMBERS: number[] = [
  NaN, Infinity, -Infinity, 0, -0, 1, -1, 0.5, -0.5, 0.1, 1.5, 2 ** 31, 2 ** 31 - 1, -(2 ** 31), 2 ** 32, 2 ** 53, -(2 ** 53),
  Number.MAX_SAFE_INTEGER, Number.MIN_SAFE_INTEGER, Number.MAX_VALUE, -Number.MAX_VALUE, Number.MIN_VALUE, 1e21, -1e21,
  1e20, 999999999999999999999, 1e-7, 1e-300, 1023.96, 1024, 1048575.99, 359.9, 360, 365, 59.999, 3599.5, 86399.5,
];

export function number(r: Rng): number {
  if (r.chance(50)) return r.pick(NUMBERS);
  const e = r.below(40) - 10;
  const v = (r.next() / 4294967296) * 10 ** e;
  return r.chance(50) ? -v : v;
}

/** Truncates, duplicates or deletes a span, swaps a delimiter or adds a random scalar. */
export function mutate(r: Rng, s: string): string {
  const cps = [...s];
  for (let k = 0; k < 1 + r.below(3); k++) {
    switch (r.below(5)) {
      case 0:
        cps.length = r.below(cps.length + 1);
        break;
      case 1: {
        const a = r.below(cps.length);
        const span = cps.slice(a, a + 1 + r.below(8));
        cps.splice(r.below(cps.length + 1), 0, ...span);
        break;
      }
      case 2:
        cps.splice(r.below(cps.length), 1 + r.below(6));
        break;
      case 3:
        cps.splice(r.below(cps.length + 1), 0, scalar(r));
        break;
      default:
        if (cps.length) cps[r.below(cps.length)] = scalar(r);
    }
  }
  return cps.join('');
}
