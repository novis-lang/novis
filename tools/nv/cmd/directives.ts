// `bun nv directives`: every `nvs.toml` leaf key, derived from `Config`'s field graph, and whether each
// one reaches a reader.
//
//     bun nv directives                     the roster: every leaf key and how it is read
//     bun nv directives --check             exit 1 on a key with no reader and no trailer, or a
//                                           trailer over a key that now has one
//     bun nv directives --json              the roster as JSON, for a generator to consume
//     bun nv directives --explain <key>     one key: its home, and every reader of it
//     bun nv directives --check-template    the shipped default file against the roster
//
// The typed block tree in `crates/nvs-config/src/tree.rs` is the one home for what an `nvs.toml` may
// say: `deny_unknown_fields` on every block makes the struct roster the accepted key set exactly. What
// the roster does *not* say is whether anything downstream ever looks at a key it accepts, and a key
// that parses and reaches nothing is worse than one that is refused: the operator writes it, the file
// is accepted, and the setting silently does nothing. The generated `nvs.toml` is rendered from this
// roster, so a key missing here is a key missing from the file an operator reads.
//
// **The walk fails closed.** A field whose type the walk does not recognise is an error, not a key
// quietly left out, so a new block added to `tree.rs` cannot join the accepted key set by saying
// nothing here.
//
// **A reader is counted three ways, and every one of them is load-bearing.** A key is read when the
// field is touched outside `tree.rs`, *or* when its dotted key appears as a string literal in code, *or*
// -- for a name that is one key in the whole roster -- when that bare name does. `Core\Storage` reaches
// its disk root as `config.get("storage.{disk}.root")` and never through the field; most blocks are read
// through the typed field with the dotted key written nowhere; and a limit is read by the short name
// `Core\Config::set` takes, which is how `config.get("max_script_depth")` reaches `[limits]
// max_script_depth`. Each of the three is the only reader of at least one key, so deleting one as
// redundant turns this gate into a generator of false gaps.
//
// **A field is the unit, not a path.** `[limits]` and `[app.limits]` are one struct, and the per-app
// merge folds the second onto the first for the same code to read, so a reader found under either path
// answers for both and one field is one entry in the gate however many paths reach it.
//
// **A registry row names a key; it does not read it.** `directive.rs` holds one row per directive and
// `capability.rs` each capability's name, so every key appears as a literal in one of them. The
// *literal* half skips those two files; their field accesses still count, which is how `Cap::grant`'s
// `caps.debug.as_ref()?.trace` reads the grant it maps.
//
// **What is searched is non-test crate source**: `crates/*/src` and `benches/*/src`, with `tree.rs`
// itself, comments and a trailing `#[cfg(test)]` module removed. A test that round-trips a key is not a
// reader. The question is whether the key is read, not whether the value is acted on: a grant that
// `Cap::grant` maps and no door asks for reads as read, and that deeper question belongs to the
// capability registry.
//
// A key with no reader declares itself on the last line of the field's own doc comment in `tree.rs`:
//
//     [unread: <why nothing reads it yet> owner: <who closes it>]
//
// `--check` fails in both directions: a key with no reader and no trailer landed silently, and a trailer
// over a key that now *has* a reader is worse, because it makes a generated file lie about its own
// surface. `--explain` asks the same question about one key and asserts rather than reports: it exits 1
// when the key reaches nothing, so a key whose only reader is a spelling a field search cannot see is
// one a check can name and hold.
//
// **`--check-template` holds the shipped default file to the roster.** Every leaf key appears in
// `default.toml`, beside the tree, exactly once and commented out (`#cpu_time = "5s"`), no key the tree
// does not parse appears at all, and every key whose field carries an `[unread:]` trailer sits under a
// `# NOT IMPLEMENTED` line naming its owner. The `#` with no space after it is what separates a setting
// from the prose above it. A `[block]` header is live, and two arrangements of live and commented-out
// headers are refused: a setting that is the same TOML key as a live header, and a setting under a
// commented-out header whose name is also a key of the live header above it. Every setting line ends
// with a trailer saying what leaving the key unset does -- `# default`, `# default: <what unset gives>`
// or `# example` -- two keys that are one field carry one trailer, and where the config crate ships the
// default itself (`CODE_DEFAULTS`) the value on a `# default` line is compared with the code.

import { existsSync, readFileSync } from "node:fs";
import { isAbsolute, join } from "node:path";
import { Glob } from "bun";
import { ROOT } from "../lib/paths.ts";
import { ArgError, comparePaths, parseArgs } from "../lib/py.ts";

export const summary = "every nvs.toml leaf key and its readers: nv directives [--check] [--json] [--explain KEY] [--check-template [PATH]]";

const TREE_REL = "crates/nvs-config/src/tree.rs";

/** The default file `--check-template` gates, beside the tree it is derived from. */
const TEMPLATE_REL = `${TREE_REL.slice(0, TREE_REL.lastIndexOf("/"))}/default.toml`;

/** The struct the walk starts from: what one configuration file deserializes into. */
const ROOT_STRUCT = "Config";

/** The two tables that name every key without reading any of them. A literal match here does not count. */
const REGISTRIES = new Set(["crates/nvs-config/src/directive.rs", "crates/nvs-config/src/capability.rs"]);

/** Types that end the walk. `Vec<String>` is a leaf too, unwrapped where the parameter is read. */
const SCALARS = new Set(["String", "bool", "u8", "u16", "u32", "u64", "i32", "i64", "f32", "f64", "usize"]);

/** The segment a map block's key carries in place of the name an operator picks: `[db.<name>]`. */
const NAME = "<name>";

const TRAILER = /\[unread:\s*(?<why>[^\]]+?)\s+owner:\s*(?<owner>[^\]]+?)\s*\]/;
const TRAILER_LIKE = /\[unread:/;

/** A block header in the default file, live or commented out: `[limits]`, `#[db.main]`, `[[server.mount]]`. */
const HEADER = /^#?\s*\[\[?([\w.\-]+)\]\]?$/;

/** A setting in the default file. Prose is `#` followed by a space or by nothing; a setting has neither. */
const SETTING = /^(?<out>#?)(?<key>[A-Za-z_][\w\-]*(?:\.[A-Za-z_][\w\-]*)*)\s*=/;

/** What an `[unread:]` key's prose block opens with, so an operator reads it before writing the key. */
const UNIMPLEMENTED = "NOT IMPLEMENTED";

/**
 * The trailer every setting line ends with: `default`, `default: <text>` or `example`. Either may end
 * `; restart required`, which `nvs-config`'s own tests hold to the registry's `Boot` rows, so it is no
 * part of what unset means and is left out of `text`.
 */
const NOTE = /^(?<kind>default|example)(?::\s*(?<text>\S.*?))?(?:\s*;\s*restart required)?\s*$/;

/** The defaults the config crate ships itself, and the setting each field of them is. */
const CODE_DEFAULTS: { rel: string; struct: string; fields: Record<string, string> }[] = [
  {
    rel: "crates/nvs-config/src/db.rs",
    struct: "PoolBounds",
    fields: { max: "db.<name>.pool.max", idle: "db.<name>.pool.idle", lifetime: "db.<name>.pool.lifetime", acquire: "db.<name>.pool.acquire" },
  },
  {
    rel: "crates/nvs-config/src/server.rs",
    struct: "Waits",
    fields: {
      header: "server.header_timeout",
      body_idle: "server.body_idle_timeout",
      write_idle: "server.write_idle_timeout",
      keepalive: "server.keepalive_timeout",
      drain: "server.drain_timeout",
    },
  },
  { rel: "crates/nvs-config/src/cache.rs", struct: "Revalidation", fields: { freq: "opcache.revalidate_freq" } },
];

/** A duration as the file writes one: `"30m"`, `"50ms"`, or a bare number of seconds. */
const DURATION = /^"?(?<n>\d+(?:\.\d+)?)\s*(?<unit>ms|s|m|h|d)?"?$/;
const UNITS: Record<string, number> = { ms: 0.001, s: 1.0, m: 60.0, h: 3600.0, d: 86400.0 };

/** A line comment, dropped where the question is whether a file's *code* names a block. */
const COMMENT = /\/\/.*/g;

/** What this workspace binds a block to when it is named neither after its key nor its type. */
const BINDINGS = new Set(["written"]);

/** A run of method calls between two field accesses: `.as_ref()?`, `.as_deref()`, `.clone()`. */
const CHAIN = String.raw`(?:\s*\.\s*\w+\s*\([^()]*\)\s*\??)*`;

/** What `strip` stops at: a comment opener, a raw string's opener or a plain string's. */
const STRIP_NEXT = /\/\/|\/\*|r#*"|"/g;
/** Inside a block comment, the next opener or closer; comments nest in Rust. */
const BLOCK_TOKEN = /\/\*|\*\//g;
/** Inside a plain string, the next escape or the closing quote. */
const STRING_STOP = /[\\"]/g;

/** A refusal the walk raises: printed alone on stderr, exit 1. */
class Fatal extends Error {}

function read(path: string): string {
  return readFileSync(isAbsolute(path) ? path : join(ROOT, path), "utf8").replace(/\r\n?/g, "\n");
}

function escape(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\\/\-#&~ ]/g, "\\$&");
}

// ------------------------------------------------------------------------------ tree.rs

/** One `pub name: Type` in a block struct, with what `serde` makes of it. */
interface Field {
  name: string;
  ty: string;
  key: string;
  doc: string[];
  line: number;
  flatten: boolean;
}

type Structs = Map<string, Field[]>;
type Enums = Map<string, string[]>;

/**
 * Every block struct's fields, every enum's payload types, and each block's own doc. A line reader: the
 * file is one flat run of derived structs with no generics, so the shape it reads is the shape the
 * module is required to keep, and anything it cannot read raises rather than being skipped.
 */
function parseTree(text: string): [Structs, Enums, Map<string, string[]>] {
  const structs: Structs = new Map();
  const enums: Enums = new Map();
  const blocks = new Map<string, string[]>();
  let doc: string[] = [];
  let attrs: string[] = [];
  let holder: string | null = null;
  let kind = "";

  const lines = text.split("\n");
  for (let index = 0; index < lines.length; index++) {
    const number = index + 1;
    const line = lines[index]!.trim();
    if (holder === null) {
      if (line.startsWith("///")) {
        doc.push(line.slice(3).trim());
        continue;
      }
      if (line.startsWith("#[")) {
        attrs.push(line);
        continue;
      }
      const opened = /^pub (struct|enum) (\w+) \{$/.exec(line);
      if (opened) {
        kind = opened[1]!;
        holder = opened[2]!;
        blocks.set(holder, doc);
        if (kind === "struct") structs.set(holder, []);
        else enums.set(holder, []);
        for (const attr of attrs) {
          if (attr.includes("rename_all")) {
            throw new Fatal(
              `nv directives: ${TREE_REL}:${number}: \`${holder}\` carries \`rename_all\`, which this walk does not spell. ` +
                "Teach it the renaming, or drop the attribute.",
            );
          }
        }
      }
      doc = [];
      attrs = [];
      continue;
    }

    if (line === "}") {
      holder = null;
      doc = [];
      attrs = [];
      continue;
    }
    if (line.startsWith("///")) {
      doc.push(line.slice(3).trim());
      continue;
    }
    if (line.startsWith("#[")) {
      attrs.push(line);
      continue;
    }
    if (kind === "struct") {
      const field = /^pub (\w+): (.+),$/.exec(line);
      if (field) {
        let renamed: string | null = null;
        for (const a of attrs) {
          const m = /rename = "([^"]+)"/.exec(a);
          if (m) {
            renamed = m[1]!;
            break;
          }
        }
        structs.get(holder)!.push({
          name: field[1]!,
          ty: field[2]!,
          key: renamed || field[1]!,
          doc,
          line: number,
          flatten: attrs.some((a) => a.includes("flatten")),
        });
        doc = [];
        attrs = [];
        continue;
      }
    } else {
      const variant = /^\w+\((.+)\),$/.exec(line);
      if (variant) {
        enums.get(holder)!.push(variant[1]!);
        doc = [];
        attrs = [];
        continue;
      }
    }
    if (line) {
      doc = [];
      attrs = [];
    }
  }

  if (!structs.has(ROOT_STRUCT)) throw new Fatal(`nv directives: ${TREE_REL} has no \`pub struct ${ROOT_STRUCT}\``);
  return [structs, enums, blocks];
}

/** `Option<T>` is how every optional block and leaf is written; the walk sees `T`. */
function unwrap(ty: string): string {
  const inner = /^Option<(.+)>$/.exec(ty);
  return inner ? inner[1]! : ty;
}

/** One leaf of the walk: the dotted key, and the field in `tree.rs` that is its home. */
interface Key {
  dotted: string;
  owner: string;
  field: Field;
  receivers: Set<string>;
  readers: string[];
  trailer: [string, string] | null;
}

function newKey(dotted: string, owner: string, field: Field, receivers: Set<string>): Key {
  return { dotted, owner, field, receivers, readers: [], trailer: null };
}

function anchor(key: Key): string {
  return `${TREE_REL}:${key.field.line}`;
}

/** `MailEndpoint` as the identifiers a reader is likely to bind it to: `mail_endpoint`, then `endpoint`. */
function snake(name: string): string[] {
  const lowered = (name.match(/[A-Z][a-z0-9]*/g) ?? []).map((p) => p.toLowerCase());
  return lowered.length > 0 ? [lowered.join("_"), lowered[lowered.length - 1]!] : [];
}

/**
 * Every leaf key under `name`, as the dotted path an operator writes. `prefix` carries the segments
 * already spelled, `chain` the struct names on the way here: a repeat is a cycle, an unbounded key
 * space, so it raises.
 */
function walk(structs: Structs, enums: Enums, name: string, prefix: string[], chain: string[], element = false): Key[] {
  if (chain.includes(name)) {
    throw new Fatal(`nv directives: \`${name}\` reaches itself through ${chain.join(" -> ")}, so the key space has no bottom.`);
  }
  // The identifiers a reader plausibly binds *this block* to: the segment it is written under, and its
  // type's name. A map block's segment is the name the operator chose, so for one of those the type is
  // all there is.
  const written = prefix.filter((s) => s !== NAME);
  const receivers = new Set<string>(written.length > 0 ? [written[written.length - 1]!] : []);
  for (const s of snake(name)) receivers.add(s);
  for (const b of BINDINGS) receivers.add(b);
  if (element) {
    // One of many, picked out of an array or a map, so the code that holds it cannot name it after the
    // key: `[[server.mount]]` is read as `block.scan` and `[[schedule]]` as `entry.cron`.
    receivers.add("block");
    receivers.add("entry");
  }
  const found: Key[] = [];
  for (const field of structs.get(name)!) {
    const segments = field.flatten ? prefix : [...prefix, field.key];
    found.push(...expand(structs, enums, field, unwrap(field.ty), segments, name, receivers, chain));
  }
  return found;
}

/** One field's contribution: a leaf key, a block to walk into, or both. */
function expand(
  structs: Structs,
  enums: Enums,
  field: Field,
  ty: string,
  segments: string[],
  owner: string,
  receivers: Set<string>,
  chain: string[],
): Key[] {
  const dotted = segments.join(".");
  const listed = /^Vec<(.+)>$/.exec(ty);
  const mapped = /^BTreeMap<String, (.+)>$/.exec(ty);

  if (SCALARS.has(ty) || (listed && SCALARS.has(listed[1]!))) return [newKey(dotted, owner, field, receivers)];
  // `[[include]]`, `[[app]]`, `[[server.mount]]`: an array of tables, spelled under the same segment.
  if (listed && structs.has(listed[1]!)) return walk(structs, enums, listed[1]!, segments, [...chain, owner], true);
  if (mapped && structs.has(mapped[1]!)) return walk(structs, enums, mapped[1]!, [...segments, NAME], [...chain, owner], true);
  if (structs.has(ty)) return walk(structs, enums, ty, segments, [...chain, owner]);
  if (enums.has(ty)) {
    // `Setting` is scalar in every variant and is the leaf; `Pool` is a scalar *and* a table under the
    // same key, so it is both a leaf and a walk.
    const found: Key[] = [];
    for (const payload of enums.get(ty)!) {
      const inner = /^Vec<(.+)>$/.exec(payload);
      if (SCALARS.has(payload) || (inner && SCALARS.has(inner[1]!))) {
        if (!found.some((k) => k.dotted === dotted)) found.push(newKey(dotted, owner, field, receivers));
      } else if (structs.has(payload)) {
        found.push(...walk(structs, enums, payload, segments, [...chain, owner]));
      } else {
        throw new Fatal(`nv directives: ${TREE_REL}:${field.line}: \`${ty}::${payload}\` is a variant this walk cannot classify.`);
      }
    }
    return found;
  }
  throw new Fatal(
    `nv directives: ${TREE_REL}:${field.line}: \`${owner}::${field.name}\` is a \`${ty}\`, which is neither a scalar, ` +
      "a block in this module nor a map of one. Add the type to the walk rather than leaving its keys uncounted.",
  );
}

// ------------------------------------------------------------------------------ readers

/**
 * Rust source with its comments removed, and every string literal in it. Comments are not readers,
 * and the scanner tracks strings so that a `//` inside one is not read as a comment opener, and raw
 * strings so that an escaped quote inside one is not read as its end.
 */
function strip(text: string): [string, string[]] {
  const out: string[] = [];
  const literals: string[] = [];
  let i = 0;
  const n = text.length;
  while (i < n) {
    STRIP_NEXT.lastIndex = i;
    const m = STRIP_NEXT.exec(text);
    if (m === null) {
      out.push(text.slice(i));
      break;
    }
    out.push(text.slice(i, m.index));
    i = m.index;
    const token = m[0];
    if (token === "//") {
      const end = text.indexOf("\n", i);
      i = end < 0 ? n : end;
    } else if (token === "/*") {
      let depth = 1;
      i += 2;
      while (depth) {
        BLOCK_TOKEN.lastIndex = i;
        const b = BLOCK_TOKEN.exec(text);
        if (b === null) {
          i = n;
          break;
        }
        depth += b[0] === "/*" ? 1 : -1;
        i = b.index + 2;
      }
    } else if (token !== '"') {
      const fence = '"' + token.slice(1, -1);
      let end = text.indexOf(fence, i + token.length);
      end = end < 0 ? n : end + fence.length;
      literals.push(text.slice(i + token.length, Math.max(i + token.length, end - fence.length)));
      out.push(" ".repeat(end - i));
      i = end;
    } else {
      let j = i + 1;
      while (j < n) {
        STRING_STOP.lastIndex = j;
        const s = STRING_STOP.exec(text);
        if (s === null) {
          j = n;
          break;
        }
        j = s.index;
        if (text[j] === "\\") {
          j += 2;
          continue;
        }
        break;
      }
      literals.push(text.slice(i + 1, j));
      out.push(" ".repeat(j + 1 - i));
      i = j + 1;
    }
  }
  return [out.join(""), literals];
}

interface Source {
  rel: string;
  code: string;
  /** The file's literals joined by newlines, which no key's literal pattern can match across. */
  joined: string;
  texts: Set<string>;
  /** `code` with its line comments removed. */
  proseFree: string;
  /** Which receivers the file names in `proseFree`, filled as keys ask. */
  names: Map<string, boolean>;
}

/**
 * Every non-test crate source file. A `#[cfg(test)]` module at column 0 and everything after it is
 * dropped: the module is last by convention, and a test that writes a key into a fixture and reads it
 * back is not what makes the key live.
 */
function corpus(): Source[] {
  const files: Source[] = [];
  for (const pattern of ["crates/*/src/**/*.rs", "benches/*/src/**/*.rs"]) {
    const paths = [...new Glob(pattern).scanSync({ cwd: ROOT, onlyFiles: true, dot: true })]
      .map((p) => p.replaceAll("\\", "/"))
      .sort(comparePaths);
    for (const rel of paths) {
      if (rel === TREE_REL) continue;
      const text = read(rel);
      const cut = /^#\[cfg\(test\)\]/m.exec(text);
      const [code, literals] = strip(cut ? text.slice(0, cut.index) : text);
      files.push({
        rel,
        code,
        joined: literals.join("\n"),
        texts: new Set(literals),
        proseFree: code.replace(COMMENT, ""),
        names: new Map(),
      });
    }
  }
  return files;
}

/**
 * The key as a reader spells it in a string. `<name>` is whatever the operator called the block, so it
 * matches any run without a dot, a quote or a space: `"storage.{disk}.root"` reads `storage.<name>.root`.
 */
function literalSource(dotted: string): string {
  const body = dotted
    .split(".")
    .map((s) => (s === NAME ? String.raw`[^".\s]+` : escape(s)))
    .join(String.raw`\.`);
  return String.raw`(?<![\w.])` + body + String.raw`(?![\w.])`;
}

/** A pattern both searched for and tried at one position. */
interface Pattern {
  search: RegExp;
  sticky: RegExp;
}

/**
 * The field read off a block the reader already holds. The receiver has to name the block, because the
 * field names alone are `path`, `host`, `dir` and `listen`, which every crate uses for something else;
 * a trailing `(` is excluded for the same reason: `cache.dir()` is a method on another type.
 */
function accessPattern(field: string, receivers: Set<string>): Pattern {
  const recv = [...receivers]
    .map(escape)
    .sort((a, b) => b.length - a.length)
    .join("|");
  const source = String.raw`\b(?:${recv})\b${CHAIN}\s*\.\s*${escape(field)}\b(?!\s*\()`;
  return { search: new RegExp(source), sticky: new RegExp(source, "y") };
}

function minus(set: Set<string>, drop: Set<string>): Set<string> {
  return new Set([...set].filter((s) => !drop.has(s)));
}

/**
 * Whether one file reads `key` off a block it holds, generic bindings included. `written` is what any
 * deserialized block is bound to, so it is trusted where the field name identifies one key on its own,
 * and otherwise only where the file names the block somewhere in its **code**: a block named in a `//!`
 * header and nowhere else is prose about another crate's business. Both patterns open on a receiver
 * spelled verbatim, so a pattern is tried only at the positions where one occurs.
 */
function readsField(file: Source, key: Key, access: Pattern, narrow: Pattern, bare: string | null): boolean {
  const { code, proseFree, names } = file;
  const spelled = (pattern: Pattern, receivers: Set<string>): boolean => {
    if (receivers.size === 0) return pattern.search.test(code);
    for (const r of receivers) {
      let at = code.indexOf(r);
      while (at >= 0) {
        pattern.sticky.lastIndex = at;
        if (pattern.sticky.test(code)) return true;
        at = code.indexOf(r, at + 1);
      }
    }
    return false;
  };

  if (bare) return spelled(access, key.receivers);
  let named = false;
  for (const r of minus(key.receivers, BINDINGS)) {
    if (!names.has(r)) names.set(r, proseFree.includes(r) && new RegExp(String.raw`\b${escape(r)}\b`).test(proseFree));
    if (names.get(r)) {
      named = true;
      break;
    }
  }
  if (named) return spelled(access, key.receivers);
  return spelled(narrow, minus(key.receivers, BINDINGS));
}

/**
 * The field names that identify exactly one key in the roster. `Core\Config::get` takes a limit by its
 * bare name, and the short spelling only resolves at all for a name that is one key.
 */
function bareNames(keys: Key[]): Set<string> {
  const fields = new Set(keys.map((k) => k.field));
  const counted = new Map<string, number>();
  for (const f of fields) counted.set(f.name, (counted.get(f.name) ?? 0) + 1);
  return new Set([...counted].filter(([, count]) => count === 1).map(([name]) => name));
}

/**
 * Fill in each key's readers, by all three spellings, over the whole corpus. A file that does not
 * contain the field's name cannot match its access pattern, and a file's literals that do not contain
 * every spelled segment cannot match its literal pattern, so the substring tests skip the regexes over
 * most of the corpus.
 */
function findReaders(keys: Key[]): void {
  const files = corpus();
  const unique = bareNames(keys);
  for (const key of keys) {
    const literal = new RegExp(literalSource(key.dotted));
    const spelled = key.dotted.split(".").filter((s) => s !== NAME);
    const access = accessPattern(key.field.name, key.receivers);
    const narrow = accessPattern(key.field.name, minus(key.receivers, BINDINGS));
    const name = key.field.name;
    const bare = unique.has(name) ? name : null;
    for (const file of files) {
      if (!REGISTRIES.has(file.rel) && spelled.every((s) => file.joined.includes(s)) && literal.test(file.joined)) {
        key.readers.push(`${file.rel} (key)`);
      } else if (file.code.includes(name) && readsField(file, key, access, narrow, bare)) {
        key.readers.push(`${file.rel} (field)`);
      } else if (bare && !REGISTRIES.has(file.rel) && file.texts.has(bare)) {
        key.readers.push(`${file.rel} (name)`);
      }
    }
  }
  // A block reached at two paths -- `[limits]` and `[app.limits]` -- is one field and one reader: the
  // per-app merge folds the block onto the global one and the same code reads both.
  for (const key of keys) {
    if (key.readers.length === 0) key.readers = keys.find((k) => k.field === key.field && k.readers.length > 0)?.readers ?? [];
  }
}

// ------------------------------------------------------------------------------ the gate

/** The `[unread: … owner: …]` each key's own doc comment declares, if any. */
function readTrailers(keys: Key[], problems: string[]): void {
  for (const key of keys) {
    const body = key.field.doc.filter((line) => line);
    const found = body.length > 0 ? TRAILER.exec(body[body.length - 1]!) : null;
    if (found) {
      key.trailer = [found.groups!.why!.trim(), found.groups!.owner!.trim()];
      continue;
    }
    if (body.some((line) => TRAILER_LIKE.test(line))) {
      problems.push(
        `${anchor(key)}: \`${key.dotted}\` carries something shaped like an \`[unread: … owner: …]\` trailer that this does not read. ` +
          "It is `[unread: <why> owner: <who>]`, on the doc comment's last line.",
      );
    }
  }
}

/** The gate, reported once per field rather than once per path. */
function check(keys: Key[]): string[] {
  const problems: string[] = [];
  readTrailers(keys, problems);
  const first = new Map<Field, Key>();
  for (const k of keys) if (!first.has(k.field)) first.set(k.field, k);
  for (const key of [...first.values()].sort((a, b) => a.field.line - b.field.line)) {
    const spelled = keys
      .filter((k) => k.field === key.field)
      .map((k) => k.dotted)
      .join(" / ");
    if (key.readers.length > 0 && key.trailer) {
      problems.push(
        `${anchor(key)}: \`${spelled}\` is declared unread, but ${key.readers[0]} reads it. ` +
          "Delete the trailer -- a generated file that marks a live key unimplemented is worse than one that omits it.",
      );
    } else if (key.readers.length === 0 && !key.trailer) {
      problems.push(
        `${anchor(key)}: \`${spelled}\` reaches no reader and declares nothing. Wire it to the code that should act on it, ` +
          "or give the field's doc comment an `[unread: <why> owner: <who>]` trailer.",
      );
    }
  }
  return problems;
}

/**
 * The roster key a spelling names. A map block is `storage.<name>.root` in the roster and
 * `storage.local.root` in a file an operator wrote, and both resolve here.
 */
function findKey(keys: Key[], wanted: string): Key | null {
  const exact = keys.find((k) => k.dotted === wanted);
  if (exact) return exact;
  return keys.find((k) => k.dotted.includes(NAME) && new RegExp(`^(?:${literalSource(k.dotted)})$`).test(wanted)) ?? null;
}

/** One key, whole: where it is written, what reads it, and by which spelling. Exit 1 when nothing reads it. */
function explain(keys: Key[], wanted: string): number {
  readTrailers(keys, []);
  const key = findKey(keys, wanted);
  if (key === null) {
    const head = wanted.split(".")[0]!;
    const near = keys.filter((k) => k.dotted.split(".")[0] === head).map((k) => k.dotted);
    console.error(`nv directives: \`${wanted}\` is not a key ${TREE_REL} parses.`);
    console.error(near.length > 0 ? `Under \`${head}\`: ${near.join(", ")}` : "`bun nv directives` lists the roster.");
    return 1;
  }
  const spelled = keys.filter((k) => k.field === key.field && k.dotted !== key.dotted).map((k) => k.dotted);
  const out = [key.dotted, `  home    ${anchor(key)}  (${key.owner}::${key.field.name}: ${key.field.ty})`];
  if (spelled.length > 0) out.push(`  also    ${spelled.join(" / ")}`);
  for (const reader of key.readers) out.push(`  read    ${reader}`);
  if (key.trailer) out.push(`  unread  ${key.trailer[0]}`, `  owner   ${key.trailer[1]}`);
  console.log(out.join("\n"));
  if (key.readers.length === 0) {
    console.error(`\nnv directives: \`${key.dotted}\` reaches no reader.`);
    return 1;
  }
  return 0;
}

// ------------------------------------------------------------------------------ the template

/** One setting in the default file: the key it spells, and the prose above it. */
interface Entry {
  dotted: string;
  line: number;
  prose: string[];
  commented: boolean;
  /**
   * The key this line becomes when it is uncommented and its own header is not: TOML puts it under the
   * nearest live header above. `null` under a live header.
   */
  lands: string | null;
  /** The value written beside the key. */
  value: string;
  /** The text past the first `#` that is not inside a string, or `null` when the line carries none. */
  note: string | null;
}

/**
 * Every setting the default file spells, under the block header it sits below. A run of settings
 * shares the prose block above it; a blank line or a header ends the block.
 */
function parseTemplate(text: string): Entry[] {
  const entries: Entry[] = [];
  let prefix = "";
  let live = "";
  let prose: string[] = [];
  const lines = text.split("\n");
  for (let index = 0; index < lines.length; index++) {
    const line = lines[index]!.trim();
    if (!line) {
      prose = [];
      continue;
    }
    const header = HEADER.exec(line);
    if (header) {
      prefix = header[1]!;
      prose = [];
      if (!line.startsWith("#")) live = prefix;
      continue;
    }
    const setting = SETTING.exec(line);
    if (setting) {
      const key = setting.groups!.key!;
      const lands = live === prefix ? null : live ? `${live}.${key}` : key;
      const [value, note] = splitValue(line.slice(setting[0].length));
      entries.push({
        dotted: prefix ? `${prefix}.${key}` : key,
        line: index + 1,
        prose,
        commented: setting.groups!.out !== "",
        lands,
        value,
        note,
      });
      continue;
    }
    if (line.startsWith("#")) {
      prose.push(line.replace(/^#+/, "").trim());
      continue;
    }
    prose = [];
  }
  return entries;
}

/** What follows a setting's `=`: the value, and the trailer past the first `#` that is not inside a string. */
function splitValue(rest: string): [string, string | null] {
  let quote = "";
  for (let index = 0; index < rest.length; index++) {
    const char = rest[index]!;
    if (quote) {
      if (char === quote) quote = "";
    } else if (char === '"' || char === "'") {
      quote = char;
    } else if (char === "#") {
      return [rest.slice(0, index).trim(), rest.slice(index + 1).trim()];
    }
  }
  return [rest.trim(), null];
}

/** A duration as the file or the code spells it, in seconds, or `null` for anything else. */
function seconds(text: string): number | null {
  text = text.trim();
  const code = /^Duration::from_(?<unit>secs|millis)\((?<expr>[\d\s*]+)\)$/.exec(text);
  if (code) {
    let product = 1;
    for (const factor of code.groups!.expr!.split("*")) product *= Number.parseInt(factor.trim(), 10);
    return code.groups!.unit === "millis" ? product / 1000 : product;
  }
  const plain = DURATION.exec(text);
  if (plain) return Number.parseFloat(plain.groups!.n!) * (plain.groups!.unit ? UNITS[plain.groups!.unit]! : 1.0);
  return null;
}

/** Whether a setting line shows the value the code spells: the same text, or the same number of seconds. */
function sameValue(written: string, expr: string): boolean {
  if (written.trim() === expr.trim()) return true;
  const left = seconds(written);
  const right = seconds(expr);
  return left !== null && right !== null && left === right;
}

/**
 * Every default the config crate ships itself, by the setting it is: the value as the code spells it,
 * and the `file:line` it is spelled at. Read out of the struct literal under `impl Default for <Type>`
 * or `pub const DEFAULT: <Type>`, one `field: expr,` per line; a default written any other way is an
 * error here rather than a comparison quietly skipped.
 */
function codeDefaults(): Map<string, [string, string]> {
  const found = new Map<string, [string, string]>();
  for (const { rel, struct, fields } of CODE_DEFAULTS) {
    let opened = false;
    let literal = false;
    const lines = read(rel).split("\n");
    for (let index = 0; index < lines.length; index++) {
      const line = lines[index]!.trim();
      if (!opened) {
        if (line === `impl Default for ${struct} {`) opened = true;
        else if (line === `pub const DEFAULT: ${struct} = ${struct} {`) opened = literal = true;
        continue;
      }
      if (!literal) {
        literal = line === "Self {" || line === `${struct} {`;
        continue;
      }
      const pair = /^(?<field>\w+): (?<expr>.+),$/.exec(line);
      if (pair && Object.hasOwn(fields, pair.groups!.field!)) {
        found.set(fields[pair.groups!.field!]!, [pair.groups!.expr!, `${rel}:${index + 1}`]);
      } else if (line.startsWith("}")) {
        break;
      }
    }
    const missing = Object.entries(fields)
      .filter(([, dotted]) => !found.has(dotted))
      .map(([field]) => field);
    if (missing.length > 0) {
      throw new Fatal(
        `nv directives: ${rel}: \`${struct}\`'s default no longer spells ${missing.join(", ")} as \`field: expr,\` under its ` +
          "struct literal, so `--check-template` cannot compare the default file with it. Teach `codeDefaults` the new " +
          "shape, or write the literal that way.",
      );
    }
  }
  return found;
}

/**
 * The trailer every setting line owes, in the three ways it goes wrong: missing or malformed, two
 * spellings of one field disagreeing, and a `# default` value the config crate does not ship.
 */
function checkNotes(keys: Key[], entries: Entry[], rel: string): string[] {
  const problems: string[] = [];
  const first = new Map<Field, [Entry, string]>();
  const shipped = codeDefaults();
  for (const entry of entries) {
    const key = findKey(keys, entry.dotted);
    if (key === null) continue;
    const note = NOTE.exec(entry.note ?? "");
    if (!note) {
      problems.push(
        `${rel}:${entry.line}: \`${entry.dotted}\` does not say what leaving it unset does. End the line with \`# default\` ` +
          "when the value shown is what an unset key gives, `# default: <what unset gives>` when that is something no " +
          "value spells, or `# example` when there is nothing to call a default.",
      );
      continue;
    }
    const kind = note.groups!.kind!;
    const text = note.groups!.text;
    const spelled = text ? `${kind}: ${text}` : kind;
    if (kind === "example" && text) {
      problems.push(
        `${rel}:${entry.line}: \`${entry.dotted}\` says \`# ${spelled}\`. \`example\` takes no text -- it says nothing is ` +
          "granted, mounted, opened or scheduled until the key is written -- and a key with a default says `# default: " +
          "<what unset gives>` instead.",
      );
      continue;
    }
    if (!first.has(key.field)) first.set(key.field, [entry, spelled]);
    const earlier = first.get(key.field)!;
    if (earlier[1] !== spelled) {
      problems.push(
        `${rel}:${entry.line}: \`${entry.dotted}\` says \`# ${spelled}\`, and \`${earlier[0].dotted}\` at line ` +
          `${earlier[0].line} says \`# ${earlier[1]}\`. They are one field (${anchor(key)}) read by one piece of code, ` +
          "so unset means one thing.",
      );
    }
    const code = shipped.get(key.dotted);
    if (code === undefined) continue;
    const [expr, at] = code;
    if (text) {
      problems.push(`${rel}:${entry.line}: \`${entry.dotted}\` says \`# ${spelled}\`, and ${at} ships \`${expr}\` for it. Show that value and say \`# default\`.`);
    } else if (!sameValue(entry.value, expr)) {
      problems.push(
        `${rel}:${entry.line}: \`${entry.dotted} = ${entry.value}\` is marked \`# default\`, and ${at} ships \`${expr}\`. ` +
          "Show the value the code ships: the file must not carry a number the crate stopped shipping.",
      );
    }
  }
  return problems;
}

/** Whether the prose above a setting carries its `# NOT IMPLEMENTED` note, owner and all. */
function marked(entry: Entry, owner: string): boolean {
  return entry.prose.some((line) => line.startsWith(UNIMPLEMENTED)) && entry.prose.join(" ").includes(owner);
}

/** `named` as a path relative to the root, spelled with forward slashes, or as given when it is outside. */
function relative(named: string): string {
  const posix = named.replaceAll("\\", "/");
  if (!isAbsolute(named)) {
    return (
      posix
        .split("/")
        .filter((p) => p !== "" && p !== ".")
        .join("/") || "."
    );
  }
  const root = ROOT.replaceAll("\\", "/") + "/";
  return posix.toLowerCase().startsWith(root.toLowerCase()) ? posix.slice(root.length) : posix;
}

/**
 * A default file against the roster, in the five ways that file rots: a key the tree parses and the
 * file omits, a key the file spells and the tree does not parse, a key spelled twice, a setting line
 * whose trailer is missing or wrong (`checkNotes`), and an `[unread:]` key with no `# NOT IMPLEMENTED`
 * note. The path is an argument so that a draft is gated where it is written.
 */
function checkTemplate(keys: Key[], path: string, rel: string): string[] {
  const problems: string[] = [];
  readTrailers(keys, problems);
  if (!existsSync(path)) {
    problems.push(
      `${rel} is not written yet, so there is nothing to gate. It is the default file goal \`config-is-written\` stage 2 ` +
        `generates: each of the ${keys.length} leaf keys \`bun nv directives\` lists, commented out, under a comment ` +
        "saying what it does and what the default is.",
    );
    return problems;
  }

  const text = read(path);
  const liveHeaders = new Set<string>();
  for (const line of text.split("\n")) {
    if (line.startsWith("#")) continue;
    const match = HEADER.exec(line.trim());
    if (match) liveHeaders.add(match[1]!);
  }
  const seen = new Map<string, Entry>();
  const entries = parseTemplate(text);
  for (const entry of entries) {
    if (liveHeaders.has(entry.dotted)) {
      problems.push(
        `${rel}:${entry.line}: \`${entry.dotted}\` is a setting, and \`[${entry.dotted}]\` is a live header in the same ` +
          "file. They are one TOML key, so uncommenting this line gives a file the boot refuses as a duplicate key. " +
          "Comment the header out and say why under it.",
      );
    }
    const key = findKey(keys, entry.dotted);
    if (key === null) {
      problems.push(
        `${rel}:${entry.line}: \`${entry.dotted}\` is not a key ${TREE_REL} parses, so every file that keeps this line is ` +
          "refused at boot the moment it is uncommented. Delete it, or add the field to the tree.",
      );
      continue;
    }
    const earlier = seen.get(key.dotted);
    if (earlier) {
      problems.push(
        `${rel}:${entry.line}: \`${key.dotted}\` is already spelled at line ${earlier.line}. One key, one place -- two of ` +
          "them is how an operator ends up uncommenting the copy that is not the one they read.",
      );
      continue;
    }
    seen.set(key.dotted, entry);
    if (!entry.commented) {
      problems.push(
        `${rel}:${entry.line}: \`${key.dotted}\` is live. Every key in this file is commented out, so that a default this ` +
          "project later tightens still reaches a deployment that took the file once.",
      );
    }
    const stray = entry.lands ? findKey(keys, entry.lands) : null;
    if (stray !== null) {
      problems.push(
        `${rel}:${entry.line}: \`${key.dotted}\` is under a commented-out header, and the live header above it has a key ` +
          `of the same name, \`${stray.dotted}\`. An operator who uncomments this line and not its header gets a file ` +
          "the boot accepts, with the value in the wrong block. Move this block below a live header that has no key of " +
          "this name.",
      );
    }
    if (key.trailer && !marked(entry, key.trailer[1])) {
      problems.push(
        `${rel}:${entry.line}: \`${key.dotted}\` is declared unread at ${anchor(key)} and the file does not say so. Open ` +
          `the comment above it with a \`# ${UNIMPLEMENTED}\` line naming \`${key.trailer[1]}\`, so an operator learns ` +
          "that writing the key does nothing before they write it.",
      );
    }
  }
  for (const key of [...keys].sort((a, b) => (a.dotted < b.dotted ? -1 : a.dotted > b.dotted ? 1 : 0))) {
    if (!seen.has(key.dotted)) {
      problems.push(
        `${rel}: \`${key.dotted}\` parses and this file does not spell it (${anchor(key)}). A key only this tool knows ` +
          "about is one an operator never finds.",
      );
    }
  }
  problems.push(...checkNotes(keys, entries, rel));
  return problems;
}

/** Every problem, then what they add up to -- or the one line that says it is green. */
function report(problems: string[], keys: Key[], green: string): number {
  if (problems.length > 0) {
    console.error(problems.join("\n\n"));
    console.error(`\n${problems.length} problem(s) over ${keys.length} leaf key(s). \`bun nv directives\` lists the roster.`);
    return 1;
  }
  console.log(green);
  return 0;
}

/**
 * The roster, and each block's own doc comment beside it. `readers = false` leaves every key's
 * `readers` empty and skips `findReaders`, which is nearly all of this command's time; only
 * `--check-template` asks for that, because the default file is never judged by who reads a key.
 */
function roster(readers: boolean): [Key[], Map<string, string[]>] {
  const [structs, enums, blocks] = parseTree(read(TREE_REL));
  const keys = walk(structs, enums, ROOT_STRUCT, [], []);
  if (readers) findReaders(keys);
  return [keys, blocks];
}

/** `JSON.stringify` as Python's `json.dumps(indent=2)` writes it: every non-ASCII character escaped. */
function pyJson(value: unknown): string {
  return JSON.stringify(value, null, 2).replace(/[\u007f-￿]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`);
}

const USAGE = [
  "usage: nv directives [-h] [--check] [--json] [--explain KEY]",
  "                     [--check-template [PATH]]",
].join("\n");

export async function run(args: string[]): Promise<number> {
  let parsed;
  try {
    parsed = parseArgs(args, { flags: ["--check", "--json"], valued: ["--explain"], optional: { "--check-template": TEMPLATE_REL } });
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv directives: error: ${e.message}`);
    return 2;
  }
  const { flags, values } = parsed;
  if (flags.has("--help")) {
    console.log(`${USAGE}\n\nnv directives: ${summary}`);
    return 0;
  }
  const template = values.get("--check-template");
  const explained = values.get("--explain");
  try {
    const [keys, blocks] = roster(template === undefined);
    if (flags.has("--json")) {
      readTrailers(keys, []);
      // `doc` and `block_doc` are what `tree.rs` already says about the key and the block holding it.
      // They are maintainer prose, so a generator shortens them for an operator rather than copying them.
      console.log(
        pyJson(
          keys.map((k) => ({
            key: k.dotted,
            block: k.owner,
            field: k.field.name,
            type: k.field.ty,
            anchor: anchor(k),
            doc: k.field.doc,
            block_doc: blocks.get(k.owner) ?? [],
            readers: k.readers,
            unread: k.trailer ? k.trailer[0] : null,
            owner: k.trailer ? k.trailer[1] : null,
          })),
        ),
      );
      return 0;
    }
    if (explained) return explain(keys, explained);
    if (template !== undefined) {
      const path = isAbsolute(template) ? template : join(ROOT, template);
      return report(
        checkTemplate(keys, path, relative(template)),
        keys,
        `directives: ${template} spells all ${keys.length} leaf keys, each once, each saying what unset does`,
      );
    }
    if (flags.has("--check")) return report(check(keys), keys, `directives: ${keys.length} leaf keys, every one read or declared`);

    readTrailers(keys, []);
    const silent = keys.filter((k) => k.readers.length === 0 && !k.trailer).length;
    const declared = keys.filter((k) => k.trailer).length;
    const out = keys.map((k) => `${k.dotted.padEnd(44)} ${k.readers.length === 0 ? "--" : k.readers[0]}`);
    out.push(`\n${keys.length} leaf keys: ${keys.length - silent - declared} read, ${declared} declared unread, ${silent} neither.`);
    console.log(out.join("\n"));
    return 0;
  } catch (e) {
    if (!(e instanceof Fatal)) throw e;
    console.error(e.message);
    return 1;
  }
}
