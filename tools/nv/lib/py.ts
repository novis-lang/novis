// What a command ported from a Python tool needs to print exactly what the tool printed: Python's line
// splitting, its `repr` of a string, the order `sorted()` gives a list of Windows paths, and argparse's
// reading of a command line with its errors.

/** Python's `str.splitlines`: every line boundary it knows, and no empty line after the last one. */
export function splitlines(text: string): string[] {
  const lines = text.split(/\r\n|[\n\r\v\f\x1c-\x1e\x85\p{Zl}\p{Zp}]/u);
  if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}

/** `repr` of a string the way Python writes it, since a message quotes a value that way. */
export function pyRepr(s: string): string {
  const q = s.includes("'") && !s.includes('"') ? '"' : "'";
  const body = s.replace(/\\/g, "\\\\").replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t");
  return q + (q === "'" ? body.replace(/'/g, "\\'") : body) + q;
}

/** Python's `int` over a command-line word, or null where it would raise. */
export function pyInt(word: string): number | null {
  const m = /^\s*([+-]?)(\d+(?:_\d+)*)\s*$/.exec(word);
  return m ? Number(m[1]! + m[2]!.replaceAll("_", "")) : null;
}

/** Python's `str.split()` with no argument, joined by one space: every run of whitespace is one. */
export function squash(s: string): string {
  return s.split(/\s+/).filter(Boolean).join(" ");
}

// Python's `textwrap`, for the options the ported tools use. A chunk is a run of whitespace or a word,
// and with `breakOnHyphens` a word also splits after a hyphen between two letters.
const WS = "[\\t\\n\\x0b\\x0c\\r ]";
const WORD_PUNCT = "[\\p{L}\\p{N}_!\"'&.,?]";
const LETTER = "[\\p{L}\\p{Nl}\\p{No}_]";
const WORDSEP = new RegExp(
  `(${WS}+` +
    `|(?<=${WORD_PUNCT})-{2,}(?=[\\p{L}\\p{N}_])` +
    `|[^\\t\\n\\x0b\\x0c\\r ]+?(?:-(?:(?<=${LETTER}{2}-)|(?<=${LETTER}-${LETTER}-))(?=${LETTER}-?${LETTER})` +
    `|(?=${WS}|$)|(?<=${WORD_PUNCT})(?=-{2,}[\\p{L}\\p{N}_])))`,
  "u",
);
const WORDSEP_SIMPLE = new RegExp(`(${WS}+)`);

export interface WrapOptions {
  breakLongWords?: boolean;
  breakOnHyphens?: boolean;
}

const pyLen = (s: string) => [...s].length;

/** Python's `textwrap.wrap(text, width, ...)`: the lines, each at most `width` long where it can be. */
export function wrap(text: string, width: number, opts: WrapOptions = {}): string[] {
  const breakLong = opts.breakLongWords ?? true;
  const onHyphens = opts.breakOnHyphens ?? true;
  let munged = "";
  for (const ch of text) {
    if (ch === "\t") munged += " ".repeat(8 - (pyLen(munged.slice(munged.lastIndexOf("\n") + 1)) % 8));
    else munged += ch;
  }
  munged = munged.replace(/[\t\n\x0b\x0c\r]/g, " ");
  const chunks = munged.split(onHyphens ? WORDSEP : WORDSEP_SIMPLE).filter((c) => c);
  chunks.reverse();
  const lines: string[] = [];
  while (chunks.length > 0) {
    const cur: string[] = [];
    let curLen = 0;
    if (chunks[chunks.length - 1]!.trim() === "" && lines.length > 0) chunks.pop();
    while (chunks.length > 0) {
      const l = pyLen(chunks[chunks.length - 1]!);
      if (curLen + l > width) break;
      cur.push(chunks.pop()!);
      curLen += l;
    }
    if (chunks.length > 0 && pyLen(chunks[chunks.length - 1]!) > width) {
      const spaceLeft = width < 1 ? 1 : width - curLen;
      if (breakLong) {
        const chunk = [...chunks[chunks.length - 1]!];
        let end = spaceLeft;
        if (onHyphens && chunk.length > spaceLeft) {
          const hyphen = chunk.slice(0, spaceLeft).lastIndexOf("-");
          if (hyphen > 0 && chunk.slice(0, hyphen).some((c) => c !== "-")) end = hyphen + 1;
        }
        cur.push(chunk.slice(0, end).join(""));
        chunks[chunks.length - 1] = chunk.slice(end).join("");
      } else if (cur.length === 0) {
        cur.push(chunks.pop()!);
      }
    }
    if (cur.length > 0 && cur[cur.length - 1]!.trim() === "") cur.pop();
    if (cur.length > 0) lines.push(cur.join(""));
  }
  return lines;
}

/** Python's `textwrap.fill`: `wrap`'s lines joined by newlines. */
export function fill(text: string, width: number, opts: WrapOptions = {}): string {
  return wrap(text, width, opts).join("\n");
}

/**
 * Python's `sorted()` over `pathlib` paths on Windows, for repo-relative forward-slash paths: a path
 * compares component by component, each one lower-cased.
 */
export function comparePaths(a: string, b: string): number {
  const pa = a.toLowerCase().split("/");
  const pb = b.toLowerCase().split("/");
  for (let i = 0; i < Math.min(pa.length, pb.length); i++) {
    if (pa[i] !== pb[i]) return pa[i]! < pb[i]! ? -1 : 1;
  }
  return pa.length - pb.length;
}

/**
 * Python's `format(x, ".Nf")`. Both round the double's exact value, so the two differ only on an exact
 * tie, which Python rounds to the even digit and `toFixed` away from zero. A tie is read off the exact
 * decimal expansion: a double that is halfway at `digits` ends in one `5` right after them.
 */
export function fixed(x: number, digits: number): string {
  const extra = Math.min(100, digits + 40) - digits;
  const exact = x.toFixed(digits + extra);
  if (!/^50*$/.test(exact.slice(-extra))) return x.toFixed(digits);
  const kept = exact.slice(0, -extra).replace(/\.$/, "");
  return Number(kept.at(-1)) % 2 === 0 ? kept : x.toFixed(digits);
}

/** Python's `format(x, "g")` for the plain figures a report prints: six significant digits, no trailing zeros. */
export const general = (x: number) => String(Number(x.toPrecision(6)));

/** A command line argparse refused. `message` is the text after `error: `. */
export class ArgError extends Error {}

export interface ArgSpec {
  /** Options that take no value. `--help` is always one of them. */
  flags: string[];
  /** Options that take exactly one value. */
  valued: string[];
  /** A short spelling and the long option it means, as `{ "-n": "--dry-run" }`. `-h` is always one. */
  short?: Record<string, string>;
  /** Options that take one value or none, argparse's `nargs="?"`, each with the value it has bare. */
  optional?: Record<string, string>;
}

export interface Parsed {
  /** Each flag given, by its long name. */
  flags: Set<string>;
  /** Each valued option given, by its long name, with the last value it was given. */
  values: Map<string, string>;
}

/** A word argparse would take as a value rather than as an option. */
function looksLikeValue(word: string): boolean {
  return !word.startsWith("-") || /^-\d+$|^-\d*\.\d+$/.test(word);
}

/**
 * `args` read the way argparse reads them for a parser with no positional arguments. An option is
 * matched exactly or by a unique prefix, every word is classified before any value is read, so an
 * ambiguous prefix is reported ahead of a missing value, and an unrecognized word only after both.
 * Throws `ArgError` with argparse's message.
 */
export function parseArgs(args: string[], spec: ArgSpec): Parsed {
  const flags = ["--help", ...spec.flags];
  const optional = spec.optional ?? {};
  const all = [...flags, ...spec.valued, ...Object.keys(optional)];
  const short: Record<string, string> = { "-h": "--help", ...spec.short };
  const resolve = (word: string): string | null => {
    if (all.includes(word)) return word;
    const hits = all.filter((o) => o.startsWith(word));
    if (hits.length > 1) throw new ArgError(`ambiguous option: ${word} could match ${hits.join(", ")}`);
    return hits[0] ?? null;
  };
  const classified = args.map((word) => {
    if (short[word]) return short[word]!;
    if (!word.startsWith("--") || word === "--") return null;
    const eq = word.indexOf("=");
    return resolve(eq >= 0 ? word.slice(0, eq) : word);
  });
  const out: Parsed = { flags: new Set(), values: new Map() };
  const unknown: string[] = [];
  for (let i = 0; i < args.length; i++) {
    const option = classified[i] ?? null;
    if (option === null) {
      unknown.push(args[i]!);
      continue;
    }
    if (flags.includes(option)) {
      out.flags.add(option);
      continue;
    }
    const word = args[i]!;
    const eq = word.indexOf("=");
    if (eq >= 0) out.values.set(option, word.slice(eq + 1));
    else if (i + 1 < args.length && looksLikeValue(args[i + 1]!)) out.values.set(option, args[++i]!);
    else if (option in optional) out.values.set(option, optional[option]!);
    else {
      const spellings = [...Object.keys(short).filter((s) => short[s] === option), option];
      throw new ArgError(`argument ${spellings.join("/")}: expected one argument`);
    }
  }
  if (unknown.length > 0) throw new ArgError(`unrecognized arguments: ${unknown.join(" ")}`);
  return out;
}
