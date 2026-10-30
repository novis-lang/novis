// `bun nv lints`: every crate's `[lints]` table generated from the workspace's, and checked to have
// stayed that way.
//
//     bun nv lints            rewrite the generated tables
//     bun nv lints --check    exit 1 if any has drifted (CI, nv verify)
//
// It also owns the other key every member manifest inherits: `publish`. Nothing here is published to
// crates.io, so `[workspace.package]` says `publish = false` and every member writes
// `publish.workspace = true`. `--check` refuses a member that does not, so a new crate cannot be
// published by saying nothing. A write does not add the line, because it touches only the lint region.
//
// `[workspace.lints]` in the root manifest is the one home for this repository's lint policy. Most
// crates inherit it with `[lints] workspace = true` and there is nothing to generate. The crates that
// hold `unsafe` cannot: the workspace pins `unsafe_code = "forbid"`, which no inner `#[allow]` can
// relax, so they need `deny` instead, and cargo refuses to let a manifest inherit the workspace table
// and override one entry of it:
//
//     error: cannot override `workspace.lints` in `lints`, either remove the
//            overrides or `lints.workspace = true` and manually specify the lints
//
// So those crates restate the whole table, and a restated table drifts. This command is what makes
// the copies copies: one home, mechanically enforced, the same shape as `nv gen-attribution`.
//
// `UNSAFE_CRATES` below is the roster of manifests that may override, and the reason each one needs
// to. It is the one home for that list, and with this header the one home of the unsafe policy: a
// crate off the roster cannot hold an `unsafe` block at all, so the roster is the whole policy. A
// crate joins it with its reason written beside its name.
//
// Inside a roster crate, each `unsafe` block carries its own `#[allow(unsafe_code, reason = "...")]`
// (or `#[expect]`), a safety-invariant comment, and Miri or ASAN coverage. `unsafe_op_in_unsafe_fn`
// is edition 2024's warn-by-default, and the `-D warnings` both clippy gates run with promotes it, so
// no crate sets it. A coroutine switch is `corosensei`'s, which handles Windows SEH and aarch64, never
// a hand-rolled one.
//
// Tested `unsafe` code is good code, and a lower `unsafe` count is not a goal. Declined: moving a
// platform call into a wrapper crate such as `nix` or `windows`, which hides the `unsafe` without
// removing it; the `undocumented_unsafe_blocks` lint, because each block already states its reason in
// its `#[expect]`; and a safe layer over the helper ABI's raw pointers, which would cost time on every
// call on the request path. Where the same `unsafe` recurs, it moves behind one safe function that
// states the reason once, as `nvs-stdlib`'s array walk is `crate::str::Elements` and `Entries`.
// Making text from bytes without a check is refused by `clippy.toml`'s `disallowed-methods`, which
// is the list's home: clippy resolves the path, so a re-export or a `use` cannot hide a call. An
// audited site states why its bytes are UTF-8 in `#[expect(clippy::disallowed_methods, reason)]`.
// The Windows-only `unsafe` in `nvs-config` and `nvs-cli` runs under no sanitizer, because every
// sanitizer job runs on Linux. That is accepted for startup and install code, and stops being
// accepted if that code grows.
//
// Four properties are worth knowing before changing anything here:
//
// * **`unsafe_code` is the only permitted delta.** A crate on the roster gets the workspace table with
//   that one key changed to `deny`. `--check` compares the rendered region byte for byte against what
//   a write would produce, so a crate cannot quietly weaken a second lint. A crate that needs another
//   exception records it in `EXCEPTIONS`, with a reason, where it is visible.
// * **It fails closed in both directions.** A crate off the roster that carries its own `[lints]`
//   table is an error, and so is a crate with no `[lints]` table at all, so a new crate cannot join
//   the workspace with no lint policy by saying nothing.
// * **A write touches the lint region and nothing else.** The region runs from the generated marker,
//   or the first `[lints` header, to the last line of the last lint table. A table after it and the
//   comment above that table are kept as they were, and every line is written with `\n`.
// * **A write is checked by cargo.** `cargo metadata` reads the workspace after the rewrite, and a
//   manifest it cannot read is put back as it was before the command reports the failure.

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { parse as parseToml } from "smol-toml";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs } from "../lib/py.ts";

export const summary = "every unsafe crate's [lints] table, generated from [workspace.lints]: nv lints [--check]";

const USAGE = "usage: nv lints [-h] [--check]";

type Tables = Record<string, Record<string, unknown>>;

/** The manifests that may override the workspace table, and why each must. A crate joins it only
 *  with a reason that says why, per this file's header. */
export const UNSAFE_CRATES: Record<string, string> = {
  "crates/nvs-runtime":
    "the helper ABI's `extern \"C\"` entry points; the reference-counted strings, arrays and objects they reach through raw " +
    "pointers, with the class descriptors objects are built from; and the global allocator, whose per-thread block cache " +
    "and per-request byte counter forward to `System`",
  "crates/nvs-stdlib":
    "a `Core` member retains and releases the runtime's reference-counted values, adopts a raw string, array or object " +
    "pointer as a handle, builds or decodes an object through its class descriptor, and walks an array through the " +
    "runtime's slot cursor, and `Core\\Json` takes serde_json's output as UTF-8 unchecked; its tests write `extern \"C\"` callees in compiled code's signature",
  "crates/nvs-codegen":
    "JIT page mapping, unmapping and instruction-cache flushing, transmuting a compiled code pointer to a callable function " +
    "pointer, `Send` and `Sync` for a unit that holds code addresses, and constructing an object through the runtime",
  "crates/nvs-host":
    "the coroutine stack switcher's one deref of the opaque yielder, a task group's and an isolate's release of the " +
    "references they are handed and their child contexts, the per-thread CPU clock through the platform API, and the " +
    "ThreadSanitizer fiber hooks under the `tsan` feature",
  "crates/nvs-config":
    "the config file permission check (`geteuid` on Unix, the file's ACL through `advapi32` on Windows), reading the " +
    "physical memory size, and creating a Windows data directory with a security descriptor",
  "crates/nvs-cli":
    "signal handling and its self-pipe, Windows service registration and dispatch, the account lookup, the monotonic " +
    "clock, the socket `umask`, making a cached code pointer callable, and the debug adapter's standard streams and " +
    "launch environment",
  "crates/nvs-ext":
    "loading an extension's compiled component back from the artifact cache, whose bytes wasmtime runs as machine code without validating them; " +
    "and the `ext_bytes` bench's global allocator, which forwards to `System` and counts the host bytes a call allocates",
  "benches/abi-probe":
    "it calls JIT-compiled code to measure it; `publish = false` and no shipped crate depends on it, so it does not widen the runtime's unsafe surface",
};

/** Per-manifest lint exceptions beyond `unsafe_code`. An entry here is a hole in the policy and has to
 *  be argued for; what it buys over a silent omission is that the hole is written down, next to its
 *  reason. */
export const EXCEPTIONS: Record<string, Tables> = {
  // This crate builds Cranelift IR to call, so it casts host addresses and slot indices into the `i64`
  // and `i32` an `iconst` takes. The cast is the measurement: a probe that widened through `try_from`
  // would be measuring the conversion rather than the ABI. The sites are all in `src/lib.rs`, and
  // `publish = false` keeps them out of anything shipped. The stricter fix is a per-site
  // `#[expect(..., reason = ...)]`, which would put the argument next to each cast instead of here.
  "benches/abi-probe": {
    clippy: { cast_possible_truncation: "allow", cast_possible_wrap: "allow", cast_sign_loss: "allow" },
  },
};

export const MARKER = "# @generated by `bun nv lints` -- edit `[workspace.lints]` in the root manifest.";
const PREAMBLE = [
  MARKER,
  "# This crate is on that command's `UNSAFE_CRATES` roster, so it restates the",
  '# workspace table with `unsafe_code = "deny"`: cargo will not let a manifest',
  "# inherit `[workspace.lints]` and override one entry, and `forbid` is not",
  "# something an individual `#[allow(unsafe_code, reason = ...)]` can relax.",
];

class Fatal extends Error {}

/** One lint value, written the way the root manifest writes it. */
export function renderValue(value: unknown): string {
  if (typeof value === "string") return `"${value}"`;
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number" && Number.isInteger(value)) return String(value);
  if (typeof value === "bigint") return String(value);
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    const inner = Object.entries(value).map(([k, v]) => `${k} = ${renderValue(v)}`);
    return `{ ${inner.join(", ")} }`;
  }
  throw new Fatal(`nv lints: cannot render lint value ${JSON.stringify(value)}`);
}

/** The lint tables `rel` must carry: the workspace's, with its own deltas applied. */
export function expectedTables(wsLints: Tables, rel: string): Tables {
  const tables: Tables = {};
  for (const [group, entries] of Object.entries(wsLints)) tables[group] = { ...entries };
  tables.rust = { ...tables.rust, unsafe_code: "deny" };
  for (const [group, entries] of Object.entries(EXCEPTIONS[rel] ?? {})) tables[group] = { ...tables[group], ...entries };
  return tables;
}

export function renderRegion(tables: Tables): string {
  const lines = [...PREAMBLE];
  for (const [group, entries] of Object.entries(tables)) {
    lines.push("", `[lints.${group}]`);
    for (const [key, value] of Object.entries(entries)) lines.push(`${key} = ${renderValue(value)}`);
  }
  return lines.join("\n") + "\n";
}

const isGeneratedMarker = (line: string) => /^# @generated by .*lints/.test(line);
const isLintsHeader = (line: string) => /^\[lints(\.[A-Za-z0-9_-]+)?\]/.test(line);
const isHeader = (line: string) => /^\s*\[/.test(line);
const isContent = (line: string) => line.trim() !== "" && !line.trimStart().startsWith("#");

/**
 * A manifest cut into what sits above its lint region, the region, and what follows it.
 *
 * The region starts at the generated marker, or at the first `[lints` header on a manifest that was
 * never generated. It ends after the last content line of the last lint table, so a blank line or a
 * comment block in front of the next table belongs to that table. The hand-written comment above the
 * region that explains why a crate needs `unsafe` is in `head` and is never touched.
 */
export function splitManifest(text: string): { head: string; region: string; tail: string } {
  const lines = text.replace(/\r\n/g, "\n").split(/(?<=\n)/);
  const start = lines.findIndex((l) => isGeneratedMarker(l) || isLintsHeader(l));
  if (start < 0) return { head: text, region: "", tail: "" };
  let end = start + 1;
  let inLints = isLintsHeader(lines[start]!);
  for (let i = start + 1; i < lines.length; i++) {
    const line = lines[i]!;
    if (isHeader(line)) {
      if (!isLintsHeader(line)) break;
      inLints = true;
      end = i + 1;
    } else if (inLints && isContent(line)) end = i + 1;
  }
  // A marker with no table under it is still the region's start: the preamble goes with it.
  if (!inLints) while (end < lines.length && lines[end]!.startsWith("#")) end++;
  return {
    head: lines.slice(0, start).join(""),
    region: lines.slice(start, end).join(""),
    tail: lines.slice(end).join("").replace(/^\n+/, ""),
  };
}

/** `head`, the generated region and `tail`, joined with one blank line between each. */
export function joinManifest(head: string, region: string, tail: string): string {
  return head.trimEnd() + "\n\n" + region + (tail ? "\n" + tail : "");
}

/** Every workspace member that has a manifest, from the root's own globs, repo-relative. */
function members(root: string, manifest: Record<string, unknown>): string[] {
  const ws = (manifest.workspace ?? {}) as { members?: string[]; exclude?: string[] };
  const excluded = (ws.exclude ?? []).map((e) => e.replace(/\/+$/, ""));
  const found: string[] = [];
  for (const pattern of ws.members ?? []) {
    const hits = [...new Bun.Glob(pattern).scanSync({ cwd: root, onlyFiles: false })].map((p) => p.replace(/\\/g, "/")).sort();
    for (const rel of hits) {
      if (excluded.some((ex) => ex === rel || new Bun.Glob(ex).match(rel))) continue;
      if (existsSync(join(root, rel, "Cargo.toml"))) found.push(rel);
    }
  }
  return found;
}

function unifiedDiff(before: string, after: string, from: string): string {
  const a = before.split("\n");
  const b = after.split("\n");
  const out = [`--- ${from}`, "+++ generated"];
  const max = Math.max(a.length, b.length);
  for (let i = 0; i < max; i++) {
    if (a[i] === b[i]) continue;
    if (a[i] !== undefined) out.push(`-${a[i]}`);
    if (b[i] !== undefined) out.push(`+${b[i]}`);
  }
  return out.join("\n");
}

export interface Outcome {
  problems: string[];
  /** Repo-relative crate directories whose manifest the write changed, with the text it replaced. */
  rewritten: { rel: string; before: string }[];
}

/** What `--check` finds, or what a write changes, over the workspace at `root`. */
export function lintTables(root: string, write: boolean): Outcome {
  const rootManifest = parseToml(readFileSync(join(root, "Cargo.toml"), "utf8")) as Record<string, unknown>;
  const wsLints = ((rootManifest.workspace ?? {}) as { lints?: Tables }).lints;
  if (!wsLints || Object.keys(wsLints).length === 0) throw new Fatal("nv lints: the root manifest has no [workspace.lints] table");
  const out: Outcome = { problems: [], rewritten: [] };
  const known = new Set(Object.keys(UNSAFE_CRATES));

  const wsPackage = ((rootManifest.workspace ?? {}) as { package?: Record<string, unknown> }).package ?? {};
  if (wsPackage.publish !== false) out.problems.push("Cargo.toml: [workspace.package] does not say `publish = false`.");

  for (const rel of members(root, rootManifest)) {
    const path = join(root, rel, "Cargo.toml");
    const text = readFileSync(path, "utf8");
    const parsed = parseToml(text) as Record<string, unknown>;
    const declared = (parsed.lints ?? {}) as Record<string, unknown>;
    const publish = ((parsed.package ?? {}) as { publish?: unknown }).publish as { workspace?: unknown } | undefined;
    if (publish?.workspace !== true) {
      out.problems.push(`${rel}/Cargo.toml: does not inherit publish. Write \`publish.workspace = true\` under [package].`);
    }

    if (!(rel in UNSAFE_CRATES)) {
      if (!declared.workspace) {
        out.problems.push(
          Object.keys(declared).length > 0
            ? `${rel}/Cargo.toml: carries its own [lints] table but is not on nv lints' UNSAFE_CRATES roster. Inherit with ` +
                "`[lints] workspace = true`, or add it to the roster with the reason it needs `unsafe`."
            : `${rel}/Cargo.toml: has no [lints] table at all. Add \`[lints]\` / \`workspace = true\`.`,
        );
      }
      continue;
    }

    known.delete(rel);
    const { head, region, tail } = splitManifest(text);
    const wanted = renderRegion(expectedTables(wsLints, rel));
    const next = joinManifest(head, wanted, tail);
    if (region === wanted && next === text) continue;
    if (write) {
      writeFileSync(path, next);
      out.rewritten.push({ rel, before: text });
    } else {
      const why = region === wanted ? " (its line endings or the blank lines around the region)" : "";
      out.problems.push(`${rel}/Cargo.toml has drifted${why}:\n` + unifiedDiff(region, wanted, `${rel}/Cargo.toml`));
    }
  }

  for (const missing of [...known].sort()) {
    out.problems.push(`${missing}: on nv lints' UNSAFE_CRATES roster but is not a workspace member. Remove the entry, or fix the path.`);
  }
  return out;
}

async function checkOrWrite(write: boolean): Promise<number> {
  const { problems, rewritten } = lintTables(ROOT, write);
  if (rewritten.length > 0) {
    const meta = await runProc(["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"]);
    if (meta.code !== 0) {
      for (const { rel, before } of rewritten) writeFileSync(join(ROOT, rel, "Cargo.toml"), before);
      console.error(`${meta.stderr.trim()}\n\nnv lints: cargo cannot read the rewritten workspace, so every manifest was put back.`);
      return 1;
    }
  }
  if (problems.length > 0) {
    console.error(problems.join("\n\n"));
    console.error(`\n${problems.length} problem(s). \`bun nv lints\` rewrites what it can.`);
    return 1;
  }
  if (write) console.log(rewritten.length > 0 ? `lints: rewrote ${rewritten.length}` : "lints: already current");
  else console.log(`lints: ${Object.keys(UNSAFE_CRATES).length} generated tables current`);
  return 0;
}

function help(): string {
  return [
    USAGE,
    "",
    "Generate every unsafe crate's [lints] table from [workspace.lints], and check it stayed that way.",
    "",
    "options:",
    "  -h, --help  show this help message and exit",
    "  --check     exit 1 if any generated table has drifted, changing nothing",
  ].join("\n");
}

export async function run(args: string[]): Promise<number> {
  let flags: Set<string>;
  try {
    ({ flags } = parseArgs(args, { flags: ["--check"], valued: [] }));
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv lints: error: ${e.message}`);
    return 2;
  }
  if (flags.has("--help")) {
    console.log(help());
    return 0;
  }
  try {
    return await checkOrWrite(!flags.has("--check"));
  } catch (e) {
    if (!(e instanceof Fatal)) throw e;
    console.error(e.message);
    return 1;
  }
}
