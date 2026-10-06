// What `nv proofs` reads out of a test file: each `covers:` marker, and each `Class::member(` call a
// case makes. `collect.ts` builds a feature's tests from these.

/** Each tree whose files are scanned for markers, with the file ending scanned there. The example,
 * attack and bench trees are attributed by path, so a marker in one is never read. */
export const MARKER_ROOTS: readonly [string, string][] = [["tests/conformance", ".nvst"], ["crates", ".rs"]];

/** Each tree whose cases are scanned for calls. */
export const CALL_ROOTS: readonly string[] = ["tests/conformance"];

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

// A reader of the scans records what it asked for, not every file the scan read: `covers:#<feature>`
// for the markers naming a feature and `calls:#<call>` for the cases making a call. A change to a
// scanned file moves the keys of what its text names before and after (`markerKeys`), and a change
// whose earlier text is not known moves every key of both kinds (`COVERS_ANY`, `CALLS_ANY`). The
// roster's scans of the stdlib for where a feature is declared are keyed the same way, `anchor:#<id>`
// (`roster.ts` `anchorScan`).

/** Every `covers:` key, as the prefix a selection looks up. */
export const COVERS_ANY = "covers:#";
/** Every `calls:` key, as the prefix a selection looks up. */
export const CALLS_ANY = "calls:#";

/** Every `anchor:` key, as the prefix a selection looks up: where the roster found a feature declared. */
export const ANCHOR_ANY = "anchor:#";

export const coversKey = (feature: string) => `${COVERS_ANY}${feature}`;
export const callsKey = (call: string) => `${CALLS_ANY}${call}`;
export const anchorKey = (feature: string) => `${ANCHOR_ANY}${feature}`;

/** Whether the scans read `path`: for markers, and for calls. */
export function scannedFor(path: string): { markers: boolean; calls: boolean } {
  const under = (base: string) => path.startsWith(`${base}/`) && !path.split("/").includes("target");
  return {
    markers: MARKER_ROOTS.some(([base, ext]) => under(base) && path.endsWith(ext)),
    calls: CALL_ROOTS.some((base) => under(base) && path.endsWith(".nvst")),
  };
}

/** The keys the text of `path` holds for the scans: a `covers:` key for each feature its markers name,
 * and a `calls:` key for each call it makes. */
export function markerKeys(path: string, text: string): string[] {
  const s = scannedFor(path);
  const out = new Set<string>();
  if (s.markers) for (const [name] of markersIn(path, text)) out.add(coversKey(name));
  if (s.calls) for (const call of callsIn(text)) out.add(callsKey(call));
  return [...out];
}
