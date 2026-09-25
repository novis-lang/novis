// What `nv proofs` reads out of a test file: each `covers:` marker, and each `Class::member(` call a
// case makes. `collect.ts` builds a feature's tests from these, and `keys/checks.ts` keys a proofs
// group on them, so one scanner answers both and a key never sees less than the proofs tool reads.

/** Each tree whose files are scanned for markers, with the file ending scanned there. The example,
 * attack and bench trees are attributed by path, so a marker in one is never read. */
export const MARKER_ROOTS: readonly [string, string][] = [["tests/conformance", ".nvst"], ["tests/differential", ".nvst"], ["crates", ".rs"]];

/** Each tree whose cases are scanned for calls. */
export const CALL_ROOTS: readonly string[] = ["tests/conformance", "tests/differential"];

/** `// covers: A, B`, in a `.nvst`, a `.nvs` or above a Rust `#[test]`. `#` lets it sit in TOML too. */
const COVERS_RE = /(?:\/\/|#)\s*covers:\s*(.+)/g;

/** A case's `Class::member(` call, with or without the `Core\` prefix. */
const CALL_RE = /(?:Core\\)?([A-Za-z_][A-Za-z0-9_\\]*)::([a-zA-Z_][a-zA-Z0-9_]*)\s*\(/g;

const names = (text: string) => text.split(",").map((p) => p.trim().replace(/^[`"']+|[`"']+$/g, "")).filter(Boolean);

/** Each feature id a `covers:` marker in `text` names, with the label its test is credited under: the
 * path, and for a Rust file the `#[test] fn` under the marker. */
export function markersIn(path: string, text: string): [string, string][] {
  if (!text.includes("covers:")) return [];
  const out: [string, string][] = [];
  for (const m of text.matchAll(COVERS_RE)) {
    let label = path;
    if (path.endsWith(".rs")) {
      const end = m.index! + m[0].length;
      const fn = /\bfn\s+([a-zA-Z_][a-zA-Z0-9_]*)/.exec(text.slice(end, end + 400));
      if (fn) label = `${label}::${fn[1]}`;
    }
    for (const name of names(m[1]!)) out.push([name, label]);
  }
  return out;
}

/** Each member a case calls as `Class::member(`, keyed `static:<class tail>::<member>`. Only that
 * written form is credited: `->member(` cannot be tied to a class without a type checker. */
export function callsIn(text: string): string[] {
  return [...text.matchAll(CALL_RE)].map((m) => `static:${m[1]!.split("\\").pop()}::${m[2]}`);
}
