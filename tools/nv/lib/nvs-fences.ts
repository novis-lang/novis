// The ```nvs code blocks of a page, and `nvs fmt` over them. A page is a `.md` or `.mdx` file under a
// `pages` entry of `tools/data/nvs-fmt-corpus.txt`. `crates/nvs-fmt/tests/common/mod.rs` reads blocks
// the same way, line by line, so the identity test and this agree on what a block is.
//
// A block opens on a line that is its indent, then ```nvs or ```novis, and closes on the first later
// line that is the same indent and ``` alone. A block that has no `<?nvs` or `<?=` tag is code from its first line,
// so it is formatted with a `<?nvs` line put in front and taken off again. A block the formatter refuses
// is a fragment, such as a signature, and is left as it is.

import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "./paths.ts";
import { run } from "./proc.ts";

export interface Block {
  /** 1-based line of the opening fence. */
  line: number;
  /** The first and the last line of the body, 0-based and exclusive at the end. */
  from: number;
  to: number;
  indent: string;
  /** The body without its indent, each line ending in `\n`. */
  body: string;
}

const OPEN = /^([ \t]*)```(nvs|novis)\b.*$/;

/** The ```nvs blocks of `text`, in order. */
export function blocks(text: string): Block[] {
  const lines = text.split("\n").map((l) => l.replace(/\r$/, ""));
  const out: Block[] = [];
  for (let i = 0; i < lines.length; i++) {
    const m = OPEN.exec(lines[i]!);
    if (!m) continue;
    const indent = m[1]!;
    let end = i + 1;
    while (end < lines.length && lines[end] !== `${indent}\`\`\``) end++;
    if (end >= lines.length) break;
    const body = lines
      .slice(i + 1, end)
      .map((l) => (l.startsWith(indent) ? l.slice(indent.length) : l.trimStart()))
      .map((l) => `${l}\n`)
      .join("");
    out.push({ line: i + 1, from: i + 1, to: end, indent, body });
    i = end;
  }
  return out;
}

const HEAD = "<?nvs\n";

/** Whether `body` is a page of its own: it opens code with a tag, so text before the tag is HTML. */
const tagged = (body: string) => /<\?(nvs|=)/.test(body);

/** What `nvs fmt` reads for `body`: the body itself when it has a tag, else the body behind `<?nvs`. */
export function wrapped(body: string): string {
  return tagged(body) ? body : HEAD + body;
}

/** `formatted`, the formatter's output for `wrapped(body)`, back as a block body. */
export function unwrapped(body: string, formatted: string): string | null {
  if (tagged(body)) return formatted;
  return formatted.startsWith(HEAD) ? formatted.slice(HEAD.length) : null;
}

/** `text` with each block's body replaced by `bodies[i]`, at the block's indent and in its line ending. */
export function replaced(text: string, found: Block[], bodies: string[]): string {
  const eol = text.includes("\r\n") ? "\r\n" : "\n";
  const lines = text.split("\n").map((l) => l.replace(/\r$/, ""));
  for (let k = found.length - 1; k >= 0; k--) {
    const b = found[k]!;
    const body = bodies[k]!.replace(/\n$/, "").split("\n").map((l) => (l === "" ? "" : b.indent + l));
    lines.splice(b.from, b.to - b.from, ...body);
  }
  return lines.join(eol);
}

export interface FenceRun {
  /** `file:line` of each block the formatter changes. */
  changed: string[];
  /** The pages written, in write mode. */
  written: string[];
  refused: number;
}

/**
 * Formats every block of `pages` with `nvs` at once: each block becomes a file under a scratch
 * directory, one `nvs fmt` formats them all, and each page whose blocks changed is written back unless
 * `check`. Throws when `nvs fmt` fails in a way a refusal does not explain.
 */
export async function formatBlocks(nvs: string, pages: string[], check: boolean): Promise<FenceRun> {
  const scratch = join(ROOT, ".agent-tmp", `nvs-fmt-blocks-${process.pid}`);
  rmSync(scratch, { recursive: true, force: true });
  mkdirSync(scratch, { recursive: true });
  try {
    const all: { page: string; text: string; found: Block[] }[] = [];
    let n = 0;
    for (const page of pages) {
      const text = readFileSync(join(ROOT, page), "utf8");
      const found = blocks(text);
      if (found.length === 0) continue;
      for (const b of found) writeFileSync(join(scratch, `${n++}.nvs`), wrapped(b.body));
      all.push({ page, text, found });
    }
    if (n === 0) return { changed: [], written: [], refused: 0 };
    const r = await run([nvs, "fmt", scratch], { cwd: ROOT });
    // 2 is a block that does not parse, which is left as it is.
    if (r.code !== 0 && r.code !== 2) throw new Error(`\`nvs fmt\` exited ${r.code}:\n${r.stderr.trimEnd()}`);
    const refused = [...r.stderr.matchAll(/not formatted, because it does not parse/g)].length;
    const changed: string[] = [];
    const written: string[] = [];
    let k = 0;
    for (const { page, text, found } of all) {
      const bodies = found.map((b) => unwrapped(b.body, readFileSync(join(scratch, `${k++}.nvs`), "utf8")) ?? b.body);
      const moved = found.filter((b, i) => bodies[i] !== b.body);
      for (const b of moved) changed.push(`${page}:${b.line}`);
      if (moved.length > 0 && !check) {
        writeFileSync(join(ROOT, page), replaced(text, found, bodies));
        written.push(page);
      }
    }
    return { changed, written, refused };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}
