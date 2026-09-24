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

/** A command line argparse refused. `message` is the text after `error: `. */
export class ArgError extends Error {}

export interface ArgSpec {
  /** Options that take no value. `--help` is always one of them. */
  flags: string[];
  /** Options that take exactly one value. */
  valued: string[];
  /** A short spelling and the long option it means, as `{ "-n": "--dry-run" }`. `-h` is always one. */
  short?: Record<string, string>;
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
  const all = [...flags, ...spec.valued];
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
    else throw new ArgError(`argument ${option}: expected one argument`);
  }
  if (unknown.length > 0) throw new ArgError(`unrecognized arguments: ${unknown.join(" ")}`);
  return out;
}
