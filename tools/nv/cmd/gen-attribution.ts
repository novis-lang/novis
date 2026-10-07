// `bun nv gen-attribution`: `THIRD-PARTY-LICENSES.txt` from the resolved dependency graph, and the
// default binary's C dependencies held against the ledger
// `rule:packaging/a-c-dependency-answers-two-questions` asks for.
//
//     bun nv gen-attribution                   regenerate the notice
//     bun nv gen-attribution --check           exit 1 if the committed notice is stale (CI)
//     bun nv gen-attribution --check-c-deps    list the C dependencies; exit 1 on one the ledger does not record
//
// The notice is the one file that reproduces every third-party license with the binary, for the
// repository and, through `include_str!` in `nvs-cli`, for the shipped `nvs` itself.
// `rule:packaging/the-third-party-notice-is-generated-never-written-by-hand` owns the policy. Three
// properties hold it together:
//
// * **It fails closed.** An SPDX identifier `PREFERENCE` does not rank, a crate whose source is not
//   fetched, or a chosen license with no text anywhere in the tree is an error, never an omitted
//   notice. `deny.toml` decides what is *allowed*, this decides what is *shipped*, and the two lists
//   are checked against each other on every run.
// * **It is host-independent.** The component list is not filtered by target, so a Windows and a Linux
//   checkout produce the same bytes and `--check` means something in CI. `windows-sys` is listed on
//   Linux, which is the right direction to err for attribution.
// * **License texts are deduplicated by content, not by identifier.** MIT requires each component's
//   own copyright line: crates whose text is byte-identical share one entry, and crates whose
//   copyright differs do not.
//
// The notice's package set is every package reachable from `nvs-cli` through a normal or build
// dependency, and every package reachable the same way from each built-in component's crate
// (`BUILT_IN_COMPONENTS`), which is compiled to wasm and embedded in the binary. The C libraries a
// component carries prebuilt (`PREBUILT_C`) have no manifest, so their licence files are read from
// beside the library.
//
// The C-dependency enumeration reads the binary's package set alone. A package counts when it
// declares `links`, or when it build-depends on a tool that compiles C (`C_BUILD_TOOLS`). Either
// signal can fire on a crate that builds nothing native, and the ledger entry is what answers it. The
// check fails in both directions: on a C dependency with no entry, and on an entry naming a crate the
// graph no longer signals. It then lists each prebuilt library, whose answer is that it runs only
// inside a component, and fails when one is missing or differs from its recorded digest.

import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs, pyRepr } from "../lib/py.ts";
import { checkLibrary, readRecord } from "./webp-lib.ts";

export const summary = "THIRD-PARTY-LICENSES.txt from the dependency graph, and the C-dependency ledger: nv gen-attribution [--check|--check-c-deps]";

const USAGE = "usage: nv gen-attribution [-h] [--check] [--check-c-deps]";

const OUTPUT_NAME = "THIRD-PARTY-LICENSES.txt";
const OUTPUT = join(ROOT, OUTPUT_NAME);
const DENY = join(ROOT, "deny.toml");

// The package whose dependency set is distributed. Everything else in the workspace is a library
// it links, or a bench or fuzz target that ships to nobody.
const ROOT_PACKAGE = "nvs-cli";
const BINARY = "nvs";

// Which half of a dual license Novis takes, most-preferred first. MIT leads because Novis is MIT: one
// license covering the most components keeps the notice short, which is why an order exists rather
// than "reproduce every offered license". An identifier absent here is an error, so a new license
// entering the tree is a decision somebody makes.
//
// It names exactly the identifiers `deny.toml`'s `licenses.allow` does, only ordered: that file decides
// what Novis may *link*, this one what it ships and which half of a choice. `checkPoliciesAgree` fails
// on a drift in either direction.
const PREFERENCE = [
  "MIT",
  // MIT with the attribution clause struck out, which is how `borrow-or-share` is offered. The text is
  // MIT's, so it ranks beside MIT, and no crate here offers it as half of a choice.
  "MIT-0",
  "Apache-2.0 WITH LLVM-exception",
  "Apache-2.0",
  "BSD-3-Clause",
  "BSD-2-Clause",
  "ISC",
  "Zlib",
  "Unicode-3.0",
  "CC0-1.0",
  // The Community Data License Agreement's permissive variant, which `webpki-roots` offers Mozilla's CA
  // set under. A data license, and one with no notice requirement; it is reproduced anyway.
  "CDLA-Permissive-2.0",
  "MPL-2.0",
  // Last, so that `Unlicense OR MIT` — how the `regex` family is offered — always takes MIT and the
  // notice stays one license shorter.
  "Unlicense",
];

// Content fingerprints, checked in order: a license file is classified by what it says, not by what it
// is called, so LICENSE, LICENSE-MIT and license.txt all work, and a LICENSE that is Apache is filed as
// Apache. Most specific first: the Unicode license opens with MIT's "Permission is hereby granted, free
// of charge", BSD-3's text contains BSD-2's, and MIT-0's contains MIT's.
const FINGERPRINTS: [string, string[]][] = [
  ["Apache-2.0 WITH LLVM-exception", ["apache license", "llvm exception"]],
  ["Apache-2.0", ["apache license", "version 2.0"]],
  ["Unicode-3.0", ["unicode license v3"]],
  ["Unlicense", ["this is free and unencumbered software released into the public domain"]],
  ["ISC", ["permission to use, copy, modify, and/or distribute this software"]],
  ["Zlib", ["altered source versions must be plainly marked as such"]],
  ["MPL-2.0", ["mozilla public license version 2.0"]],
  ["CC0-1.0", ["creative commons legal code", "cc0 1.0 universal"]],
  ["CDLA-Permissive-2.0", ["community data license agreement", "permissive"]],
  ["BSD-3-Clause", ["redistribution and use in source and binary forms", "endorse or promote"]],
  ["BSD-2-Clause", ["redistribution and use in source and binary forms"]],
  ["MIT-0", ["mit no attribution"]],
  ["MIT", ["permission is hereby granted, free of charge"]],
];

const LICENSE_FILE_PREFIXES = ["license", "licence", "copying", "unlicense"];

const WIDTH = 78;

// Build dependencies that mean "this crate compiles or links C". `cc` and `cmake` also compile
// assembly and C++, which is the same question for this ledger: memory-unsafe code the Rust
// toolchain did not check.
const C_BUILD_TOOLS = new Set(["cc", "cmake", "pkg-config", "bindgen", "nasm-rs", "meson"]);

// The verdicts an entry may carry. `no-native-code`: a signal fired but this tree builds nothing
// native from the crate. `unreachable`: attacker-controlled data does not reach the code.
// `verified`: it does, and the record names the verification that answers the second question.
const VERDICTS = ["no-native-code", "unreachable", "verified"];

interface Entry {
  verdict: string;
  /** Features that must stay active for the verdict to hold. `cargo metadata` lists a build
   * dependency whether or not the feature that uses it is on, so the flag is checked here. */
  requires: string[];
  /** One or two sentences, pointing at `Cargo.toml`'s comment on the dependency, where it was chosen. */
  record: string;
}

// The ledger: every C dependency of the default binary. Adding an entry is a decision somebody makes
// and writes down; the gate is that a person wrote the sentence, not that a tool inferred it.
const C_DEPENDENCIES: Record<string, Entry> = {
  ring: {
    verdict: "verified",
    requires: [],
    record:
      "The one genuine C dependency in the default binary: BoringSSL's pregenerated assembly behind a " +
      "Rust API. It is rustls's crypto provider, and `nvs-stdlib` reaches it directly for JWS's four " +
      "signature algorithms — RSASSA-PKCS1-v1_5, RSASSA-PSS, ECDSA over P-256 and Ed25519. Question 1 is " +
      "yes on both counts — a TLS record layer and a token verifier are each exactly where attacker bytes " +
      "land — and question 2 is answered once for both by OSS-Fuzz and BoringSSL's formally verified " +
      "field arithmetic. Cargo.toml's `rustls` comment is the home of the transport decision, including " +
      "why the wasm branch is not available to a client that owns its socket, and its `ring` row is the " +
      "home of the signature one.",
  },
  "libsqlite3-sys": {
    verdict: "verified",
    requires: ["bundled"],
    record:
      "SQLite itself, compiled from the amalgamation by `rusqlite`'s sys crate. Question 1 is yes -- a " +
      "database engine parses SQL an application composed and stores bytes a request supplied -- and " +
      "question 2 is the one record `rule:core-api/tier-placement` section 4 names by hand: TH3, 100% " +
      "MC/DC branch coverage over the whole library, plus a continuous fuzzing corpus and the anomaly log " +
      "SQLite publishes against every release. `bundled` is required rather than incidental, and " +
      "Cargo.toml's `rusqlite` comment is the home of why -- a host's own libsqlite3 is a different build " +
      "of a different version, and the verification record belongs to the one this tree compiles.",
  },
  "sqlite-wasm-rs": {
    verdict: "no-native-code",
    requires: [],
    record:
      "`libsqlite3-sys`'s wasm32 half, and a target-gated dependency this tree never compiles: it is " +
      "reached only under `cfg(target_arch = \"wasm32\")`, and Novis's own wasm target is the extension " +
      "sandbox, which links no database at all. It is in the graph for the same reason `windows-sys` is " +
      "listed on Linux -- this enumeration is deliberately host-independent, so a Windows and a Linux " +
      "checkout produce the same bytes. The engine this binary actually runs is `libsqlite3-sys`'s " +
      "bundled amalgamation, immediately above.",
  },
  blake3: {
    verdict: "no-native-code",
    requires: ["pure"],
    record:
      "Ships hand-written assembly built through `cc` by default, and this tree takes " +
      "`default-features = false` with `pure` instead, so nothing native is compiled. Cargo.toml's " +
      "`blake3` comment is the home of why: the portable implementation is already faster than SHA-256, " +
      "so the pure-Rust default costs nothing worth spending an exception on.",
  },
  defmt: {
    verdict: "no-native-code",
    requires: [],
    record:
      "`links = \"defmt\"` is Cargo's one-version token, not a native library — the crate is a logging " +
      "framework for embedded targets and compiles no C. It is in the graph only because this " +
      "enumeration is host-independent, the same way `windows-sys` is listed on Linux.",
  },
  "wasm-bindgen-shared": {
    verdict: "no-native-code",
    requires: [],
    record:
      "`links = \"wasm_bindgen\"` is the same one-version token as `defmt`'s, used to keep the macro and " +
      "the runtime at one version. It builds no C, and it is reached only through a " +
      "`cfg(target_arch = \"wasm32\")` dependency that no shipped `nvs` binary compiles.",
  },
  "rayon-core": {
    verdict: "no-native-code",
    requires: [],
    record:
      "`links = \"rayon-core\"` is the same one-version token as `defmt`'s, keeping one global thread " +
      "pool per process. It builds no C. wasmtime's `parallel-compilation` reaches it, and so does " +
      "`criterion` for the benches.",
  },
  wasmtime: {
    verdict: "unreachable",
    requires: [],
    record:
      "Its build script compiles `helpers.c`, some fifty lines of trampolines a debugger calls to " +
      "read a guest's memory, and one function that says whether the unwinder is libunwind. No guest " +
      "and no request reaches them: they exist for a person stepping through JIT code under gdb or " +
      "lldb. Everything that runs a guest is Rust.",
  },
  "wasmtime-internal-fiber": {
    verdict: "unreachable",
    requires: [],
    record:
      "Compiles `windows.c` on Windows only, the stack switch a guest's async call runs on, through " +
      "the Win32 fiber API. It moves no data: it switches stacks, and what runs on them is Rust and " +
      "compiled wasm. On every other host the switch is inline assembly in Rust.",
  },
  "wasmtime-internal-jit-debug": {
    verdict: "unreachable",
    requires: [],
    record:
      "Compiles `gdbjit.c` under `gdb_jit_int`, one of wasmtime's default features: the registration " +
      "hook GDB reads JIT code from. It reads the compiled image of a module the configuration loaded, " +
      "only when debug info is turned on, which `nvs_ext::call`'s `Config` never does.",
  },
  "ittapi-sys": {
    verdict: "unreachable",
    requires: [],
    record:
      "Intel's ITT notify library, behind wasmtime's default `profiling` feature, for VTune. Only a " +
      "`Config` that sets the VTune profiling strategy calls it, and `nvs_ext::call`'s does not, so " +
      "nothing a request sends reaches it.",
  },
  "zstd-sys": {
    verdict: "unreachable",
    requires: [],
    record:
      "zstd, behind wasmtime's default `cache` feature, which compresses wasmtime's own on-disk module " +
      "cache. `nvs` never turns that cache on: a compiled component is stored through " +
      "`nvs_ext::load::ModuleCache` in the artifact cache " +
      "(`rule:packaging/a-wasm-module-cache-reuses-the-artifact-cache`), so the library is linked and " +
      "never called.",
  },
};

// The built-in components: Rust crates outside the workspace, compiled for `wasm32-wasip2` and
// embedded in the binary (`rule:packaging/the-first-party-components-are-built-in`). Their crates ship
// with the binary, so the notice carries them.
const BUILT_IN_COMPONENTS = [{ name: "image", manifest: "extensions/image/Cargo.toml" }];

// The target every component is built for. Its graph is resolved for this target alone, because a
// dependency behind another platform's `cfg`, or behind `cfg(fuzzing)`, is never linked.
const COMPONENT_TARGET = "wasm32-wasip2";

// The C libraries inside a built-in component, each prebuilt for wasm by its own `bun nv` tool
// (`rule:packaging/a-prebuilt-wasm-library-is-rebuilt-in-ci`). Confined to wasm is the answer
// `rule:packaging/a-c-dependency-answers-two-questions` gives a codec, so the record is the
// confinement: the library must exist and match its digest. `texts` are reproduced in the notice.
interface Prebuilt {
  name: string;
  component: string;
  dir: string;
  license: string;
  texts: string[];
}
const PREBUILT_C: Prebuilt[] = [
  { name: "libwebp", component: "image", dir: "extensions/image/libwebp", license: "BSD-3-Clause", texts: ["COPYING", "PATENTS"] },
];

interface Package {
  id: string;
  name: string;
  version: string;
  source: string | null;
  links?: string | null;
  license?: string | null;
  manifest_path: string;
}
interface Dep {
  pkg: string;
  dep_kinds: { kind: string | null }[];
}
interface Node {
  id: string;
  deps: Dep[];
  features: string[];
}
interface Metadata {
  packages: Package[];
  workspace_members: string[];
  resolve: { nodes: Node[] };
}

class Fatal extends Error {}

/** `cargo metadata` for the workspace, or for the crate `manifest` names when it is given. */
async function cargoMetadata(manifest?: string, platform?: string): Promise<Metadata> {
  const at = manifest ? ["--manifest-path", join(ROOT, manifest)] : [];
  const on = platform ? ["--filter-platform", platform] : [];
  const r = await runProc(["cargo", "metadata", "--format-version", "1", "--locked", ...at, ...on], { cwd: ROOT });
  if (r.code !== 0) throw new Fatal(`error: cargo metadata${manifest ? ` for ${manifest}` : ""} failed:\n${r.stderr.trim()}`);
  return JSON.parse(r.stdout) as Metadata;
}

/** Every third-party package reachable from `roots`, by default the binary's package, through a
 * normal or build dependency. */
function shippedPackages(meta: Metadata, roots?: string[]): Package[] {
  const byId = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const stack = roots ?? meta.packages.filter((p) => p.name === ROOT_PACKAGE).map((p) => p.id);
  if (stack.length === 0) throw new Fatal(`error: no package named ${ROOT_PACKAGE} in this workspace`);
  const seen = new Set<string>();
  while (stack.length > 0) {
    const id = stack.pop()!;
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dep of nodes.get(id)!.deps) {
      if (dep.dep_kinds.some((k) => k.kind === null || k.kind === "build")) stack.push(dep.pkg);
    }
  }
  // A workspace member is one of Novis's own crates, covered by LICENSE. Every other package is
  // third party, including a path crate under `vendor/` that `[patch.crates-io]` puts in place of
  // the published one: it has no `source`, and its authors' licence still ships.
  const own = new Set(meta.workspace_members);
  return [...seen]
    .map((id) => byId.get(id)!)
    .filter((p) => !own.has(p.id))
    .sort((a, b) => cmp(a.name.toLowerCase(), b.name.toLowerCase()) || cmp(a.version, b.version));
}

function cmp(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** The binary's third-party packages, then every built-in component's, each name and version once. */
async function allShippedPackages(meta: Metadata): Promise<Package[]> {
  const all = new Map(shippedPackages(meta).map((p) => [`${p.name} ${p.version}`, p]));
  for (const component of BUILT_IN_COMPONENTS) {
    const own = await cargoMetadata(component.manifest, COMPONENT_TARGET);
    for (const p of shippedPackages(own, own.workspace_members)) {
      if (!all.has(`${p.name} ${p.version}`)) all.set(`${p.name} ${p.version}`, p);
    }
  }
  return [...all.values()].sort((a, b) => cmp(a.name.toLowerCase(), b.name.toLowerCase()) || cmp(a.version, b.version));
}

/** A prebuilt library's version, read from the record its tool writes. */
function prebuiltVersion(lib: Prebuilt): string {
  return readRecord(join(ROOT, lib.dir)).source.version;
}

/** Every shipped package that compiles or links C, as `[name, version, signal]`, sorted. */
function cDependencies(meta: Metadata, packages: Package[]): [string, string, string][] {
  const byId = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const found: [string, string, string][] = [];
  for (const pkg of packages) {
    if (pkg.links) {
      found.push([pkg.name, pkg.version, `declares \`links = "${pkg.links}"\``]);
      continue;
    }
    const tools = [
      ...new Set(
        nodes
          .get(pkg.id)!
          .deps.filter((d) => d.dep_kinds.some((k) => k.kind === "build"))
          .map((d) => byId.get(d.pkg)!.name)
          .filter((name) => C_BUILD_TOOLS.has(name)),
      ),
    ].sort(cmp);
    if (tools.length > 0) found.push([pkg.name, pkg.version, `build-depends on ${tools.map((t) => `\`${t}\``).join(", ")}`]);
  }
  return found.sort((a, b) => cmp(a[0], b[0]) || cmp(a[1], b[1]) || cmp(a[2], b[2]));
}

async function checkCDeps(): Promise<number> {
  const meta = await cargoMetadata();
  const packages = shippedPackages(meta);
  const found = cDependencies(meta, packages);
  const active = new Map<string, Set<string>>();
  for (const node of meta.resolve.nodes) {
    for (const pkg of packages) if (pkg.id === node.id) active.set(pkg.name, new Set(node.features));
  }
  const present = new Set(found.map(([name]) => name));
  const problems: string[] = [];
  const rule = "`rule:packaging/a-c-dependency-answers-two-questions`";

  console.log(`C dependencies of the \`${BINARY}\` binary: ${found.length} in the graph, ${Object.keys(C_DEPENDENCIES).length} recorded`);
  for (const [name, version, reason] of found) {
    const entry = C_DEPENDENCIES[name];
    if (!entry) {
      console.log(`  ${name} ${version} — ${reason} [NOT RECORDED]`);
      problems.push(
        `  - ${name} ${version} ${reason}, and nothing in this file records it.\n` +
          `    Answer ${rule}'s two questions — does attacker-controlled data\n` +
          `    reach it, and if so what is its verification record — and add the\n` +
          `    entry to C_DEPENDENCIES in tools/nv/cmd/gen-attribution.ts, or confine the\n` +
          `    code to wasm as § 4's second question requires.`,
      );
      continue;
    }
    console.log(`  ${name} ${version} — ${reason} [${entry.verdict}]`);
    if (!VERDICTS.includes(entry.verdict)) {
      problems.push(`  - ${name} is recorded with the verdict ${pyRepr(entry.verdict)}, which is not one\n    of ${VERDICTS.join(", ")}.`);
    }
    const missing = entry.requires.filter((f) => !(active.get(name) ?? new Set()).has(f));
    if (missing.length > 0) {
      problems.push(
        `  - ${name}'s record holds only while it is built with ${entry.requires.map(pyRepr).join(", ")},\n` +
          `    and ${missing.map(pyRepr).join(", ")} is no longer active. Either restore\n` +
          `    the feature in Cargo.toml or answer ${rule} for the native code\n` +
          `    it now compiles.`,
      );
    }
  }
  for (const name of Object.keys(C_DEPENDENCIES).sort(cmp)) {
    if (!present.has(name)) {
      problems.push(
        `  - ${name} is recorded in C_DEPENDENCIES but nothing in the graph\n` +
          `    signals it any more. Delete the entry: a ledger of dependencies\n` +
          `    that left is a ledger nobody trusts.`,
      );
    }
  }
  console.log(`C libraries inside the built-in components: ${PREBUILT_C.length}, each compiled to wasm`);
  for (const lib of PREBUILT_C) {
    const problem = checkLibrary(join(ROOT, lib.dir));
    if (problem) {
      console.log(`  ${lib.name} — prebuilt under ${lib.dir}/ [NOT CONFINED]`);
      problems.push(`  - ${lib.name}: ${problem}`);
      continue;
    }
    console.log(`  ${lib.name} ${prebuiltVersion(lib)} — wasm static library under ${lib.dir}/, inside the ${lib.component} component [confined-to-wasm]`);
  }
  if (problems.length === 0) return 0;
  console.error("error: the C-dependency ledger is out of date:");
  console.error(problems.join("\n"));
  return 1;
}

// ---------------------------------------------------------------------------------------------------
// SPDX expressions
// ---------------------------------------------------------------------------------------------------

type Spdx = { kind: "id"; ident: string } | { kind: "and" | "or"; terms: Spdx[] };

/**
 * An SPDX expression as a tree. Only what Cargo manifests use is handled: OR, AND, WITH and
 * parentheses. `WITH` binds tightest and is folded into the identifier it qualifies, which is how
 * `PREFERENCE` and `deny.toml` name those licenses.
 */
function parseSpdx(expr: string): Spdx {
  // Some crates still write the pre-SPDX `MIT/Apache-2.0`.
  const tokens = pySplit(expr.replaceAll("/", " OR ").replaceAll("(", " ( ").replaceAll(")", " ) "));
  const at = pyRepr(expr);
  let pos = 0;
  const peek = (): string | undefined => tokens[pos];

  const primary = (): Spdx => {
    let node: Spdx;
    if (peek() === "(") {
      pos += 1;
      node = expression();
      if (peek() !== ")") throw new Fatal(`unbalanced parentheses in ${at}`);
      pos += 1;
    } else {
      const token = peek();
      if (token === undefined || token === "AND" || token === "OR" || token === ")") {
        throw new Fatal(`expected a license identifier in ${at}`);
      }
      pos += 1;
      node = { kind: "id", ident: token };
    }
    if (peek() === "WITH") {
      pos += 1;
      const exception = peek();
      if (exception === undefined) throw new Fatal(`dangling WITH in ${at}`);
      pos += 1;
      if (node.kind !== "id") throw new Fatal(`WITH applied to a compound expression in ${at}`);
      node = { kind: "id", ident: `${node.ident} WITH ${exception}` };
    }
    return node;
  };
  const series = (op: "and" | "or", next: () => Spdx): Spdx => {
    const terms = [next()];
    const word = op.toUpperCase();
    while (peek() === word) {
      pos += 1;
      terms.push(next());
    }
    return terms.length === 1 ? terms[0]! : { kind: op, terms };
  };
  const expression = (): Spdx => series("or", () => series("and", primary));

  const tree = expression();
  if (pos !== tokens.length) throw new Fatal(`trailing tokens in ${at}`);
  return tree;
}

/** An SPDX identifier `PREFERENCE` ranks nowhere, thrown so an OR can decline one branch and take another. */
class Undeclared extends Error {
  ident: string;
  constructor(ident: string) {
    super(ident);
    this.ident = ident;
  }
}

/**
 * The licenses Novis takes from an SPDX tree. An OR collapses to its most-preferred option; an AND
 * keeps every conjunct, so `(MIT OR Apache-2.0) AND BSD-3-Clause` reproduces two notices.
 */
function choose(node: Spdx, where: string): string[] {
  try {
    return take(node);
  } catch (e) {
    if (!(e instanceof Undeclared)) throw e;
    throw new Fatal(
      `error: ${where} offers ${pyRepr(e.ident)}, which tools/nv/cmd/gen-attribution.ts has no\n` +
        `       policy for. Add it to PREFERENCE (and to deny.toml's allow list)\n` +
        `       once someone has decided Novis may ship under it.`,
    );
  }
}

function take(node: Spdx): string[] {
  if (node.kind === "id") {
    if (!PREFERENCE.includes(node.ident)) throw new Undeclared(node.ident);
    return [node.ident];
  }
  if (node.kind === "and") {
    const out: string[] = [];
    for (const term of node.terms) for (const ident of take(term)) if (!out.includes(ident)) out.push(ident);
    return out;
  }
  // An OR: Novis takes one branch, so a branch it has no policy for is one it declines — `r-efi` offers
  // `MIT OR Apache-2.0 OR LGPL-2.1-or-later`, and Novis takes MIT. That is cargo-deny's own reading of
  // an OR. An OR with no understood branch still fails: that license is one Novis would ship under.
  const options: string[][] = [];
  const declined: Undeclared[] = [];
  for (const term of node.terms) {
    try {
      options.push(take(term));
    } catch (e) {
      if (!(e instanceof Undeclared)) throw e;
      declined.push(e);
    }
  }
  if (options.length === 0) throw declined[0];
  const rank = (opt: string[]) => Math.min(...opt.map((i) => PREFERENCE.indexOf(i)));
  return options.reduce((best, opt) => (rank(opt) < rank(best) ? opt : best));
}

// ---------------------------------------------------------------------------------------------------
// License texts
// ---------------------------------------------------------------------------------------------------

// The characters Python's `str.isspace` accepts. `normalize` strips exactly these, so a text that
// ends in a form feed or a BOM comes out the same as the notice CI has always checked.
const SPACE = new Set(
  [..."\t\n\x0b\x0c\r\x1c\x1d\x1e\x1f \x85\xa0     　"]
    .map((c) => c.charCodeAt(0))
    .concat(Array.from({ length: 11 }, (_, i) => 0x2000 + i)),
);

function rstrip(s: string): string {
  let end = s.length;
  while (end > 0 && SPACE.has(s.charCodeAt(end - 1))) end -= 1;
  return s.slice(0, end);
}

function strip(s: string): string {
  let start = 0;
  while (start < s.length && SPACE.has(s.charCodeAt(start))) start += 1;
  return rstrip(s.slice(start));
}

function pySplit(s: string): string[] {
  const out: string[] = [];
  let word = "";
  for (const c of s) {
    if (SPACE.has(c.charCodeAt(0))) {
      if (word) out.push(word);
      word = "";
    } else word += c;
  }
  if (word) out.push(word);
  return out;
}

// Windows-1252's row 0x80..0x9F; every other byte is its Latin-1 code point. The five bytes the code
// page leaves undefined decode to U+FFFD.
const CP1252 = [
  0x20ac, 0xfffd, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039, 0x0152,
  0xfffd, 0x017d, 0xfffd, 0xfffd, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x02dc, 0x2122,
  0x0161, 0x203a, 0x0153, 0xfffd, 0x017e, 0x0178,
];

/**
 * A license file's text, never with a corrupted copyright line. Almost every crate ships UTF-8, but a
 * few ship Latin-1, and the byte that differs is the © in the copyright notice — the part attribution
 * exists to reproduce. So a file that is not UTF-8 is read as Windows-1252, a codec that cannot fail.
 */
function readText(path: string): string {
  const raw = readFileSync(path);
  try {
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(raw);
  } catch {
    let out = "";
    for (const b of raw) out += String.fromCharCode(b >= 0x80 && b < 0xa0 ? CP1252[b - 0x80]! : b);
    return out;
  }
}

function classify(text: string): string | null {
  const lowered = text.toLowerCase();
  for (const [ident, needles] of FINGERPRINTS) if (needles.every((n) => lowered.includes(n))) return ident;
  return null;
}

function isFile(path: string): boolean {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

/**
 * One crate directory's license texts by SPDX identifier, and every NOTICE file in it. Apache-2.0
 * § 4(d) makes reproducing a NOTICE mandatory, so unlike a license text it is never deduplicated.
 */
function readLicenseFiles(directory: string): [Map<string, string>, [string, string][]] {
  const texts = new Map<string, string>();
  const notices: [string, string][] = [];
  let entries: string[];
  try {
    entries = readdirSync(directory).sort(cmp);
  } catch (e) {
    throw new Fatal(
      `error: cannot read ${directory}: ${(e as Error).message}\n` +
        `       Run \`cargo fetch\` so every dependency's source is on disk.`,
    );
  }
  for (const entry of entries) {
    const path = join(directory, entry);
    if (!isFile(path)) continue;
    const lowered = entry.toLowerCase();
    if (lowered.startsWith("notice")) {
      notices.push([entry, readText(path)]);
      continue;
    }
    if (!LICENSE_FILE_PREFIXES.some((p) => lowered.startsWith(p))) continue;
    const text = readText(path);
    const ident = classify(text);
    // The first file per identifier wins, and entries are sorted, so a crate shipping one license
    // twice gives the same text every run.
    if (ident !== null && !texts.has(ident)) texts.set(ident, text);
  }
  return [texts, notices];
}

/** Line endings and trailing whitespace only: a CRLF checkout matches an LF one, and no word changes. */
function normalize(text: string): string {
  return strip(text.replaceAll("\r\n", "\n").split("\n").map(rstrip).join("\n"));
}

/** `deny.toml`'s `allow` list, read line by line: no TOML parser in the tool that audits dependencies. */
function denyAllowlist(): Set<string> {
  const allowed = new Set<string>();
  let text: string;
  try {
    text = readFileSync(DENY, "utf-8");
  } catch {
    return allowed;
  }
  let inside = false;
  for (const line of text.split(/\r\n|\r|\n/)) {
    const stripped = strip(line);
    if (stripped.startsWith("allow") && stripped.includes("=") && stripped.includes("[")) {
      inside = true;
      continue;
    }
    if (!inside) continue;
    if (stripped.startsWith("]")) break;
    let value = strip(stripped.split("#")[0]!);
    while (value.endsWith(",")) value = value.slice(0, -1);
    value = strip(value);
    if (value.length >= 2 && value[0] === '"' && value.at(-1) === '"') allowed.add(value.slice(1, -1));
  }
  return allowed;
}

/**
 * Fails when `PREFERENCE` and `deny.toml`'s allow list have drifted apart. The two cannot be merged —
 * `deny.toml` is cargo-deny's format and has no order — so they are checked against each other, which
 * catches a license added to one file and not the other.
 */
function checkPoliciesAgree(allowed: Set<string>): void {
  if (allowed.size === 0) return;
  const unranked = [...allowed].filter((i) => !PREFERENCE.includes(i)).sort(cmp);
  const unallowed = PREFERENCE.filter((i) => !allowed.has(i)).sort(cmp);
  const problems: string[] = [];
  if (unranked.length > 0) problems.push(`allowed by deny.toml but absent from PREFERENCE: ${unranked.join(", ")}`);
  if (unallowed.length > 0) problems.push(`ranked in PREFERENCE but not allowed by deny.toml: ${unallowed.join(", ")}`);
  if (problems.length > 0) {
    throw new Fatal(
      `error: the two license policies disagree:\n${problems.map((p) => `  - ${p}`).join("\n")}\n` +
        `       Both files must name the same identifiers; only the order differs.`,
    );
  }
}

// ---------------------------------------------------------------------------------------------------
// The notice
// ---------------------------------------------------------------------------------------------------

interface Component {
  name: string;
  version: string;
  taken: string;
  offered: string;
}
interface Group {
  license: string;
  text: string;
  components: string[];
}

const line = (char = "-") => char.repeat(WIDTH);
const width = (s: string) => [...s].length;
const pad = (s: string, w: number) => s + " ".repeat(Math.max(0, w - width(s)));

function render(components: Component[], groups: Group[], notices: [string, string, string][]): string {
  const out: string[] = [];
  const add = (s: string) => out.push(s);

  add("Novis — third-party attribution");
  add(line("="));
  add("");
  add("This file is the complete notice for third-party components compiled into");
  add(`the \`${BINARY}\` binary. Novis's own terms are MIT and live in LICENSE; they are`);
  add("not restated here.");
  add("");
  add("GENERATED FILE — do not edit by hand. Regenerate with:");
  add("");
  add("    bun nv gen-attribution");
  add("");
  add("");
  // `nvs info` slices this file at the two section headings below and prints the parts verbatim, so
  // the wording that explains a section is inside it: a shipped binary has no file to talk about.
  // crates/nvs-cli/src/info.rs owns the other half of that agreement.
  add(`COMPONENTS (${components.length})`);
  add(line());
  add("");
  add("Where a component offers a choice of licenses, Novis takes the one shown and");
  add("the full offer follows in parentheses. Components are listed for every");
  add("platform Novis builds for, so one may appear that a given build omits.");
  add("");

  const nameW = Math.max(...components.map((c) => width(c.name)));
  const versionW = Math.max(...components.map((c) => width(c.version)));
  const takenW = Math.max(...components.map((c) => width(c.taken)));
  for (const c of components) {
    const offered = c.offered === c.taken ? "" : `  (${c.offered})`;
    add(rstrip(`  ${pad(c.name, nameW)}  ${pad(c.version, versionW)}  ${pad(c.taken, takenW)}${offered}`));
  }

  add("");
  add("");
  add(`LICENSE TEXTS (${groups.length})`);
  add(line());
  add("");
  add("Each text below is reproduced verbatim as its component ships it. Components");
  add("whose text is byte-identical share one entry; components whose copyright");
  add("lines differ do not, which is why one license identifier can appear more");
  add("than once.");
  add("");

  groups.forEach((group, i) => {
    add("");
    add(line("="));
    add(`[${i + 1}/${groups.length}]  ${group.license}`);
    add(line("="));
    add("");
    add("Applies to:");
    for (const name of group.components) add(`  ${name}`);
    add("");
    add(line());
    add("");
    add(group.text);
    add("");
  });

  if (notices.length > 0) {
    add("");
    add(`NOTICE FILES (${notices.length})`);
    add(line());
    add("");
    add("Reproduced as required by Apache License 2.0 § 4(d).");
    for (const [component, filename, text] of notices) {
      add("");
      add(line("="));
      add(`${component} — ${filename}`);
      add(line("="));
      add("");
      add(text);
      add("");
    }
  }

  return rstrip(out.join("\n")) + "\n";
}

async function build(): Promise<string> {
  const meta = await cargoMetadata();
  const packages = await allShippedPackages(meta);
  const allowed = denyAllowlist();
  checkPoliciesAgree(allowed);

  const components: Component[] = [];
  // Keyed by identifier and text together, so identical texts collapse and a differing copyright line
  // keeps its own entry. A Map keeps first-seen order, which is the tie-break the groups sort on.
  const byText = new Map<string, { ident: string; text: string; names: string[] }>();
  // Crates that ship no license file, resolved in a second pass against a sibling's text.
  const pending: [string, string][] = [];
  const knownText = new Map<string, string>();
  const notices: [string, string, string][] = [];
  const problems: string[] = [];
  const group = (ident: string, text: string) => {
    const key = `${ident}\0${text}`;
    if (!byText.has(key)) byText.set(key, { ident, text, names: [] });
    return byText.get(key)!.names;
  };

  for (const pkg of packages) {
    const label = `${pkg.name} ${pkg.version}`;
    const expr = pkg.license;
    if (!expr) {
      // `license-file` instead of `license`: the crate's terms are not an SPDX expression at all.
      problems.push(`${label}: no \`license\` field in its manifest`);
      continue;
    }
    const taken = choose(parseSpdx(expr), label);
    components.push({ name: pkg.name, version: pkg.version, taken: taken.join(" AND "), offered: expr });

    for (const ident of taken) {
      if (allowed.size > 0 && !allowed.has(ident)) problems.push(`${label}: ${ident} is not in deny.toml's allow list`);
    }

    const [texts, crateNotices] = readLicenseFiles(dirname(pkg.manifest_path));
    for (const [filename, text] of crateNotices) notices.push([label, filename, normalize(text)]);

    for (const ident of taken) {
      const text = texts.get(ident);
      if (text === undefined) {
        pending.push([label, ident]);
        continue;
      }
      const body = normalize(text);
      if (!knownText.has(ident)) knownText.set(ident, body);
      group(ident, body).push(label);
    }
  }

  // A prebuilt C library has no manifest: its record names the licence, and its own files are the text.
  for (const lib of PREBUILT_C) {
    const version = prebuiltVersion(lib);
    components.push({ name: lib.name, version, taken: lib.license, offered: lib.license });
    const text = lib.texts.map((name) => normalize(readText(join(ROOT, lib.dir, name)))).join("\n\n");
    group(lib.license, text).push(`${lib.name} ${version}`);
  }
  components.sort((a, b) => cmp(a.name.toLowerCase(), b.name.toLowerCase()) || cmp(a.version, b.version));

  // A crate with no license file of its own is common where only a repository's root crate carries
  // one. Borrowing the sibling's text is exact for a license with no per-crate copyright line
  // (Apache-2.0), and is why this is an error for anything else.
  for (const [label, ident] of pending) {
    const text = knownText.get(ident);
    if (text === undefined) {
      problems.push(
        `${label}: takes ${ident} but ships no license file, and no other ` +
          `component in the tree ships one for ${ident} either`,
      );
      continue;
    }
    group(ident, text).push(label);
  }

  if (problems.length > 0) {
    throw new Fatal(`error: attribution is incomplete:\n${problems.map((p) => `  - ${p}`).join("\n")}`);
  }

  const lower = (a: string, b: string) => cmp(a.toLowerCase(), b.toLowerCase());
  const groups: Group[] = [...byText.values()].map((g) => ({
    license: g.ident,
    text: g.text,
    components: [...g.names].sort(lower),
  }));
  groups.sort(
    (a, b) => PREFERENCE.indexOf(a.license) - PREFERENCE.indexOf(b.license) || lower(a.components[0]!, b.components[0]!),
  );
  notices.sort((a, b) => cmp(a[0], b[0]) || cmp(a[1], b[1]) || cmp(a[2], b[2]));

  return render(components, groups, notices);
}

async function writeOrCheck(check: boolean): Promise<number> {
  const generated = await build();
  if (check) {
    let current: string;
    try {
      current = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(readFileSync(OUTPUT));
    } catch {
      console.error(`error: ${OUTPUT_NAME} does not exist`);
      return 1;
    }
    if (current.replaceAll("\r\n", "\n").replaceAll("\r", "\n") !== generated) {
      console.error(
        `error: ${OUTPUT_NAME} is out of date with Cargo.lock.\n` +
          `       Run \`bun nv gen-attribution\` and commit the result.`,
      );
      return 1;
    }
    console.log(`${OUTPUT_NAME} is up to date`);
    return 0;
  }
  writeFileSync(OUTPUT, generated, "utf-8");
  const commas = (n: number) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const lines = generated.split("\n").length - 1;
  console.log(`wrote ${OUTPUT_NAME} (${commas(width(generated))} bytes, ${commas(lines)} lines)`);
  return 0;
}

function help(): string {
  return [
    USAGE,
    "",
    "Generate THIRD-PARTY-LICENSES.txt from the resolved dependency graph.",
    "",
    "options:",
    "  -h, --help      show this help message and exit",
    "  --check         exit non-zero if the committed file is out of date, writing",
    "                  nothing",
    "  --check-c-deps  list the default binary's C dependencies; exit non-zero on",
    "                  one `rule:packaging/a-c-dependency-answers-two-questions`",
    "                  has no record for, and the prebuilt C libraries inside",
    "                  the built-in components",
  ].join("\n");
}

export async function run(args: string[]): Promise<number> {
  let flags: Set<string>;
  try {
    ({ flags } = parseArgs(args, { flags: ["--check", "--check-c-deps"], valued: [] }));
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv gen-attribution: error: ${e.message}`);
    return 2;
  }
  if (flags.has("--help")) {
    console.log(help());
    return 0;
  }
  try {
    return flags.has("--check-c-deps") ? await checkCDeps() : await writeOrCheck(flags.has("--check"));
  } catch (e) {
    if (!(e instanceof Fatal)) throw e;
    console.error(e.message);
    return 1;
  }
}
