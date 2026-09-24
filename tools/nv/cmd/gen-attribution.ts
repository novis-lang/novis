// `bun nv gen-attribution --check-c-deps`: the default binary's C dependencies, held against the ledger
// `rule:packaging/a-c-dependency-answers-two-questions` asks for.
//
//     bun nv gen-attribution --check-c-deps    list them; exit 1 on one the ledger does not record
//
// The enumeration comes from `cargo metadata` over the package set the third-party notice is built
// from: every package reachable from `nvs-cli` through a normal or build dependency, host-independent,
// so a Windows and a Linux checkout agree. A package counts when it declares `links`, or when it
// build-depends on a tool that compiles C (`C_BUILD_TOOLS`). Either signal can fire on a crate that
// builds nothing native, and the ledger entry is what answers it. The check fails in both directions:
// on a C dependency with no entry, and on an entry naming a crate the graph no longer signals.
//
// Writing and checking `THIRD-PARTY-LICENSES.txt` itself is still `python tools/gen-attribution.py`
// and `--check`; this command refuses those two modes until they are ported.

import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs, pyRepr } from "../lib/py.ts";

export const summary = "the default binary's C dependencies against their ledger: nv gen-attribution --check-c-deps";

const USAGE = "usage: nv gen-attribution [-h] [--check] [--check-c-deps]";

const ROOT_PACKAGE = "nvs-cli";
const BINARY = "nvs";

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
};

interface Package {
  id: string;
  name: string;
  version: string;
  source: string | null;
  links?: string | null;
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
  resolve: { nodes: Node[] };
}

class Fatal extends Error {}

async function cargoMetadata(): Promise<Metadata> {
  const r = await runProc(["cargo", "metadata", "--format-version", "1", "--locked"], { cwd: ROOT });
  if (r.code !== 0) throw new Fatal(`error: cargo metadata failed:\n${r.stderr.trim()}`);
  return JSON.parse(r.stdout) as Metadata;
}

/** Every third-party package reachable from the binary through a normal or build dependency. */
function shippedPackages(meta: Metadata): Package[] {
  const byId = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const stack = meta.packages.filter((p) => p.name === ROOT_PACKAGE).map((p) => p.id);
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
  // A package with no `source` is one of the workspace's own crates.
  return [...seen].map((id) => byId.get(id)!).filter((p) => p.source !== null && p.source !== undefined);
}

const cmp = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

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
  if (problems.length === 0) return 0;
  console.error("error: the C-dependency ledger is out of date:");
  console.error(problems.join("\n"));
  return 1;
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
    "                  has no record for",
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
  if (!flags.has("--check-c-deps")) {
    console.error("nv gen-attribution: writing and checking THIRD-PARTY-LICENSES.txt is not ported yet;");
    console.error("                    `python tools/gen-attribution.py [--check]` still does it.");
    return 2;
  }
  try {
    return await checkCDeps();
  } catch (e) {
    if (!(e instanceof Fatal)) throw e;
    console.error(e.message);
    return 1;
  }
}
