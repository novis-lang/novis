// One node of `nvs ast --json`, and what the panel shows for it.
//
// The schema is the compiler's, frozen by `rule:ide/ast-json-schema-is-frozen` and emitted by
// `crates/nvs-cli/src/ast.rs`: a node is its `kind`, its `span` as `[start, end]`, its own scalar
// fields as siblings of those, and its `children`. Nothing here re-derives any of it — there is no
// parser in this client (`rule:ide/the-ast-panel-shells-out-to-the-cli`), and a node's kind is a
// string this file never enumerates, so a production added to the grammar renders the day the
// binary emits it.
//
// Nothing here imports `vscode`, for the reason `src/concealment.ts` gives: the headless tier runs
// it in plain Node, and a rendering whose test needs a display is a rendering nobody tests. What is
// left in `src/ast.ts` is the panel — the process, the tree view and the editor — and every
// decision about what a node *reads* as is here.

/**
 * A node of the frozen schema.
 *
 * The index signature is the scalar fields: which ones a production has is the compiler's decision
 * and the panel's business is only to show them, so they are read by difference from the three
 * names above rather than from a list this file would have to be taught.
 */
export interface Node {
  kind: string;
  span: [number, number];
  children: Node[];
  [field: string]: unknown;
}

/**
 * The document `nvs ast --json` printed, or nothing when it is not one.
 *
 * A document that does not parse or does not hold the schema's three names is refused whole rather
 * than rendered as far as it goes: the panel would otherwise draw a partial tree for a binary that
 * is not the one this client understands, and say nothing about it.
 */
export function read(text: string): Node | undefined {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return undefined;
  }
  return shaped(value) ? value : undefined;
}

/** The label a node is shown under: its kind, which is the one name the schema promises. */
export function label(node: Node): string {
  return node.kind;
}

/**
 * The node's own scalar fields, dimmed beside its label.
 *
 * `op=Add`, `value=true` — the two shapes the compiler emits, a fixed spelling out of a closed set
 * and a flag. They are joined rather than listed because a tree item has one line, and a field the
 * panel has never heard of is shown by the same rule as one it has.
 */
export function detail(node: Node): string {
  return fields(node)
    .map(([name, value]) => `${name}=${String(value)}`)
    .join("  ");
}

/** What hovering a node says: its kind and the bytes it covers. */
export function tooltip(node: Node): string {
  return `${node.kind}  bytes ${node.span[0]}..${node.span[1]}`;
}

/**
 * The UTF-16 index `offset` lands on in `text`.
 *
 * A span is a byte offset into the file's UTF-8 bytes, which is what the compiler counts in, and
 * every position API in the editor counts UTF-16 code units. The two agree on ASCII and part
 * company at the first accented letter in a comment, so a panel selecting `document.positionAt`
 * of a raw span would highlight the wrong text in exactly the files a developer opens it for.
 *
 * The equality is the ASCII case, which is most files: when every byte is one code unit the offset
 * is already the answer and nothing is copied.
 */
export function index(text: string, offset: number): number {
  if (Buffer.byteLength(text, "utf8") === text.length) {
    return offset;
  }
  return Buffer.from(text, "utf8").subarray(0, offset).toString("utf8").length;
}

/** The names the schema reserves, which is everything that is not a scalar field. */
const RESERVED = ["kind", "span", "children"];

/** A node's scalar fields, in the order the document carried them. */
function fields(node: Node): [string, unknown][] {
  return Object.entries(node).filter(([name]) => !RESERVED.includes(name));
}

/** Whether `value` is a node, and whether every node under it is one. */
function shaped(value: unknown): value is Node {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const node = value as Partial<Node>;
  return (
    typeof node.kind === "string" &&
    Array.isArray(node.span) &&
    node.span.length === 2 &&
    node.span.every((at) => typeof at === "number") &&
    Array.isArray(node.children) &&
    node.children.every(shaped)
  );
}
