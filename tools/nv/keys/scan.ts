// How much of a `.rs` file a reader reads. Each tier is a digest over less of the file than the one
// before it, and a key takes the narrowest tier its check can be shown to read:
//
// - `raw`: the bytes. `fmt` reads these, and so does every test binary built from the file's own
//   package, because a policy test reads source as text and a comment is then an input.
// - `docs`: the tokens and the text of every doc comment. `clippy`, rustdoc and a doc-test read this.
// - `code`: the tokens alone, with their layout removed. `rustc` reads nothing else. A doc comment
//   leaves one placeholder per run of them, and a file with a bidirectional-text character in any
//   comment has every comment folded in, because `rustc` denies one.
// - `shipped`: the code without the body of any inline `#[cfg(test)] mod name { ... }`. A binary
//   built without `cfg(test)` cannot reach a token inside one.
// - `card`: the shipped code with the initialiser of every `const` whose type is a registry card
//   type blanked: a `*Doc` struct, or a slice of one. Only what prints a card reads it, so a check
//   that only runs programs keys here.
//
// Layout is removed conservatively: a run of whitespace stays, as one space, between two words and
// between two operator characters, so `& &x` and `&&x` never share a key. Literals are lifted out
// before that and hashed verbatim. No tier sees a line number, so a check that pins one in its
// expected output must key on `raw`.
//
// `includes` lists every `include_str!`/`include_bytes!` site with a literal path, and whether the
// site is inside an inline test module. A file embedded from outside a test module is data, and is
// raw in every key that holds the embedding file. One embedded from inside a test module is an
// input of that package's test binaries alone.

/** Folded into every tiered digest, so a change to the scanner is a change to every key. */
export const SCANNER = "nv-1";

export type Tier = "raw" | "docs" | "code" | "shipped" | "card";

/** The tiers from widest to narrowest. */
export const TIERS: readonly Tier[] = ["raw", "docs", "code", "shipped", "card"];

export interface IncludeSite {
  /** The literal path, as written. */
  path: string;
  /** Is the site inside an inline `#[cfg(test)] mod`? */
  inTest: boolean;
}

export interface Analysis {
  docs: string;
  code: string;
  shipped: string;
  card: string;
  includes: IncludeSite[];
}

const TOKEN =
  /(?<doc>\/\/(?:\/(?!\/)|!)[^\n]*)|(?<line>\/\/[^\n]*)|(?<block>\/\*)|(?<raw>(?<![A-Za-z0-9_])(?:b|c)?r(?<hashes>#*)"[\s\S]*?"\k<hashes>)|(?<str>(?:(?<![A-Za-z0-9_])(?:b|c))?"(?:\\[\s\S]|[^"\\])*")|(?<chr>(?:(?<![A-Za-z0-9_])b)?'(?:\\(?:u\{[^}\n]*\}|x[0-9a-fA-F]{2}|[^\n])|[^\\'\n])')/g;
const NEST = /\/\*|\*\//g;
const WORD = "[\\p{L}\\p{N}_]";
const NOT_WORD = "[^\\p{L}\\p{N}_]";
const GLUE = new RegExp(
  `(?<=${WORD}) (?=${NOT_WORD})|(?<=${NOT_WORD}) (?=${WORD})|(?<=[()\\[\\]{},;\\x01\\x02]) | (?=[()\\[\\]{},;\\x01\\x02])`,
  "gu",
);
const DOC_RUN = /\x01(?: ?\x01)+/g;
// An inline test module's header as `scan` leaves it: the attribute, any doc comment or bracket-free
// attribute after it, an optional visibility, and the opening brace. Nothing looser is matched.
const TEST_MOD = /#\[cfg\(test\)\](?:\x01|#\[[^\[\]]*\])*(?:pub(?:\([^()]*\))? ?)?mod \w+\{/g;
// A `const` of a card type or a slice of one, up to its `=`.
const CARD_CONST = /(?<![\p{L}\p{N}_])const [\p{L}\p{N}_]+:(?: ?&(?:'[A-Za-z_]+)?\[[A-Za-z_]*Doc\]|[A-Za-z_]*Doc)=/gu;
const INCLUDE = /include_(?:str|bytes)!\(\x02/g;
// What `text_direction_codepoint_in_comment` denies: the embeddings, overrides and isolates.
const BIDI = /[‪-‮⁦-⁩]/;

export interface Scanned {
  /** The tokens with layout removed; `\x01` stands for a run of doc comments, `\x02` for a literal. */
  code: string;
  literals: string[];
  docs: string[];
  plain: string[];
}

/** One `.rs` file's tokens, literals, doc comments and other comments. */
export function scan(text: string): Scanned {
  const code: string[] = [];
  const literals: string[] = [];
  const docs: string[] = [];
  const plain: string[] = [];
  let pos = 0;
  TOKEN.lastIndex = 0;
  for (;;) {
    TOKEN.lastIndex = pos;
    const m = TOKEN.exec(text);
    if (m === null) {
      code.push(text.slice(pos));
      break;
    }
    code.push(text.slice(pos, m.index));
    let end = m.index + m[0].length;
    const g = m.groups!;
    if (g.block !== undefined) {
      // Block comments nest, which no regular expression follows. An unclosed one runs to the end
      // of the file, as it does for rustc.
      let depth = 1;
      let at = end;
      while (depth > 0) {
        NEST.lastIndex = at;
        const n = NEST.exec(text);
        if (n === null) {
          at = text.length;
          break;
        }
        depth += n[0] === "/*" ? 1 : -1;
        at = n.index + 2;
      }
      end = at;
      const body = text.slice(m.index, end);
      const isDoc = body.startsWith("/*!") || (body.startsWith("/**") && !body.startsWith("/***") && !body.startsWith("/**/"));
      (isDoc ? docs : plain).push(body);
      code.push(isDoc ? " \x01 " : " ");
    } else if (g.doc !== undefined) {
      docs.push(m[0].trimEnd());
      code.push(" \x01 ");
    } else if (g.line !== undefined) {
      plain.push(m[0].trimEnd());
      code.push(" ");
    } else {
      literals.push(m[0]);
      code.push("\x02");
    }
    pos = end;
  }
  const flat = code.join("").replace(/\s+/g, " ").trim().replace(GLUE, "");
  return { code: flat.replace(DOC_RUN, "\x01"), literals, docs, plain };
}

/** The index just past the brace that closes the one opened before `from`, or -1. */
function closing(code: string, from: number): number {
  let depth = 1;
  for (let i = from; i < code.length; i++) {
    const c = code[i];
    if (c === "{") depth++;
    else if (c === "}" && --depth === 0) return i;
  }
  return -1;
}

/** The `[start, end)` body of every inline test module. One whose braces never close is left whole. */
export function testModules(code: string): [number, number][] {
  const out: [number, number][] = [];
  TEST_MOD.lastIndex = 0;
  for (let m = TEST_MOD.exec(code); m !== null; m = TEST_MOD.exec(code)) {
    const start = m.index + m[0].length;
    const close = closing(code, start);
    if (close < 0) break;
    out.push([start, close]);
    TEST_MOD.lastIndex = close;
  }
  return out;
}

/** The `[start, end)` initialiser of every card `const`, up to the `;` that ends it. */
export function cardConsts(code: string): [number, number][] {
  const out: [number, number][] = [];
  CARD_CONST.lastIndex = 0;
  for (let m = CARD_CONST.exec(code); m !== null; m = CARD_CONST.exec(code)) {
    const start = m.index + m[0].length;
    let depth = 0;
    let end = -1;
    for (let i = start; i < code.length; i++) {
      const c = code[i];
      if (c === "(" || c === "[" || c === "{") depth++;
      else if (c === ")" || c === "]" || c === "}") depth--;
      else if (c === ";" && depth === 0) {
        end = i;
        break;
      }
      if (depth < 0) break;
    }
    if (end < 0) continue;
    out.push([start, end]);
    CARD_CONST.lastIndex = end;
  }
  return out;
}

/** `code` and `literals` with each range removed, and the literals inside a range with it. */
export function cut(code: string, literals: string[], ranges: [number, number][]): [string, string[]] {
  const out: string[] = [];
  const kept: string[] = [];
  let pos = 0;
  let lit = 0;
  const count = (a: number, b: number) => {
    let n = 0;
    for (let i = code.indexOf("\x02", a); i >= 0 && i < b; i = code.indexOf("\x02", i + 1)) n++;
    return n;
  };
  for (const [start, end] of ranges) {
    out.push(code.slice(pos, start));
    const before = count(pos, start);
    kept.push(...literals.slice(lit, lit + before));
    lit += before + count(start, end);
    pos = end;
  }
  out.push(code.slice(pos));
  kept.push(...literals.slice(lit));
  return [out.join(""), kept];
}

/** A string literal's text without its prefix, hashes and quotes. */
export function literalBody(literal: string): string {
  const start = literal.indexOf('"');
  const end = literal.lastIndexOf('"');
  return start >= 0 && start < end ? literal.slice(start + 1, end) : "";
}

/** A hex blake2b digest of `chunks`, each followed by a separator so no two splits collide. */
export function digest(...chunks: (string | Uint8Array)[]): string {
  const h = new Bun.CryptoHasher("blake2b256");
  for (const c of chunks) {
    h.update(c);
    h.update("\0");
  }
  return h.digest("hex").slice(0, 32);
}

/** Every tier of one `.rs` file's text but `raw`, and its include sites. */
export function analyse(text: string): Analysis {
  const s = scan(text);
  const all = [...s.docs, ...s.plain];
  const comments = all.some((c) => BIDI.test(c)) ? all : [];
  const tokens = (code: string, literals: string[]) => digest(SCANNER, code, ...literals.map((l) => "\x03" + l), ...comments.map((c) => "\x04" + c));

  const tests = testModules(s.code);
  const [shippedCode, shippedLits] = cut(s.code, s.literals, tests);
  const [cardCode, cardLits] = cut(shippedCode, shippedLits, cardConsts(shippedCode));
  const code = tokens(s.code, s.literals);

  const includes: IncludeSite[] = [];
  const starts: number[] = [];
  for (let i = s.code.indexOf("\x02"); i >= 0; i = s.code.indexOf("\x02", i + 1)) starts.push(i);
  INCLUDE.lastIndex = 0;
  for (let m = INCLUDE.exec(s.code); m !== null; m = INCLUDE.exec(s.code)) {
    const at = m.index + m[0].length - 1;
    const index = starts.indexOf(at);
    const path = index >= 0 ? literalBody(s.literals[index] ?? "") : "";
    if (path) includes.push({ path, inTest: tests.some(([a, b]) => at >= a && at < b) });
  }
  return {
    docs: digest(code, ...s.docs),
    code,
    shipped: tokens(shippedCode, shippedLits),
    card: tokens(cardCode, cardLits),
    includes,
  };
}
