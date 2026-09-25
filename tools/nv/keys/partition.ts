// The partitions of the tree: who reads a path decides which one it is in. A check that cannot be
// keyed more narrowly keys on the partitions it reads, and `nv why` groups a key's inputs by them.
//
// `crates` is what the binary is built from, and a crate's `tests/` and `benches/` directories are
// `crate-tests`, because cargo runs them and the binary is not built from them. `docs` is
// `docs/reference/` and `docs/spec/`, which the binary embeds and some crate tests read; `goals` is
// `docs/agent/goals/`, which one crate test walks; `prose` is the rest of `docs/`, which nothing
// under `crates/` opens. The case trees under `tests/` are each their own partition, `tests` is what
// is left of it, and `bench-members` is `benches/members/`, which only the bench tools read. The
// files a wrap rewrites every session are `state`. Any other top-level entry is `other`.

export const PARTITIONS: Record<string, readonly string[]> = {
  crates: ["crates", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml", "deny.toml", "nvs.toml", "LICENSE", "THIRD-PARTY-LICENSES.txt"],
  "crate-tests": ["benches"],
  examples: ["examples"],
  tests: ["tests"],
  conformance: [],
  differential: [],
  hostile: [],
  "lsp-cases": [],
  "bench-members": [],
  docs: [],
  goals: [],
  prose: ["docs"],
  tools: ["tools"],
  editors: ["editors"],
};

// The splits below the top level, first match wins. `vectors/` is embedded, so it is `crates` even
// though it sits under a crate's `tests/`.
const SPLITS: [string, string][] = [
  ["crates/nvs-stdlib/tests/vectors/", "crates"],
  ["tools/data/php-builtins.txt", "crates"],
  ["docs/reference/", "docs"],
  ["docs/spec/", "docs"],
  ["docs/agent/goals/", "goals"],
  ["tests/conformance/", "conformance"],
  ["tests/differential/", "differential"],
  ["tests/hostile/", "hostile"],
  ["tests/lsp/", "lsp-cases"],
  ["benches/members/lang/types/numbers-bool-int-uint-float-decimal.nvs", "crate-tests"],
  ["benches/members/", "bench-members"],
];

export const OTHER = "other";
export const STATE = "state";
/** A goal's handoff record, main's or a side goal's: what a wrap rewrites every session. */
const STATE_FILES = /^data\/goals\/(side\/)?[^/]+\.handoff\.json$/;

const TOP_OWNER = new Map<string, string>();
for (const [name, tops] of Object.entries(PARTITIONS)) for (const top of tops) TOP_OWNER.set(top, name);

/** The partition a repo-relative path belongs to. */
export function partitionOf(rel: string): string {
  if (STATE_FILES.test(rel)) return STATE;
  for (const [prefix, name] of SPLITS) if (rel.startsWith(prefix)) return name;
  const parts = rel.split("/");
  if (parts[0] === "crates" && parts.length > 3 && (parts[2] === "tests" || parts[2] === "benches")) return "crate-tests";
  return TOP_OWNER.get(parts[0]!) ?? OTHER;
}
